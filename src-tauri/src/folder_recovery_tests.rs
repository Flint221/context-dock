use crate::{
    cloud::{self, s},
    cloud_models::CloudDraft,
    folder_recovery,
    registry::Registry,
};
use serde_json::json;
use std::{collections::HashSet, fs, path::PathBuf};

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    db: PathBuf,
    r: Registry,
    id: String,
}
impl Fixture {
    fn new(tracking: bool) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("Project Ω");
        let app = temp.path().join("app");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&app).unwrap();
        fs::create_dir(root.join("docs")).unwrap();
        fs::write(root.join("docs/Notes.md"), "Original notes").unwrap();
        fs::write(root.join("Other.md"), "Other content").unwrap();
        let db = app.join("test.db");
        let mut r = Registry::open(&db).unwrap();
        let id = r.add("My project", root.to_str().unwrap()).unwrap();
        if tracking {
            let ids = r.source_ids(&id).unwrap();
            r.configure_tracking(&id, true, false, ids).unwrap();
        }
        Self {
            _temp: temp,
            root,
            db,
            r,
            id,
        }
    }
    fn moved(&self) -> PathBuf {
        let parent = self.root.parent().unwrap().join("New parent");
        fs::create_dir(&parent).unwrap();
        let target = parent.join("Moved project Ω");
        fs::rename(&self.root, &target).unwrap();
        target
    }
    fn reassign(&mut self, target: &std::path::Path) {
        let plan = self
            .r
            .folder_plan(&self.id, target.to_str().unwrap())
            .unwrap();
        self.r
            .reassign_folder(&self.id, &plan.path, &plan.token)
            .unwrap();
    }
}

#[test]
fn loss_persists_across_restart_but_the_original_returning_root_reconnects() {
    let mut f = Fixture::new(false);
    let target = f.moved();
    assert_eq!(f.r.check_folders().unwrap(), vec![f.id.clone()]);
    assert_eq!(
        f.r.snapshot().unwrap().projects[0].folder_state,
        "needs_reassignment"
    );
    f.r = Registry::open(&f.db).unwrap();
    fs::rename(&target, &f.root).unwrap();
    // Startup workspace read checks lost roots as well as ready ones.
    assert_eq!(f.r.snapshot().unwrap().projects[0].folder_state, "ready");
    f.r.refresh(&f.id).unwrap();
    folder_recovery::require_ready(&f.r.db, &f.id).unwrap();
    assert_eq!(f.r.source_ids(&f.id).unwrap().len(), 2);
    assert_eq!(f.r.snapshot().unwrap().projects[0].folder_state, "ready");
    assert_eq!(f.r.snapshot().unwrap().projects[0].id, f.id);
}

#[test]
fn move_keeps_history_cloud_chats_and_preferences_with_missing_files_reported() {
    let mut f = Fixture::new(true);
    f.r.theme("dark").unwrap();
    let before = f.r.snapshot().unwrap();
    let did = before.documents[0].document_id.clone();
    let versions = f.r.versions(&did).unwrap();
    let snapshot = versions[0].snapshot_path.clone();
    let chat = crate::chat::create(&f.r, &f.id).unwrap();
    f.r.db
        .execute(
            "UPDATE cloud_projects SET store_id='vs_preserved' WHERE project_id=?1",
            [&f.id],
        )
        .unwrap();
    let target = f.moved();
    fs::remove_file(target.join("Other.md")).unwrap();
    let plan = f.r.folder_plan(&f.id, target.to_str().unwrap()).unwrap();
    assert_eq!(plan.found, 1);
    assert_eq!(plan.unchanged, 1);
    assert_eq!(plan.missing, vec!["Other.md"]);
    f.r.reassign_folder(&f.id, &plan.path, &plan.token).unwrap();
    let after = f.r.snapshot().unwrap();
    assert_eq!(after.projects[0].id, f.id);
    assert!(after.projects[0].tracking_enabled);
    assert_eq!(after.theme, "dark");
    assert_eq!(after.documents.len(), 2);
    assert_eq!(
        s(&cloud::project(&f.r.db, &f.id).unwrap(), "store_id"),
        "vs_preserved"
    );
    assert_eq!(
        crate::chat::get(&f.r, &f.id, s(&chat, "id")).unwrap()["id"],
        chat["id"]
    );
    assert_eq!(f.r.versions(&did).unwrap()[0].snapshot_path, snapshot);
    assert!(std::path::Path::new(&snapshot).is_file());
    assert!(after
        .documents
        .iter()
        .any(|d| d.name == "Other.md" && d.status == "missing"));
    assert_eq!(
        fs::read_to_string(target.join("docs/Notes.md")).unwrap(),
        "Original notes"
    );
    f.r.reconcile(&f.id, "manual_refresh", &HashSet::new(), true)
        .unwrap();
    assert_eq!(f.r.versions(&did).unwrap().len(), versions.len());
}

