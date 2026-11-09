//! Explicit live acceptance: marked pilot only, fresh disposable files, no UI input.
use crate::{
    drive_api::{ApiError, DriveApi, GoogleDrive},
    drive_auth::{self, NativeVault, Vault},
    drive_publication_commands as commands, drive_publish as publication,
    registry::Registry,
};
use rusqlite::params;
use serde_json::json;
use std::{
    collections::HashSet,
    fs,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

fn publish_retry(
    r: &Registry,
    project: &str,
    account: &str,
    doc: &str,
    parent: &str,
    api: &dyn DriveApi,
) {
    for attempt in 0..8 {
        match publication::publish(r, project, account, doc, parent, api) {
            Ok(()) => return,
            Err(e) if e.retry && attempt < 7 => {
                commands::record_error(r, project, Some(doc), &e).unwrap();
                std::thread::sleep(Duration::from_secs(e.delay.clamp(1, 30)));
            }
            Err(e) => panic!("Live publication did not verify: {}", e.message),
        }
    }
    unreachable!()
}

#[test]
#[ignore = "Explicit live Google and Windows Recycle Bin test; pilot vault and disposable files only"]
fn live_google_recovery_refresh_and_verified_recycling() {
    assert_eq!(
        std::env::var("CONTEXTDOCK_RUN_LIVE_CHECKS").unwrap(),
        "disposable-only"
    );
    let version: u32 = std::env::var("CONTEXTDOCK_LIVE_CHECK_VERSION")
        .unwrap()
        .parse()
        .unwrap();
    assert!(version > 0);
    let data = crate::profile::pilot_data_dir().unwrap();
    let pilot = data.parent().unwrap();
    let base = pilot.join(format!("ContextDock_Live_Check_v{version}"));
    fs::create_dir(&base).expect("Preserve previous live fixtures; select the next version.");
    let root = base.join("Project");
    let app = base.join("Data");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&app).unwrap();
    let mut saved = NativeVault
        .load()
        .unwrap()
        .expect("Connect the isolated pilot first.");
    let account = saved
        .session
        .as_ref()
        .expect("Sign in to the pilot first.")
        .account
        .id
        .clone();
    // Exercise the real refresh endpoint without revoking the user's grant.
    saved.session.as_mut().unwrap().expires_at = 0;
    NativeVault.save(&saved).unwrap();
    drop(saved);
    let now = chrono::Utc::now().timestamp();
    let access =
        drive_auth::access(&NativeVault, &drive_auth::GoogleHttp::new().unwrap(), now).unwrap();
    assert!(
        NativeVault
            .load()
            .unwrap()
            .unwrap()
            .session
            .as_ref()
            .unwrap()
            .expires_at
            > now + 60
    );
    println!("Live check: Google token refresh persisted successfully.");
    let old = rusqlite::Connection::open_with_flags(
        data.join("contextdock.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let root_id: String = old
        .query_row(
            "SELECT root_id FROM drive_accounts WHERE account_id=?1 AND root_ready=1",
            [&account],
            |r| r.get(0),
        )
        .unwrap();
    drop(old);
    fs::write(
        root.join("ContextDock_Context_v1.md"),
        b"disposable original Markdown",
    )
    .unwrap();
    fs::write(
        root.join("ContextDock_Notes_v1.docx"),
        crate::drive_publish_tests::sample_docx("disposable original Word"),
    )
    .unwrap();
    let db = app.join("contextdock.db");
    let mut r = Registry::open(&db).unwrap();
    let project = r
        .add(
            &format!("ContextDock Live Check v{version}"),
            root.to_str().unwrap(),
        )
        .unwrap();
    r.configure_tracking(&project, true, false, vec![]).unwrap();
    publication::configure(&r.db, &project, &account, true, false).unwrap();
    let api = commands::api(
        db.clone(),
        project.clone(),
        account.clone(),
        None,
        access,
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap();
    publication::adopt_root(&r.db, &account, &root_id, &api).unwrap();
    r.db.execute(
        "INSERT INTO drive_folder_choices(project_id,may_create) VALUES(?1,1)",
        [&project],
    )
    .unwrap();
    let parent = publication::ensure_destination(&r.db, &project, &account, &api).unwrap();
    let docs = publication::eligible_documents(&r.db, &project).unwrap();
    assert_eq!(docs.len(), 2);
    for doc in &docs {
        publish_retry(&r, &project, &account, doc, &parent, &api);
    }
    let first = publication::publications(&r.db, &project).unwrap();
    println!("Live check: initial Markdown and Word bytes verified.");
    fs::write(
        root.join("ContextDock_Context_v2.md"),
        b"disposable second Markdown",
    )
    .unwrap();
    fs::write(
        root.join("ContextDock_Notes_v2.docx"),
        crate::drive_publish_tests::sample_docx("disposable second Word"),
    )
    .unwrap();
    r.reconcile(&project, "watcher", &HashSet::new(), true)
        .unwrap();
    for doc in &docs {
        publish_retry(&r, &project, &account, doc, &parent, &api);
    }
    let markdown = docs
        .iter()
        .find(|d| {
            publication::current(&r.db, &project, d)
                .unwrap()
                .name
                .ends_with(".md")
        })
        .unwrap()
        .clone();
    fs::write(
        root.join("ContextDock_Context_v3.md"),
        b"disposable recovered Markdown",
    )
    .unwrap();
    r.reconcile(&project, "watcher", &HashSet::new(), true)
        .unwrap();
    let access = drive_auth::access(
        &NativeVault,
        &drive_auth::GoogleHttp::new().unwrap(),
        chrono::Utc::now().timestamp(),
    )
    .unwrap();
    let writes = Arc::new(AtomicUsize::new(0));
    let interrupted = Arc::new(AtomicBool::new(false));
    let (count, cut) = (writes.clone(), interrupted.clone());
    let guard = commands::open(&db).unwrap();
    let guard_project = project.clone();
    let guard_account = account.clone();
    let interrupted_api=GoogleDrive::new(access,move |write| {
        publication::require_enabled(&guard.db,&guard_project,&guard_account)?;
        if write { count.fetch_add(1,Ordering::SeqCst); }
        else if count.load(Ordering::SeqCst)>=2 && !cut.swap(true,Ordering::SeqCst) {
            return Err(ApiError{retry:true,..ApiError::local("Injected interruption after the live remote write; verification receipt remains pending.")});
        }
        Ok(())
    }).unwrap();
    assert!(
        publication::publish(&r, &project, &account, &markdown, &parent, &interrupted_api).is_err()
    );
    assert!(interrupted.load(Ordering::SeqCst));
    let pending = publication::publications(&r.db, &project).unwrap();
    assert!(pending
        .iter()
        .find(|p| p.document_id == markdown)
        .unwrap()
        .pending_hash
        .is_some());
    drop(interrupted_api);
    drop(api);
    drop(r);
    let r = commands::open(&db).unwrap();
    let access = drive_auth::access(
        &NativeVault,
        &drive_auth::GoogleHttp::new().unwrap(),
        chrono::Utc::now().timestamp(),
    )
    .unwrap();
    let api = commands::api(
        db.clone(),
        project.clone(),
        account.clone(),
        None,
        access,
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap();
    publish_retry(&r, &project, &account, &markdown, &parent, &api);
    let current = publication::publications(&r.db, &project).unwrap();
    for before in first {
        assert_eq!(
            before.file_id,
            current
                .iter()
                .find(|p| p.document_id == before.document_id)
                .unwrap()
                .file_id
        );
    }
    assert!(current
        .iter()
        .all(|p| p.state == "synced" && p.pending_hash.is_none()));
    println!("Live check: remote-write interruption recovered after registry reopen with unchanged Drive IDs.");
    // Deliberately change an older disposable file after capture. Cleanup must
    // compare fresh bytes with its receipt and keep it, without rescanning it.
    fs::write(
        root.join("ContextDock_Context_v1.md"),
        b"changed older disposable file; must be retained",
    )
    .unwrap();
    crate::drive_recycle::configure(&r.db, &project, true).unwrap();
    let mut recycled = 0;
    for doc in &docs {
        publish_retry(&r, &project, &account, doc, &parent, &api);
        recycled += crate::drive_recycle::run(
            &r,
            &project,
            &account,
            doc,
            &parent,
            &api,
            &crate::recycle_bin::NativeBin,
        )
        .unwrap();
    }
    assert_eq!(recycled, 2);
    assert!(root.join("ContextDock_Context_v1.md").is_file());
    assert!(!root.join("ContextDock_Context_v2.md").exists());
    assert!(!root.join("ContextDock_Notes_v1.docx").exists());
    assert!(root.join("ContextDock_Context_v3.md").is_file());
    assert!(root.join("ContextDock_Notes_v2.docx").is_file());
    let receipts:i64=r.db.query_row("SELECT COUNT(*) FROM drive_recycle WHERE project_id=?1 AND state='recycled' AND bin_receipt IS NOT NULL",[&project],|r|r.get(0)).unwrap();
    assert_eq!(receipts, 2);
    crate::drive_recycle::configure(&r.db, &project, false).unwrap();
    publication::configure(&r.db, &project, &account, false, false).unwrap();
    let files = publication::publications(&r.db, &project).unwrap();
    fs::write(base.join("ContextDock_Live_Check_v1.json"),serde_json::to_vec_pretty(&json!({"project":project,"folderId":parent,"tokenRefresh":true,"remoteWriteRecovery":true,"recycled":receipts,"changedOlderRetained":true,"newestKept":true,"recyclingOff":true,"publications":files})).unwrap()).unwrap();
    println!("Live check: two unchanged older files reached the Windows Recycle Bin; changed older and newest files retained. Test project publication and recycling are off.");
    println!("Live evidence folder: {}", base.display());
    let enabled: bool =
        r.db.query_row(
            "SELECT recycle FROM drive_projects WHERE project_id=?1",
            params![project],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!enabled);
}
