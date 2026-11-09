use crate::{latest_tracking, registry::Registry};
use std::{collections::HashSet, fs};
use tempfile::tempdir;

#[test]
fn latest_tracking_does_not_create_archives_or_snapshot_bytes_and_reuses_save_metadata() {
    let t = tempdir().unwrap();
    let root = t.path().join("Project");
    let app = t.path().join("App");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&app).unwrap();
    fs::write(root.join("ContextDock_Context_v1.md"), "one").unwrap();
    let mut r = Registry::open(&app.join("registry.db")).unwrap();
    assert!(latest_tracking::enabled(&r.db));
    let project = r.add("Project", root.to_str().unwrap()).unwrap();
    r.configure_tracking(&project, true, true, vec![]).unwrap();
    assert!(!r.tracking_flags(&project).unwrap().1);
    for n in 0..8 {
        fs::write(root.join("ContextDock_Context_v1.md"), format!("save {n}")).unwrap();
        r.reconcile(&project, "watcher", &HashSet::new(), true)
            .unwrap();
    }
    assert_eq!(
        r.db.query_row("SELECT COUNT(*) FROM document_versions", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert!(!app.join("snapshots").exists());
    assert!(!app.join("staging").exists());
    assert!(!root.join("Old Context Files").exists());
    assert_eq!(
        fs::read_to_string(root.join("ContextDock_Context_v1.md")).unwrap(),
        "save 7"
    );
}

#[test]
fn latest_tracking_groups_numbered_files_and_keeps_old_local_sources_in_place() {
    let t = tempdir().unwrap();
    let root = t.path().join("Project");
    let app = t.path().join("App");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&app).unwrap();
    fs::write(root.join("ContextDock_Context_v1.md"), "one").unwrap();
    fs::write(root.join("ContextDock_Context_v2.md"), "two").unwrap();
    let mut r = Registry::open(&app.join("registry.db")).unwrap();
    let project = r.add("Project", root.to_str().unwrap()).unwrap();
    r.configure_tracking(&project, true, false, vec![]).unwrap();
    let families = crate::drive_publish::eligible_documents(&r.db, &project).unwrap();
    assert_eq!(families.len(), 1);
    assert_eq!(
        crate::drive_publish::current(&r.db, &project, &families[0])
            .unwrap()
            .name,
        "ContextDock_Context_v2.md"
    );
    assert!(root.join("ContextDock_Context_v1.md").exists());
    assert!(root.join("ContextDock_Context_v2.md").exists());
    assert!(!app.join("snapshots").exists());
}

#[test]
fn latest_tracking_upgrade_preserves_existing_snapshot_and_archived_source_bytes() {
    let t = tempdir().unwrap();
    let root = t.path().join("Project");
    let app = t.path().join("App");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&app).unwrap();
    fs::write(root.join("ContextDock_Context_v1.md"), "legacy content").unwrap();
    let db = app.join("registry.db");
    let mut old = Registry::open_legacy_fixture(&db).unwrap();
    let project = old.add("Project", root.to_str().unwrap()).unwrap();
    old.configure_tracking(&project, true, true, vec![])
        .unwrap();
    let snapshot: String = old
        .db
        .query_row(
            "SELECT snapshot_relative FROM document_versions LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    old.db.execute("INSERT INTO project_chats(id,project_id,title,created_at,updated_at) VALUES('preserved-chat',?1,'Saved chat','sample','sample')",[&project]).unwrap();
    fs::create_dir(root.join("Old Context Files")).unwrap();
    fs::write(
        root.join("Old Context Files/ContextDock_Archive_v1.md"),
        "preserved archive",
    )
    .unwrap();
    old.db
        .execute_batch(
            "DELETE FROM preferences WHERE key='workflow.latest_only'; PRAGMA user_version=15;",
        )
        .unwrap();
    drop(old);
    let mut r = Registry::open(&db).unwrap();
    assert!(latest_tracking::enabled(&r.db));
    assert!(!r.tracking_flags(&project).unwrap().1);
    fs::write(root.join("ContextDock_Context_v1.md"), "current content").unwrap();
    r.reconcile(&project, "watcher", &HashSet::new(), true)
        .unwrap();
    assert_eq!(
        fs::read_to_string(app.join(snapshot)).unwrap(),
        "legacy content"
    );
    assert_eq!(
        fs::read_to_string(root.join("Old Context Files/ContextDock_Archive_v1.md")).unwrap(),
        "preserved archive"
    );
    assert_eq!(fs::read_dir(app.join("snapshots")).unwrap().count(), 1);
    assert_eq!(
        r.db.query_row(
            "SELECT title FROM project_chats WHERE id='preserved-chat'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "Saved chat"
    );
}

#[test]
fn latest_tracking_forgetting_a_missing_path_keeps_document_identity_and_saved_metadata() {
    let t = tempdir().unwrap();
    let root = t.path().join("Project");
    let app = t.path().join("App");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&app).unwrap();
    let source = root.join("ContextDock_Context_v1.md");
    fs::write(&source, "one").unwrap();
    let mut r = Registry::open(&app.join("registry.db")).unwrap();
    let project = r.add("Project", root.to_str().unwrap()).unwrap();
    r.configure_tracking(&project, true, false, vec![]).unwrap();
    let before: (String, String) =
        r.db.query_row("SELECT id,document_id FROM source_files LIMIT 1", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    fs::remove_file(&source).unwrap();
    r.refresh(&project).unwrap();
    let plan = r
        .preview_forget(&project, "ContextDock_Context_v1.md")
        .unwrap();
    assert_eq!(plan.revision_count, 0);
    r.forget_missing(&project, &plan.path, &plan.token).unwrap();
    assert!(r.snapshot().unwrap().documents.is_empty());
    assert!(r
        .path_selection(&project)
        .unwrap()
        .entries
        .iter()
        .all(|entry| entry.kind != "file"));
    assert_eq!(
        r.db.query_row("SELECT COUNT(*) FROM document_versions", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert!(crate::drive_publish::eligible_documents(&r.db, &project)
        .unwrap()
        .is_empty());
    fs::write(&source, "one").unwrap();
    r.refresh(&project).unwrap();
    let after: (String, String) =
        r.db.query_row("SELECT id,document_id FROM source_files LIMIT 1", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!(before, after);
}