#[test]
fn review_rejects_changed_files_stale_project_and_unverified_writes_atomically() {
    let mut f = Fixture::new(true);
    let target = f.moved();
    let plan = f.r.folder_plan(&f.id, target.to_str().unwrap()).unwrap();
    fs::write(target.join("docs/Notes.md"), "Edited after review").unwrap();
    assert!(f
        .r
        .reassign_folder(&f.id, &plan.path, &plan.token)
        .unwrap_err()
        .contains("changed since review"));
    assert_eq!(f.r.path(&f.id).unwrap(), plan.previous_path);
    let fresh = f.r.folder_plan(&f.id, target.to_str().unwrap()).unwrap();
    assert_eq!(fresh.changed, 1);
    f.r.db.execute_batch("CREATE TRIGGER refuse_reassignment BEFORE UPDATE OF path ON projects BEGIN SELECT RAISE(IGNORE); END;").unwrap();
    assert!(f
        .r
        .reassign_folder(&f.id, &fresh.path, &fresh.token)
        .unwrap_err()
        .contains("could not be verified"));
    assert_eq!(f.r.path(&f.id).unwrap(), plan.previous_path);
    f.r.db
        .execute_batch("DROP TRIGGER refuse_reassignment;")
        .unwrap();
    f.reassign(&target);
    assert!(f
        .r
        .reassign_folder(&f.id, &fresh.path, &fresh.token)
        .is_err());
}

#[test]
fn different_directory_at_old_path_does_not_silently_replace_the_project() {
    let f = Fixture::new(false);
    let _target = f.moved();
    fs::create_dir(&f.root).unwrap();
    fs::write(f.root.join("unrelated.md"), "Wrong project").unwrap();
    let r = Registry::open(&f.db).unwrap();
    assert_eq!(
        r.snapshot().unwrap().projects[0].folder_state,
        "needs_reassignment"
    );
    assert!(r
        .snapshot()
        .unwrap()
        .documents
        .iter()
        .all(|d| d.name != "unrelated.md"));
}

#[test]
fn invalid_removed_overlapping_and_app_storage_destinations_leave_registration_unchanged() {
    let mut f = Fixture::new(false);
    let original = f.r.path(&f.id).unwrap();
    assert!(f.r.folder_plan(&f.id, "relative").is_err());
    assert!(f
        .r
        .folder_plan(&f.id, f.root.join("missing").to_str().unwrap())
        .is_err());
    assert!(f
        .r
        .folder_plan(&f.id, f.root.join("Other.md").to_str().unwrap())
        .is_err());
    assert!(f.r.folder_plan(&f.id, f.r.store.to_str().unwrap()).is_err());
    let other = f.root.parent().unwrap().join("Other project");
    fs::create_dir(&other).unwrap();
    let other_id = f.r.add("Other", other.to_str().unwrap()).unwrap();
    assert!(f.r.folder_plan(&f.id, other.to_str().unwrap()).is_err());
    f.r.remove(&other_id).unwrap();
    assert!(f.r.folder_plan(&f.id, other.to_str().unwrap()).is_err());
    fs::create_dir(other.join("child")).unwrap();
    assert!(f
        .r
        .folder_plan(&f.id, other.join("child").to_str().unwrap())
        .is_err());
    assert_eq!(f.r.path(&f.id).unwrap(), original);
}

