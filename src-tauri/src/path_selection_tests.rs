use crate::{
    path_selection::{self, Rule},
    registry::Registry,
};
use std::{collections::HashSet, fs};
use tempfile::tempdir;
fn folder(path: &str) -> Rule {
    Rule {
        path: path.into(),
        kind: "folder".into(),
    }
}
#[test]
fn newly_discovered_unrelated_file_is_excluded_until_selected() {
    let t = tempdir().unwrap();
    let root = t.path().join("project");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("existing.md"), "existing").unwrap();
    fs::create_dir(t.path().join("app")).unwrap();
    let mut r = Registry::open(&t.path().join("app/registry.db")).unwrap();
    let id = r.add("Project", root.to_str().unwrap()).unwrap();
    r.configure_tracking(&id, true, false, r.source_ids(&id).unwrap())
        .unwrap();
    fs::write(root.join("new.md"), "new").unwrap();
    r.reconcile(&id, "test", &HashSet::new(), true).unwrap();
    let view = r.path_selection(&id).unwrap();
    assert!(view.entries.iter().any(|e| e.path == "new.md" && e.new));
    assert!(!path_selection::allowed(&r.db, &id, "new.md").unwrap());
    assert_eq!(
        r.db.query_row("SELECT COUNT(*) FROM document_versions", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    r.save_path_selection(&id, view.revision, vec![]).unwrap();
    r.reconcile(&id, "test", &HashSet::new(), true).unwrap();
    assert_eq!(
        r.db.query_row("SELECT COUNT(*) FROM document_versions", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert!(
        !r.path_selection(&id)
            .unwrap()
            .entries
            .iter()
            .find(|e| e.path == "new.md")
            .unwrap()
            .new
    );
}
#[test]
fn new_version_inherits_selected_family_but_not_excluded_parent() {
    let t = tempdir().unwrap();
    let root = t.path().join("project");
    fs::create_dir_all(root.join("private")).unwrap();
    fs::create_dir(t.path().join("app")).unwrap();
    fs::write(root.join("Report_v1.md"), "first").unwrap();
    fs::write(root.join("private/Notes_v1.md"), "private first").unwrap();
    let db = t.path().join("app/registry.db");
    let mut r = Registry::open(&db).unwrap();
    let id = r.add("Project", root.to_str().unwrap()).unwrap();
    r.configure_tracking(&id, true, false, vec![]).unwrap();
    let revision = path_selection::revision(&r.db, &id).unwrap();
    r.save_path_selection(&id, revision, vec![folder("private")])
        .unwrap();
    fs::write(root.join("Report_v2.md"), "second").unwrap();
    fs::write(root.join("private/Notes_v2.md"), "private second").unwrap();
    r.reconcile(&id, "test", &HashSet::new(), true).unwrap();
    assert!(path_selection::allowed(&r.db, &id, "Report_v2.md").unwrap());
    assert!(!path_selection::allowed(&r.db, &id, "private/Notes_v2.md").unwrap());
    let report = r
        .snapshot()
        .unwrap()
        .documents
        .into_iter()
        .find(|d| d.name == "Report_v2.md")
        .unwrap();
    assert_eq!(r.versions(&report.document_id).unwrap().len(), 2);
    drop(r);
    let mut r = Registry::open(&db).unwrap();
    r.reconcile(&id, "test", &HashSet::new(), true).unwrap();
    assert!(path_selection::allowed(&r.db, &id, "Report_v2.md").unwrap());
    let view = r.path_selection(&id).unwrap();
    assert!(view
        .entries
        .iter()
        .any(|e| e.path == "private/Notes_v2.md" && e.new));
    assert_eq!(r.versions(&report.document_id).unwrap().len(), 2);
}
#[test]
fn exclusions_cover_nested_future_files_persist_and_reinclude_without_losing_history() {
    let t = tempdir().unwrap();
    let root = t.path().join("project");
    fs::create_dir_all(root.join("Drafts/Nested")).unwrap();
    fs::create_dir_all(root.join("Drafts-copy")).unwrap();
    fs::create_dir_all(root.join("Empty")).unwrap();
    fs::write(root.join("Drafts/Nested/one.md"), "first").unwrap();
    fs::write(root.join("Drafts-copy/keep.md"), "keep").unwrap();
    fs::create_dir(t.path().join("app")).unwrap();
    let db = t.path().join("app/registry.db");
    let mut r = Registry::open(&db).unwrap();
    let id = r.add("Project", root.to_str().unwrap()).unwrap();
    r.configure_tracking(&id, true, false, r.source_ids(&id).unwrap())
        .unwrap();
    let view = r.path_selection(&id).unwrap();
    assert!(view
        .entries
        .iter()
        .any(|e| e.path == "Empty" && e.kind == "folder"));
    let before =
        crate::cloud::rows(&r.db, "SELECT * FROM document_versions ORDER BY id", &[]).unwrap();
    r.save_path_selection(&id, view.revision, vec![folder("DRAFTS")])
        .unwrap();
    assert_eq!(
        crate::cloud::rows(&r.db, "SELECT * FROM document_versions ORDER BY id", &[]).unwrap(),
        before
    );
    fs::write(root.join("Drafts/Nested/one.md"), "edited but excluded").unwrap();
    fs::write(root.join("Drafts/Nested/future.md"), "new excluded").unwrap();
    fs::write(root.join("Drafts-copy/keep.md"), "included edit").unwrap();
    drop(r);
    let mut r = Registry::open(&db).unwrap();
    r.reconcile(&id, "test", &HashSet::new(), true).unwrap();
    let count: i64 =
        r.db.query_row("SELECT COUNT(*) FROM document_versions", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 3);
    assert!(!path_selection::allowed(&r.db, &id, "drafts\\nested\\future.md").unwrap());
    assert!(path_selection::allowed(&r.db, &id, "Drafts-copy/keep.md").unwrap());
    let v = r.path_selection(&id).unwrap();
    r.save_path_selection(&id, v.revision, vec![]).unwrap();
    r.reconcile(&id, "test", &HashSet::new(), true).unwrap();
    assert_eq!(
        r.db.query_row("SELECT COUNT(*) FROM document_versions", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        5
    );
    assert_eq!(
        fs::read(root.join("Drafts/Nested/one.md")).unwrap(),
        b"edited but excluded"
    );
}
#[test]
fn invalid_stale_and_unverified_rule_writes_roll_back() {
    let t = tempdir().unwrap();
    fs::create_dir(t.path().join("p")).unwrap();
    let mut r = Registry::open(&t.path().join("registry.db")).unwrap();
    let id = r.add("P", t.path().join("p").to_str().unwrap()).unwrap();
    for path in [
        "../escape",
        "C:/absolute",
        "/absolute",
        "a//b",
        "a/../b",
        "a/.",
        "a.",
    ] {
        assert!(r.save_path_selection(&id, 0, vec![folder(path)]).is_err());
        assert!(path_selection::rules(&r.db, &id).unwrap().is_empty());
    }
    r.save_path_selection(&id, 0, vec![folder("Empty")])
        .unwrap();
    assert!(r.save_path_selection(&id, 0, vec![]).is_err());
    r.db.execute_batch("CREATE TRIGGER ignore_path_rule BEFORE INSERT ON path_exclusions BEGIN SELECT RAISE(IGNORE); END;").unwrap();
    assert!(r
        .save_path_selection(&id, 1, vec![folder("Other")])
        .is_err());
    assert_eq!(
        path_selection::rules(&r.db, &id).unwrap(),
        vec![folder("empty")]
    );
    assert_eq!(path_selection::revision(&r.db, &id).unwrap(), 1);
}
#[test]
fn schema_six_migration_retains_legacy_ignored_files_and_saved_history() {
    let t = tempdir().unwrap();
    let root = t.path().join("p");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("one.md"), "one").unwrap();
    fs::create_dir(t.path().join("app")).unwrap();
    let db = t.path().join("app/registry.db");
    let mut r = Registry::open(&db).unwrap();
    let id = r.add("P", root.to_str().unwrap()).unwrap();
    r.configure_tracking(&id, true, false, vec![]).unwrap();
    // Simulate a file ignored by the pre-folder-selection implementation.
    r.db.execute(
        "UPDATE source_files SET disposition='ignored' WHERE project_id=?1",
        [&id],
    )
    .unwrap();
    r.db.execute_batch(
        "DROP TABLE path_exclusions; DROP TABLE path_selection; PRAGMA user_version=6;",
    )
    .unwrap();
    drop(r);
    let r = Registry::open(&db).unwrap();
    assert!(!path_selection::allowed(&r.db, &id, "one.md").unwrap());
    assert_eq!(r.snapshot().unwrap().projects[0].id, id);
}
#[test]
fn excluded_pending_capture_stays_paused_until_reincluded_with_snapshot_bytes_retained() {
    let t = tempdir().unwrap();
    let root = t.path().join("project");
    fs::create_dir(&root).unwrap();
    fs::create_dir(t.path().join("app")).unwrap();
    let db = t.path().join("app/registry.db");
    let mut r = Registry::open(&db).unwrap();
    fs::write(root.join("one.md"), "initial").unwrap();
    let id = r.add("P", root.to_str().unwrap()).unwrap();
    r.configure_tracking(&id, true, false, r.source_ids(&id).unwrap())
        .unwrap();
    fs::write(root.join("one.md"), "changed").unwrap();
    r.db.execute_batch("CREATE TRIGGER fail_capture BEFORE INSERT ON document_versions BEGIN SELECT RAISE(ABORT,'synthetic interrupted capture'); END;").unwrap();
    r.reconcile(&id, "test", &HashSet::new(), true).unwrap();
    assert_eq!(
        r.db.query_row(
            "SELECT COUNT(*) FROM operations WHERE kind='capture' AND state!='done'",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    let rev = path_selection::revision(&r.db, &id).unwrap();
    r.save_path_selection(
        &id,
        rev,
        vec![Rule {
            path: "one.md".into(),
            kind: "file".into(),
        }],
    )
    .unwrap();
    r.db.execute_batch("DROP TRIGGER fail_capture;").unwrap();
    assert!(r.recover_operations(&id, true).unwrap().is_empty());
    assert_eq!(
        r.db.query_row("SELECT COUNT(*) FROM document_versions", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    let rev = path_selection::revision(&r.db, &id).unwrap();
    r.save_path_selection(&id, rev, vec![]).unwrap();
    assert!(r.recover_operations(&id, true).unwrap().is_empty());
    assert_eq!(
        r.db.query_row("SELECT COUNT(*) FROM document_versions", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(fs::read(root.join("one.md")).unwrap(), b"changed");
}
