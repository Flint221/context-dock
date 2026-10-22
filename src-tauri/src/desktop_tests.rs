//! Desktop tests never use live credentials, project sources or the app database.
use crate::{
    cloud, desktop,
    path_selection::{self, Rule},
    registry::Registry,
    review::{self, ReviewRequest},
};
use std::{collections::HashSet, fs};
use tempfile::{tempdir, TempDir};
struct Fixture {
    _temp: TempDir,
    root: std::path::PathBuf,
    r: Registry,
    id: String,
}
impl Fixture {
    fn new() -> Self {
        let t = tempdir().unwrap();
        let root = t.path().join("project");
        fs::create_dir(&root).unwrap();
        fs::create_dir(t.path().join("app")).unwrap();
        fs::write(root.join("Existing_v1.md"), "original").unwrap();
        let mut r = Registry::open(&t.path().join("app/test.db")).unwrap();
        let id = r.add("Disposable", root.to_str().unwrap()).unwrap();
        r.configure_tracking(&id, true, false, r.source_ids(&id).unwrap())
            .unwrap();
        Self {
            _temp: t,
            root,
            r,
            id,
        }
    }
    fn add(&mut self, path: &str) {
        if let Some(parent) = self.root.join(path).parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(self.root.join(path), "new document").unwrap();
        self.r
            .reconcile(&self.id, "test", &HashSet::new(), true)
            .unwrap();
    }
    fn request(&self, path: &str, action: &str) -> ReviewRequest {
        ReviewRequest {
            project: self.id.clone(),
            paths: vec![path.into()],
            action: action.into(),
        }
    }
}
#[test]
fn approval_preview_does_not_capture_or_upload_and_apply_only_admits_selected_paths() {
    let mut f = Fixture::new();
    f.add("New.md");
    f.add("Other.docx");
    let before =
        f.r.versions(&f.r.snapshot().unwrap().documents[0].document_id)
            .unwrap()
            .len();
    let req = f.request("New.md", "approve");
    let p = review::preview(&f.r, &req).unwrap();
    assert!(!path_selection::allowed(&f.r.db, &f.id, "New.md").unwrap());
    assert_eq!(review::pending(&f.r.db).unwrap().len(), 2);
    let settings = cloud::project(&f.r.db, &f.id).unwrap();
    review::apply(&mut f.r, &req, p["token"].as_str().unwrap()).unwrap();
    assert!(path_selection::allowed(&f.r.db, &f.id, "New.md").unwrap());
    assert!(!path_selection::allowed(&f.r.db, &f.id, "Other.docx").unwrap());
    assert_eq!(review::pending(&f.r.db).unwrap().len(), 1);
    let after = cloud::project(&f.r.db, &f.id).unwrap();
    for key in ["enabled", "paused", "auto_sync", "started"] {
        assert_eq!(after[key], settings[key]);
    }
    assert_eq!(
        cloud::rows(&f.r.db, "SELECT * FROM cloud_jobs", &[])
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        fs::read_to_string(f.root.join("Existing_v1.md")).unwrap(),
        "original"
    );
    assert_eq!(before, 1);
    assert!(review::apply(&mut f.r, &req, p["token"].as_str().unwrap()).is_err());
}
#[test]
fn ignored_files_and_desktop_choices_survive_restart_without_losing_history() {
    let mut f = Fixture::new();
    f.add("Unwanted.md");
    let req = f.request("Unwanted.md", "ignore");
    let p = review::preview(&f.r, &req).unwrap();
    review::apply(&mut f.r, &req, p["token"].as_str().unwrap()).unwrap();
    review::set_preference(&f.r.db, "desktop.uploads_paused", true).unwrap();
    review::set_preference(&f.r.db, "desktop.keep_running", false).unwrap();
    f.r = Registry::open(&f.r.store.join("test.db")).unwrap();
    assert!(review::pending(&f.r.db).unwrap().is_empty());
    assert!(!path_selection::allowed(&f.r.db, &f.id, "Unwanted.md").unwrap());
    assert!(desktop::uploads_paused(&f.r.db));
    assert!(!review::preference(&f.r.db, "desktop.keep_running", true));
    assert_eq!(
        fs::read_to_string(f.root.join("Unwanted.md")).unwrap(),
        "new document"
    );
    assert_eq!(
        f.r.db
            .query_row("SELECT count(*) FROM document_versions", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
}
#[test]
fn review_refuses_changed_files_stale_rules_excluded_folders_and_removed_projects() {
    let mut f = Fixture::new();
    f.add("Secret/Notes.md");
    let req = f.request("Secret/Notes.md", "approve");
    let p = review::preview(&f.r, &req).unwrap();
    fs::write(f.root.join("Secret/Notes.md"), "changed after review").unwrap();
    f.r.refresh(&f.id).unwrap();
    assert!(review::apply(&mut f.r, &req, p["token"].as_str().unwrap()).is_err());
    let p = review::preview(&f.r, &req).unwrap();
    let revision = path_selection::revision(&f.r.db, &f.id).unwrap();
    let mut rules = path_selection::rules(&f.r.db, &f.id).unwrap();
    rules.push(Rule {
        path: "secret".into(),
        kind: "folder".into(),
    });
    f.r.save_path_selection(&f.id, revision, rules).unwrap();
    assert!(review::pending(&f.r.db).unwrap().is_empty());
    assert!(review::apply(&mut f.r, &req, p["token"].as_str().unwrap()).is_err());
    f.r.remove(&f.id).unwrap();
    assert!(review::preview(&f.r, &req).is_err());
}
#[test]
fn untracked_approval_does_not_enable_tracking_and_missing_sources_are_not_approved() {
    let mut f = Fixture::new();
    f.add("New.md");
    f.r.configure_tracking(&f.id, false, false, vec![]).unwrap();
    let req = f.request("New.md", "approve");
    let p = review::preview(&f.r, &req).unwrap();
    review::apply(&mut f.r, &req, p["token"].as_str().unwrap()).unwrap();
    assert!(!f.r.snapshot().unwrap().projects[0].tracking_enabled);
    f.add("Missing.md");
    fs::remove_file(f.root.join("Missing.md")).unwrap();
    f.r.refresh(&f.id).unwrap();
    assert!(review::preview(&f.r, &f.request("Missing.md", "approve"))
        .unwrap_err()
        .contains("Restore"));
}
#[test]
fn new_approved_family_versions_are_not_pending_but_unrelated_documents_are() {
    let mut f = Fixture::new();
    f.add("Existing_v2.md");
    assert!(review::pending(&f.r.db).unwrap().is_empty());
    f.add("Unrelated.md");
    assert_eq!(review::pending(&f.r.db).unwrap()[0].path, "Unrelated.md");
}
#[test]
fn notification_batching_watermarks_survive_restart_and_throttle_new_bursts() {
    let mut f = Fixture::new();
    let key = "approval:test";
    assert!(!review::notice_due(&f.r.db, key, "one", 1000, 30).unwrap());
    assert!(!review::notice_due(&f.r.db, key, "one,two", 1020, 30).unwrap());
    assert!(review::notice_due(&f.r.db, key, "one,two", 1030, 30).unwrap());
    review::notice_sent(&f.r.db, key, 1030).unwrap();
    f.r = Registry::open(&f.r.store.join("test.db")).unwrap();
    assert!(!review::notice_due(&f.r.db, key, "one,two", 1400, 30).unwrap());
    assert!(!review::notice_due(&f.r.db, key, "three", 1040, 30).unwrap());
    assert!(!review::notice_due(&f.r.db, key, "three", 1100, 30).unwrap());
    assert!(review::notice_due(&f.r.db, key, "three", 1330, 30).unwrap());
}
#[test]
fn schema_eleven_upgrade_preserves_registry_snapshots_and_pending_files() {
    let mut f = Fixture::new();
    f.add("New.md");
    let before = cloud::rows(&f.r.db, "SELECT * FROM document_versions", &[]).unwrap();
    let pending = review::pending(&f.r.db).unwrap().len();
    f.r.db
        .execute_batch(
            "DROP TABLE review_decisions; DROP TABLE desktop_notices; PRAGMA user_version=11;",
        )
        .unwrap();
    f.r = Registry::open(&f.r.store.join("test.db")).unwrap();
    assert_eq!(
        before,
        cloud::rows(&f.r.db, "SELECT * FROM document_versions", &[]).unwrap()
    );
    assert_eq!(review::pending(&f.r.db).unwrap().len(), pending);
    assert!(!desktop::uploads_paused(&f.r.db));
    assert!(review::preference(&f.r.db, "desktop.keep_running", true));
}
#[test]
fn notification_routes_allow_only_internal_pages_and_uuid_projects() {
    let id = "8ac018a7-3d59-4042-a228-db5785141ae1";
    assert!(desktop::parse_route(&format!("contextdock://review?project={id}")).is_some());
    assert!(desktop::parse_route("contextdock://review").is_some());
    for url in [
        "https://example.com",
        "contextdock://project",
        "contextdock://review?project=../../secret",
        "contextdock://review?project=8ac018a7-3d59-4042-a228-db5785141ae1&execute=delete",
        "contextdock://delete?project=x",
    ] {
        assert!(desktop::parse_route(url).is_none(), "{url}");
    }
}
#[test]
#[cfg(windows)]
#[ignore = "Explicit Windows startup integration test; uses only a disposable registration"]
fn windows_startup_registration_round_trip_respects_os_override() {
    use windows_registry::CURRENT_USER;
    let name = format!("ContextDock_Verification_{}", uuid::Uuid::new_v4());
    let command = "\"C:\\ContextDock Verification\\contextdock.exe\" --background";
    struct Cleanup(String);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = crate::startup::set_for(&self.0, "", false);
            if let Ok(k) = CURRENT_USER
                .options()
                .write()
                .open(crate::startup::APPROVED)
            {
                let _ = k.remove_value(&self.0);
            }
        }
    }
    let _cleanup = Cleanup(name.clone());
    assert!(
        !crate::startup::status_for(&name, command)
            .unwrap()
            .registered
    );
    crate::startup::set_for(&name, command, true).unwrap();
    let status = crate::startup::status_for(&name, command).unwrap();
    assert!(status.registered);
    assert_eq!(status.enabled, Some(true));
    let approval = CURRENT_USER.create(crate::startup::APPROVED).unwrap();
    let disabled = [3, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0];
    approval
        .set_bytes(&name, windows_registry::Type::Bytes, &disabled)
        .unwrap();
    assert_eq!(
        crate::startup::status_for(&name, command).unwrap().enabled,
        Some(false)
    );
    crate::startup::set_for(&name, command, true).unwrap();
    assert_eq!(&*approval.get_value(&name).unwrap(), &disabled);
    crate::startup::set_for(&name, command, false).unwrap();
    assert!(
        !crate::startup::status_for(&name, command)
            .unwrap()
            .registered
    );
}
#[test]
#[cfg(windows)]
#[ignore = "Explicit native tray smoke test; opens only a blank fixture window"]
fn windows_tray_hidden_tracking_reopen_and_guarded_exit() {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    };
    use tauri::{Listener, Manager};
    let f = Fixture::new();
    let root = f.root.clone();
    let db = f.r.store.join("test.db");
    let shared = Arc::new(Mutex::new(f.r));
    let for_setup = shared.clone();
    let mut context = tauri::generate_context!();
    context.set_default_window_icon(Some(tauri::image::Image::new_owned(
        vec![90, 130, 220, 255],
        1,
        1,
    )));
    context.config_mut().identifier = "com.contextdock.verification".into();
    context.config_mut().app.windows[0].title = "ContextDock verification (disposable)".into();
    context.config_mut().app.windows[0].url =
        tauri::WebviewUrl::External("about:blank".parse().unwrap());
    let app = tauri::Builder::default()
        .any_thread()
        .on_window_event(|w, e| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = e {
                api.prevent_close();
                desktop::request_close(w.app_handle(), "close");
            }
        })
        .setup(move |app| {
            app.manage(crate::Store(for_setup.clone()));
            app.manage(crate::cloud_commands::CloudRuntime::start(
                db.clone(),
                app.handle().clone(),
            )?);
            app.manage(crate::monitor::Monitor::start(for_setup.clone(), |_| {})?);
            app.manage(desktop::start(app.handle(), db.clone())?);
            desktop::tray(app.handle())?;
            Ok(())
        })
        .build(context)
        .unwrap();
    let handle = app.handle().clone();
    let controller = std::thread::spawn(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            for _ in 0..100 {
                if handle.get_webview_window("main").is_some() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            let w = handle
                .get_webview_window("main")
                .expect("Fixture window was not created");
            assert!(!w.is_visible().unwrap());
            desktop::launch(&handle, &["verification".into(), "--background".into()]);
            assert!(!w.is_visible().unwrap());
            desktop::launch(&handle, &["verification".into()]);
            assert!(w.is_visible().unwrap());
            desktop::desktop_ready(handle.clone()).unwrap();
            let closes = Arc::new(AtomicUsize::new(0));
            let counter = closes.clone();
            let listener = handle.listen("desktop-close", move |_| {
                counter.fetch_add(1, Ordering::Relaxed);
            });
            w.close().unwrap();
            for _ in 0..100 {
                if closes.load(Ordering::Relaxed) > 0 {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            assert_eq!(closes.load(Ordering::Relaxed), 1);
            assert!(w.is_visible().unwrap());
            handle.unlisten(listener);
            tauri::async_runtime::block_on(desktop::desktop_complete(
                handle.state::<crate::Store>(),
                handle.clone(),
                "close".into(),
            ))
            .unwrap();
            assert!(!w.is_visible().unwrap());
            fs::write(root.join("Existing_v1.md"), "edited while hidden").unwrap();
            fs::write(root.join("Brand_New.md"), "requires review").unwrap();
            let mut observed = false;
            for _ in 0..100 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                let r = shared.lock().unwrap();
                let count: i64 =
                    r.db.query_row("SELECT count(*) FROM document_versions", [], |r| r.get(0))
                        .unwrap();
                if count >= 2
                    && review::pending(&r.db)
                        .unwrap()
                        .iter()
                        .any(|f| f.path == "Brand_New.md")
                {
                    observed = true;
                    break;
                }
            }
            assert!(
                observed,
                "hidden native workers did not reconcile fixture files"
            );
            let id = shared.lock().unwrap().snapshot().unwrap().projects[0]
                .id
                .clone();
            desktop::navigate(&handle, serde_json::json!({"page":"review","project":id}));
            assert!(w.is_visible().unwrap());
            assert_eq!(
                desktop::desktop_route(handle.clone()).unwrap()["page"],
                "review"
            );
            tauri::async_runtime::block_on(desktop::desktop_complete(
                handle.state::<crate::Store>(),
                handle.clone(),
                "exit".into(),
            ))
            .unwrap();
        }));
        if outcome.is_err() {
            handle.exit(1);
        }
        outcome
    });
    let code = app.run_return(|_, _| {});
    assert_eq!(code, 0);
    controller.join().unwrap().unwrap();
}