#[test]
fn lost_root_blocks_capture_chat_cloud_and_recovery_without_losing_saved_data() {
    let mut f = Fixture::new(true);
    let _target = f.moved();
    assert!(crate::chat::create(&f.r, &f.id).is_err());
    assert!(f.r.configure_tracking(&f.id, true, true, vec![]).is_err());
    assert!(f.r.recover_operations(&f.id, true).is_err());
    assert!(crate::chat::authorize(&f.r.db, &f.id, "synthetic-scope").is_err());
    let d = CloudDraft {
        enabled: true,
        automatic: true,
        mode: "current".into(),
        documents: vec![],
        soft_limit: 1024,
    };
    assert!(cloud::desired(&f.r.db, &f.id, &d).is_err());
    assert!(
        !crate::cloud_commands::needed(&f.r.db, &json!({"enabled":1,"project_id":f.id})).unwrap()
    );
    f.r.reconcile(&f.id, "manual_refresh", &HashSet::new(), true)
        .unwrap();
    assert_eq!(f.r.tracking_view(&f.id).unwrap().families.len(), 2);
    assert_eq!(
        f.r.db
            .query_row("SELECT COUNT(*) FROM document_versions", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        2
    );
}

#[test]
fn schema_five_migrates_in_place_and_all_missing_files_can_be_reviewed() {
    let mut f = Fixture::new(true);
    f.r.db.execute_batch("ALTER TABLE projects DROP COLUMN folder_state; ALTER TABLE projects DROP COLUMN folder_identity; ALTER TABLE projects DROP COLUMN folder_revision; PRAGMA user_version=5;").unwrap();
    f.r = Registry::open(&f.db).unwrap();
    assert_eq!(f.r.snapshot().unwrap().projects[0].id, f.id);
    let empty = f.root.parent().unwrap().join("Empty replacement");
    fs::create_dir(&empty).unwrap();
    let plan = f.r.folder_plan(&f.id, empty.to_str().unwrap()).unwrap();
    assert_eq!(plan.found, 0);
    assert_eq!(plan.missing.len(), 2);
    f.r.reassign_folder(&f.id, &plan.path, &plan.token).unwrap();
    assert!(f
        .r
        .snapshot()
        .unwrap()
        .documents
        .iter()
        .all(|d| d.status == "missing"));
}

#[cfg(windows)]
#[test]
fn junction_reassignment_is_refused() {
    use std::os::windows::process::CommandExt;
    let f = Fixture::new(false);
    let link = f.root.parent().unwrap().join("junction");
    let output = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&link)
        .arg(&f.root)
        .creation_flags(0x08000000)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(f.r.folder_plan(&f.id, link.to_str().unwrap()).is_err());
    fs::remove_dir(link).unwrap();
}

#[cfg(windows)]
#[test]
fn watcher_rebinds_to_moved_root_and_tracking_off_projects_also_report_loss() {
    use std::{
        sync::{Arc, Mutex},
        thread,
        time::{Duration, Instant},
    };
    fn wait_for(mut predicate: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(15);
        while !predicate() {
            assert!(
                Instant::now() < deadline,
                "Folder recovery monitor timed out"
            );
            thread::sleep(Duration::from_millis(100));
        }
    }
    let f = Fixture::new(true);
    let did =
        f.r.snapshot()
            .unwrap()
            .documents
            .iter()
            .find(|d| d.name == "Notes.md")
            .unwrap()
            .document_id
            .clone();
    let id = f.id.clone();
    let root = f.root.clone();
    let target = f.root.parent().unwrap().join("Moved while watching");
    let shared = Arc::new(Mutex::new(f.r));
    let monitor = crate::monitor::Monitor::start(shared.clone(), |_| {}).unwrap();
    wait_for(|| {
        shared
            .lock()
            .unwrap()
            .tracking_view(&id)
            .unwrap()
            .watch_status
            == "watching"
    });
    fs::rename(&root, &target).unwrap();
    wait_for(|| {
        shared.lock().unwrap().snapshot().unwrap().projects[0].folder_state == "needs_reassignment"
    });
    {
        let mut r = shared.lock().unwrap();
        let plan = r.folder_plan(&id, target.to_str().unwrap()).unwrap();
        r.reassign_folder(&id, &plan.path, &plan.token).unwrap();
    }
    wait_for(|| {
        shared
            .lock()
            .unwrap()
            .tracking_view(&id)
            .unwrap()
            .watch_status
            == "watching"
    });
    fs::write(target.join("docs/Notes.md"), "Captured after move").unwrap();
    wait_for(|| shared.lock().unwrap().versions(&did).unwrap().len() == 2);
    shared
        .lock()
        .unwrap()
        .configure_tracking(&id, false, false, vec![])
        .unwrap();
    fs::rename(&target, &root).unwrap();
    wait_for(|| {
        shared
            .lock()
            .unwrap()
            .db
            .query_row(
                "SELECT folder_state FROM projects WHERE id=?1",
                [&id],
                |r| r.get::<_, String>(0),
            )
            .unwrap()
            == "needs_reassignment"
    });
    monitor.shutdown();
}

