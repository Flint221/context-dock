use crate::{path_selection, registry::Registry};
use std::{collections::HashSet, fs};
use tempfile::{tempdir, TempDir};
struct Fixture {
    _temp: TempDir,
    root: std::path::PathBuf,
    db: std::path::PathBuf,
    r: Registry,
    id: String,
}
impl Fixture {
    fn new(files: &[(&str, &str)]) -> Self {
        let t = tempdir().unwrap();
        let root = t.path().join("project");
        fs::create_dir(&root).unwrap();
        let app = t.path().join("app");
        fs::create_dir(&app).unwrap();
        for (path, text) in files {
            fs::write(root.join(path), text).unwrap();
        }
        let db = app.join("registry.db");
        let mut r = Registry::open(&db).unwrap();
        let id = r.add("Disposable", root.to_str().unwrap()).unwrap();
        r.configure_tracking(&id, true, false, vec![]).unwrap();
        Self {
            _temp: t,
            root,
            db,
            r,
            id,
        }
    }
    fn count(&self, sql: &str) -> i64 {
        self.r.db.query_row(sql, [], |r| r.get(0)).unwrap()
    }
}
#[test]
fn forgetting_unapproved_deleted_file_clears_discovery_pending_rules_and_restart() {
    let mut f = Fixture::new(&[("Keep_v1.md", "keep")]);
    fs::write(f.root.join("ContextDock_Approval_Test_v1.md"), "test").unwrap();
    f.r.reconcile(&f.id, "test", &HashSet::new(), true).unwrap();
    fs::remove_file(f.root.join("ContextDock_Approval_Test_v1.md")).unwrap();
    let plan =
        f.r.preview_forget(&f.id, "ContextDock_Approval_Test_v1.md")
            .unwrap();
    assert_eq!(plan.revision_count, 0);
    assert!(plan.blockers.is_empty());
    assert!(f
        .r
        .forget_missing(&f.id, &plan.path, &plan.token)
        .unwrap()
        .is_empty());
    assert_eq!(f.count("SELECT count(*) FROM documents"), 1);
    assert_eq!(f.count("SELECT count(*) FROM new_paths"), 0);
    assert_eq!(f.count("SELECT count(*) FROM path_exclusions"), 0);
    drop(f.r);
    f.r = Registry::open(&f.db).unwrap();
    assert_eq!(
        f.r.path_selection(&f.id)
            .unwrap()
            .entries
            .iter()
            .filter(|e| e.kind == "file")
            .count(),
        1
    );
    fs::write(f.root.join(&plan.path), "recreated").unwrap();
    f.r.reconcile(&f.id, "test", &HashSet::new(), true).unwrap();
    assert!(!path_selection::allowed(&f.r.db, &f.id, &plan.path).unwrap());
    assert_eq!(f.count("SELECT count(*) FROM document_versions"), 1);
}
#[test]
fn forgetting_deleted_tracked_file_deletes_all_its_snapshots_and_preserves_other_data() {
    let mut f = Fixture::new(&[("Remove_v1.md", "first"), ("Keep_v1.md", "keep")]);
    fs::write(f.root.join("Remove_v1.md"), "second longer").unwrap();
    f.r.reconcile(&f.id, "test", &HashSet::new(), true).unwrap();
    let snapshots: Vec<String> = crate::cloud::rows(
        &f.r.db,
        "SELECT snapshot_relative FROM document_versions WHERE source_name='Remove_v1.md'",
        &[],
    )
    .unwrap()
    .iter()
    .map(|v| crate::cloud::s(v, "snapshot_relative").into())
    .collect();
    assert_eq!(snapshots.len(), 2);
    fs::create_dir(f.root.join("Old Context Files")).unwrap();
    fs::write(
        f.root.join("Old Context Files/Backup_v1.md"),
        "preserved archive",
    )
    .unwrap();
    fs::remove_file(f.root.join("Remove_v1.md")).unwrap();
    let plan = f.r.preview_forget(&f.id, "Remove_v1.md").unwrap();
    assert_eq!(plan.revision_count, 2);
    assert!(plan.blockers.is_empty(), "{:?}", plan.blockers);
    assert!(f
        .r
        .forget_missing(&f.id, &plan.path, &plan.token)
        .unwrap()
        .is_empty());
    assert!(snapshots.iter().all(|p| !f.r.store.join(p).exists()));
    assert_eq!(f.count("SELECT count(*) FROM document_versions"), 1);
    assert_eq!(f.count("SELECT count(*) FROM pragma_foreign_key_check"), 0);
    assert_eq!(
        fs::read_to_string(f.root.join("Keep_v1.md")).unwrap(),
        "keep"
    );
    assert_eq!(
        fs::read_to_string(f.root.join("Old Context Files/Backup_v1.md")).unwrap(),
        "preserved archive"
    );
}
#[test]
fn forgetting_one_missing_version_keeps_the_other_versions_and_current_source() {
    let mut f = Fixture::new(&[("Report_v1.md", "old"), ("Report_v2.md", "current")]);
    fs::remove_file(f.root.join("Report_v1.md")).unwrap();
    let plan = f.r.preview_forget(&f.id, "Report_v1.md").unwrap();
    assert!(plan.blockers.is_empty(), "{:?}", plan.blockers);
    f.r.forget_missing(&f.id, &plan.path, &plan.token).unwrap();
    assert_eq!(f.count("SELECT count(*) FROM document_versions"), 1);
    let view = f.r.tracking_view(&f.id).unwrap();
    let family = view
        .families
        .iter()
        .find(|f| f.active_filename.as_deref() == Some("Report_v2.md"))
        .unwrap();
    assert_eq!(family.version_count, 1);
    assert!(f.root.join("Report_v2.md").exists());
    assert_eq!(f.count("SELECT count(*) FROM pragma_foreign_key_check"), 0);
    fs::remove_file(f.root.join("Report_v2.md")).unwrap();
    let last = f.r.preview_forget(&f.id, "Report_v2.md").unwrap();
    assert!(last.blockers.is_empty(), "{:?}", last.blockers);
    f.r.forget_missing(&f.id, &last.path, &last.token).unwrap();
    assert!(f.r.tracking_view(&f.id).unwrap().families.is_empty());
    assert_eq!(f.count("SELECT count(*) FROM pragma_foreign_key_check"), 0);
}
#[test]
fn reappeared_files_changed_history_cloud_references_and_missing_root_prevent_deletion() {
    let mut f = Fixture::new(&[("Test_v1.md", "test")]);
    fs::remove_file(f.root.join("Test_v1.md")).unwrap();
    let plan = f.r.preview_forget(&f.id, "Test_v1.md").unwrap();
    fs::write(f.root.join(&plan.path), "returned").unwrap();
    assert!(f.r.forget_missing(&f.id, &plan.path, &plan.token).is_err());
    fs::remove_file(f.root.join(&plan.path)).unwrap();
    f.r.db
        .execute("UPDATE cloud_projects SET revision=revision+1", [])
        .unwrap();
    assert!(f
        .r
        .forget_missing(&f.id, &plan.path, &plan.token)
        .unwrap_err()
        .contains("changed"));
    f.r.db.execute("INSERT INTO cloud_jobs(id,project_id,document_id,version_id,remote_name,file_id,state,updated_at) SELECT 'job',d.project_id,d.id,v.id,'owned-test','file-test','indexed','now' FROM documents d JOIN document_versions v ON v.document_id=d.id",[]).unwrap();
    let blocked = f.r.preview_forget(&f.id, &plan.path).unwrap();
    assert!(blocked.blockers.iter().any(|b| b.contains("cloud")));
    assert!(f
        .r
        .forget_missing(&f.id, &blocked.path, &blocked.token)
        .is_err());
    fs::rename(&f.root, f.root.with_extension("moved")).unwrap();
    assert!(f.r.preview_forget(&f.id, &plan.path).is_err());
    assert_eq!(f.count("SELECT count(*) FROM document_versions"), 1);
}
#[test]
#[cfg(windows)]
fn locked_snapshot_cleanup_is_durable_and_retried_after_restart() {
    let mut f = Fixture::new(&[("Test_v1.md", "test")]);
    let relative: String =
        f.r.db
            .query_row("SELECT snapshot_relative FROM document_versions", [], |r| {
                r.get(0)
            })
            .unwrap();
    let lock = crate::storage::locked_read(&f.r.store.join(&relative), false).unwrap();
    fs::remove_file(f.root.join("Test_v1.md")).unwrap();
    let plan = f.r.preview_forget(&f.id, "Test_v1.md").unwrap();
    assert!(!f
        .r
        .forget_missing(&f.id, &plan.path, &plan.token)
        .unwrap()
        .is_empty());
    assert_eq!(f.count("SELECT count(*) FROM forgotten_snapshots"), 1);
    assert!(f.r.store.join(&relative).exists());
    drop(lock);
    drop(f.r);
    f.r = Registry::open(&f.db).unwrap();
    assert!(!f.r.store.join(relative).exists());
    assert_eq!(f.count("SELECT count(*) FROM forgotten_snapshots"), 0);
}
#[test]
fn cleanup_refuses_corrupt_or_outside_snapshots_and_preserves_queue() {
    let f = Fixture::new(&[("Keep_v1.md", "keep")]);
    let path =
        f.r.store
            .join("snapshots")
            .join(&f.id)
            .join("unreferenced.md");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "do not delete").unwrap();
    let relative = format!("snapshots/{}/unreferenced.md", f.id);
    f.r.db
        .execute(
            "INSERT INTO forgotten_snapshots VALUES(?1,'wrong',?2,NULL)",
            rusqlite::params![relative, f.id],
        )
        .unwrap();
    f.r.db
        .execute(
            "INSERT INTO forgotten_snapshots VALUES('../outside.md','wrong',?1,NULL)",
            [&f.id],
        )
        .unwrap();
    assert_eq!(f.r.purge_forgotten_snapshots().unwrap().len(), 2);
    assert_eq!(fs::read_to_string(path).unwrap(), "do not delete");
    assert_eq!(f.count("SELECT count(*) FROM forgotten_snapshots"), 2);
}