#[test]
fn refresh_all_reconnects_original_roots_and_preserves_history_selection_and_cloud_identity() {
    let mut f = Fixture::new(true);
    let before = f.r.snapshot().unwrap();
    let ids: Vec<_> = before.documents.iter().map(|d| d.id.clone()).collect();
    let versions: i64 =
        f.r.db
            .query_row("SELECT count(*) FROM document_versions", [], |r| r.get(0))
            .unwrap();
    f.r.db
        .execute(
            "UPDATE cloud_projects SET store_id='vs_retained',paused=1 WHERE project_id=?1",
            [&f.id],
        )
        .unwrap();
    let revision = crate::path_selection::revision(&f.r.db, &f.id).unwrap();
    f.r.save_path_selection(
        &f.id,
        revision,
        vec![crate::path_selection::Rule {
            path: "docs".into(),
            kind: "folder".into(),
        }],
    )
    .unwrap();
    let second = f.root.parent().unwrap().join("Second project");
    fs::create_dir(&second).unwrap();
    fs::write(second.join("Second.md"), "second").unwrap();
    let other = f.r.add("Second", second.to_str().unwrap()).unwrap();
    let moved = f.moved();
    let moved_second = second.with_extension("offline");
    fs::rename(&second, &moved_second).unwrap();
    f.r.check_folders().unwrap();
    fs::rename(&moved, &f.root).unwrap();
    fs::rename(&moved_second, &second).unwrap();
    f.r.refresh_folders().unwrap();
    let after = f.r.snapshot().unwrap();
    assert!(after.projects.iter().all(|p| p.folder_state == "ready"));
    assert_eq!(after.projects.len(), 2);
    assert!(after
        .projects
        .iter()
        .any(|p| p.id == other && !p.tracking_enabled));
    assert!(ids.iter().all(|id| after
        .documents
        .iter()
        .any(|d| &d.id == id && d.status == "available")));
    assert_eq!(
        f.r.db
            .query_row("SELECT count(*) FROM document_versions", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        versions
    );
    assert!(!crate::path_selection::allowed(&f.r.db, &f.id, "docs/Notes.md").unwrap());
    let cloud = cloud::project(&f.r.db, &f.id).unwrap();
    assert_eq!(s(&cloud, "store_id"), "vs_retained");
    assert_eq!(cloud::n(&cloud, "paused"), 1);
}
#[test]
fn a_replacement_directory_or_unverified_identity_still_requires_reassignment() {
    let mut f = Fixture::new(false);
    let moved = f.moved();
    f.r.check_folders().unwrap();
    fs::create_dir_all(f.root.join("docs")).unwrap();
    fs::copy(moved.join("docs/Notes.md"), f.root.join("docs/Notes.md")).unwrap();
    fs::copy(moved.join("Other.md"), f.root.join("Other.md")).unwrap();
    f.r.refresh_folders().unwrap();
    assert_eq!(
        f.r.snapshot().unwrap().projects[0].folder_state,
        "needs_reassignment"
    );
    assert!(folder_recovery::require_ready(&f.r.db, &f.id).is_err());
    let replacement = f.root.with_extension("replacement");
    fs::rename(&f.root, replacement).unwrap();
    fs::rename(&moved, &f.root).unwrap();
    f.r.db
        .execute(
            "UPDATE projects SET folder_identity=NULL WHERE id=?1",
            [&f.id],
        )
        .unwrap();
    f.r.refresh_folders().unwrap();
    assert_eq!(
        f.r.snapshot().unwrap().projects[0].folder_state,
        "needs_reassignment"
    );
}
#[test]
fn running_monitor_resumes_original_reconnected_folder_without_reassignment() {
    use std::{
        sync::{Arc, Mutex},
        thread,
        time::Duration,
    };
    fn wait(check: impl Fn() -> bool) {
        for _ in 0..150 {
            if check() {
                return;
            }
            thread::sleep(Duration::from_millis(100));
        }
        panic!("Monitor did not reach expected state");
    }
    let f = Fixture::new(true);
    let did =
        f.r.snapshot()
            .unwrap()
            .documents
            .iter()
            .find(|d| d.name == "Notes.md")
            .unwrap()
            .document_id
            .clone();
    let id = f.id.clone();
    let root = f.root.clone();
    let offline = root.with_extension("offline");
    let shared = Arc::new(Mutex::new(f.r));
    let monitor = crate::monitor::Monitor::start(shared.clone(), |_| {}).unwrap();
    wait(|| {
        shared
            .lock()
            .unwrap()
            .tracking_view(&id)
            .unwrap()
            .watch_status
            == "watching"
    });
    fs::rename(&root, &offline).unwrap();
    wait(|| {
        shared.lock().unwrap().snapshot().unwrap().projects[0].folder_state == "needs_reassignment"
    });
    fs::rename(&offline, &root).unwrap();
    wait(|| {
        let r = shared.lock().unwrap();
        r.tracking_view(&id).unwrap().watch_status == "watching"
            && folder_recovery::require_ready(&r.db, &id).is_ok()
    });
    fs::write(root.join("docs/Notes.md"), "Edited after drive reconnect").unwrap();
    wait(|| shared.lock().unwrap().versions(&did).unwrap().len() == 2);
    monitor.shutdown();
}
