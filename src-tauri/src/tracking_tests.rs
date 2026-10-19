use crate::{
    models,
    operations::{ArchivePlan, CapturePlan},
    registry::Registry,
    storage,
};
use std::{collections::HashSet, fs, path::PathBuf};
use tempfile::{tempdir, TempDir};
struct Fixture {
    _temp: TempDir,
    root: PathBuf,
    r: Registry,
    project: String,
}
impl Fixture {
    fn new(files: &[(&str, &str)]) -> Self {
        let temp = tempdir().unwrap();
        let root = temp.path().join("Project Ω");
        let store = temp.path().join("app");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir(&store).unwrap();
        for (name, bytes) in files {
            let p = root.join(name);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, bytes).unwrap();
        }
        let mut r = Registry::open(&store.join("registry.db")).unwrap();
        let project = r.add("Test", root.to_str().unwrap()).unwrap();
        Self {
            _temp: temp,
            root,
            r,
            project,
        }
    }
    fn enable(&mut self, archive: bool) {
        let ids = self.r.source_ids(&self.project).unwrap();
        self.r
            .configure_tracking(&self.project, true, archive, ids)
            .unwrap();
        let state = self.r.tracking_view(&self.project).unwrap();
        assert!(state.error.is_none(), "{:?}", state.error);
    }
    fn scan(&mut self) {
        self.r
            .reconcile(&self.project, "manual_refresh", &HashSet::new(), true)
            .unwrap();
    }
    fn document(&self, name: &str) -> String {
        self.r
            .snapshot()
            .unwrap()
            .documents
            .iter()
            .find(|s| s.name == name)
            .unwrap()
            .document_id
            .clone()
    }
    fn count(&self, document: &str) -> usize {
        self.r.versions(document).unwrap().len()
    }
    fn pending_archive(&self) -> (String, ArchivePlan) {
        let(id,json):(String,String)=self.r.db.query_row("SELECT id,payload FROM operations WHERE kind='archive' AND state!='done' ORDER BY rowid DESC LIMIT 1",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
        (id, serde_json::from_str(&json).unwrap())
    }
    fn prepare_replacement(&mut self, old: &str, new: &str, bytes: &str) -> String {
        let document = self.document(old);
        fs::write(self.root.join(new), bytes).unwrap();
        self.r.refresh(&self.project).unwrap();
        let sid = self
            .r
            .snapshot()
            .unwrap()
            .documents
            .iter()
            .find(|s| s.name == new)
            .unwrap()
            .id
            .clone();
        self.r
            .db
            .execute(
                "UPDATE source_files SET document_id=?1,disposition='tracked' WHERE id=?2",
                rusqlite::params![document, sid],
            )
            .unwrap();
        let source = self.r.source(&sid).unwrap();
        let root = PathBuf::from(self.r.path(&self.project).unwrap());
        let stage = storage::stage(&root, &source.relative, &self.r.store).unwrap();
        self.r
            .capture(&source, stage, "new_replacement", true)
            .unwrap();
        document
    }
}
#[test]
fn v1_migration_preserves_ids_preferences_and_does_not_capture_or_move() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("project");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("Master_v1.md"), "original").unwrap();
    let db = temp.path().join("registry.db");
    {
        let c = rusqlite::Connection::open(&db).unwrap();
        c.execute_batch(include_str!("../migrations/001_registry.sql"))
            .unwrap();
        c.execute("INSERT INTO projects(id,name,path,path_key,created_at) VALUES('project-id','Custom',?1,?1,'2026-01-01T00:00:00Z')",[crate::scanner::canonical(root.to_str().unwrap()).unwrap()]).unwrap();
        c.execute_batch("INSERT INTO documents(id,project_id,name,relative_path,path_key,kind,size,modified_at,last_observed,status) VALUES('document-id','project-id','Master_v1.md','Master_v1.md','master_v1.md','markdown',8,NULL,'2026-01-01T00:00:00Z','available'); INSERT INTO preferences VALUES('theme','dark'); PRAGMA user_version=1;").unwrap();
    }
    let r = Registry::open(&db).unwrap();
    let s = r.snapshot().unwrap();
    assert_eq!(s.projects[0].id, "project-id");
    assert_eq!(s.documents[0].id, "document-id");
    assert_eq!(s.documents[0].document_id, "document-id");
    assert_eq!(s.theme, "dark");
    assert!(!s.projects[0].tracking_enabled);
    assert!(!s.projects[0].archive_enabled);
    assert!(r.versions("document-id").unwrap().is_empty());
    assert!(!root.join("Old Context Files").exists());
    assert_eq!(
        fs::read_to_string(root.join("Master_v1.md")).unwrap(),
        "original"
    );
    drop(r);
    Registry::open(&db).unwrap();
}
#[test]
fn matching_only_normalizes_explicit_trailing_versions_and_dates() {
    use crate::matching::stem;
    for name in [
        "Boyd_Master_Context_2026-09-28_v4.md",
        "Boyd_Master_Context_20261019_version_3.markdown",
        "Boyd_Master_Context_v2.1.docx",
    ] {
        assert_eq!(stem(name).normalized, "boyd master context");
        assert!(stem(name).versioned);
    }
    assert_ne!(
        stem("Boyd_Master_Context.md").normalized,
        stem("Boyd_Technical_Documentation.md").normalized
    );
    assert_eq!(
        stem("v2_Workstream_2026_Draft.md").normalized,
        "v2 workstream 2026 draft"
    );
}
#[test]
fn independent_initial_versions_same_path_archival_and_duplicate_hashes() {
    let mut f = Fixture::new(&[
        ("Master_Context_v1.md", "master one"),
        ("Technical_Documentation_v1.md", "technical one"),
    ]);
    f.enable(true);
    let master = f.document("Master_Context_v1.md");
    let technical = f.document("Technical_Documentation_v1.md");
    assert_ne!(master, technical);
    assert_eq!(f.count(&master), 1);
    assert_eq!(f.count(&technical), 1);
    assert!(!f.root.join("Old Context Files").exists());
    fs::write(f.root.join("Master_Context_v1.md"), "master two").unwrap();
    f.scan();
    assert_eq!(f.count(&master), 2);
    assert_eq!(f.count(&technical), 1);
    let v = f.r.versions(&master).unwrap();
    assert_eq!(
        fs::read_to_string(&v[1].snapshot_path).unwrap(),
        "master one"
    );
    assert_eq!(
        fs::read_to_string(v[1].archive_path.as_ref().unwrap()).unwrap(),
        "master one"
    );
    assert!(v[0].current);
    assert_eq!(
        fs::read_to_string(f.root.join("Master_Context_v1.md")).unwrap(),
        "master two"
    );
    for _ in 0..3 {
        f.scan()
    }
    assert_eq!(f.count(&master), 2);
    assert!(f.r.tracking_view(&f.project).unwrap().operations.is_empty());
    fs::write(f.root.join("Master_Context_v1.md"), "master one").unwrap();
    f.scan();
    assert_eq!(f.count(&master), 2);
    assert_eq!(
        f.r.versions(&master)
            .unwrap()
            .iter()
            .find(|v| v.current)
            .unwrap()
            .number,
        1
    );
}
#[test]
fn confident_new_filename_archives_previous_source_and_retains_family_id() {
    let mut f = Fixture::new(&[("Master_Context_2026-10-08_v1.md", "one")]);
    f.enable(true);
    let id = f.document("Master_Context_2026-10-08_v1.md");
    fs::write(f.root.join("Master_Context_2026-10-19_v2.md"), "two").unwrap();
    f.scan();
    let state = f.r.tracking_view(&f.project).unwrap();
    assert!(state.error.is_none(), "{:?}", state.error);
    assert!(state.reviews.is_empty());
    assert_eq!(f.document("Master_Context_2026-10-19_v2.md"), id);
    assert_eq!(f.count(&id), 2);
    assert!(!f.root.join("Master_Context_2026-10-08_v1.md").exists());
    assert!(f.root.join("Master_Context_2026-10-19_v2.md").exists());
    let v = f.r.versions(&id).unwrap();
    assert_eq!(
        fs::read_to_string(v[1].archive_path.as_ref().unwrap()).unwrap(),
        "one"
    );
    let before = f.r.snapshot().unwrap().documents.len();
    f.scan();
    assert_eq!(f.r.snapshot().unwrap().documents.len(), before);
    assert_eq!(f.count(&id), 2);
}
#[test]
fn ambiguous_families_and_other_directories_require_review_and_manual_associations_persist() {
    let mut f = Fixture::new(&[
        ("Design_Notes_v1.md", "first"),
        ("Design_Notes_2026-10-01_v2.md", "second"),
    ]);
    f.enable(true);
    let id = f.document("Design_Notes_v1.md");
    fs::write(f.root.join("Design_Notes_v3.md"), "third").unwrap();
    f.scan();
    let review = f.r.tracking_view(&f.project).unwrap().reviews.remove(0);
    assert_eq!(review.candidate_ids.len(), 2);
    assert!(f.root.join("Design_Notes_v1.md").exists());
    f.r.resolve_match(&f.project, &review.source_id, "assign", Some(id.clone()))
        .unwrap();
    assert_eq!(f.count(&id), 2);
    fs::write(f.root.join("Design_Notes_v4.md"), "fourth").unwrap();
    f.scan();
    assert!(f.r.tracking_view(&f.project).unwrap().reviews.is_empty());
    assert_eq!(f.count(&id), 3);
    fs::create_dir(f.root.join("other")).unwrap();
    fs::write(f.root.join("other/Design_Notes_v5.md"), "other workstream").unwrap();
    f.scan();
    let review = f.r.tracking_view(&f.project).unwrap().reviews.remove(0);
    f.r.resolve_match(&f.project, &review.source_id, "new", None)
        .unwrap();
    assert_eq!(f.count(&id), 3);
    assert_ne!(f.r.source(&review.source_id).unwrap().document_id, id);
}
#[test]
fn identical_content_copy_needs_review_but_rename_preserves_identity_without_new_version() {
    let mut f = Fixture::new(&[("Master_Context_v1.md", "same")]);
    f.enable(false);
    let id = f.document("Master_Context_v1.md");
    fs::rename(
        f.root.join("Master_Context_v1.md"),
        f.root.join("Renamed.md"),
    )
    .unwrap();
    f.scan();
    assert_eq!(f.document("Renamed.md"), id);
    assert_eq!(f.count(&id), 1);
    assert!(f.r.tracking_view(&f.project).unwrap().reviews.is_empty());
    fs::copy(f.root.join("Renamed.md"), f.root.join("Copy.md")).unwrap();
    f.scan();
    let review = f.r.tracking_view(&f.project).unwrap().reviews.remove(0);
    assert!(review.explanation.contains("Identical"));
    f.r.resolve_match(&f.project, &review.source_id, "assign", Some(id.clone()))
        .unwrap();
    assert_eq!(f.count(&id), 1);
    assert!(f.root.join("Renamed.md").exists());
    assert!(f.root.join("Copy.md").exists());
}
#[test]
fn tracking_without_archival_disable_reenable_and_unregistration_retain_history() {
    let mut f = Fixture::new(&[("Master_Context_v1.md", "one")]);
    f.enable(false);
    let id = f.document("Master_Context_v1.md");
    fs::write(f.root.join("Master_Context_v2.md"), "two").unwrap();
    f.scan();
    assert!(f.root.join("Master_Context_v1.md").exists());
    assert_eq!(f.count(&id), 2);
    f.scan();
    assert_eq!(
        f.r.tracking_view(&f.project)
            .unwrap()
            .families
            .iter()
            .find(|d| d.id == id)
            .unwrap()
            .active_filename
            .as_deref(),
        Some("Master_Context_v2.md")
    );
    f.r.configure_tracking(&f.project, false, false, vec![])
        .unwrap();
    fs::write(f.root.join("Master_Context_v2.md"), "three").unwrap();
    f.scan();
    assert_eq!(f.count(&id), 2);
    f.r.configure_tracking(&f.project, true, false, vec![])
        .unwrap();
    assert_eq!(f.count(&id), 3);
    let snapshot = f.r.versions(&id).unwrap()[0].snapshot_path.clone();
    f.r.remove(&f.project).unwrap();
    assert!(f.r.snapshot().unwrap().projects.is_empty());
    assert!(PathBuf::from(&snapshot).exists());
    let restored = f.r.add("Restored", f.root.to_str().unwrap()).unwrap();
    assert_eq!(restored, f.project);
    assert!(!f.r.tracking_flags(&restored).unwrap().0);
    assert_eq!(f.count(&id), 3);
}
#[test]
fn changed_old_source_and_destination_collisions_never_move_or_overwrite() {
    let mut f = Fixture::new(&[("Master_v1.md", "one")]);
    f.enable(true);
    let document = f.prepare_replacement("Master_v1.md", "Master_v2.md", "two");
    let (_, plan) = f.pending_archive();
    fs::write(f.root.join("Master_v1.md"), "unexpected new work").unwrap();
    let errors = f.r.recover_operations(&f.project, true).unwrap();
    assert!(!errors.is_empty());
    assert!(f.root.join("Master_v1.md").exists());
    assert!(!f.root.join(&plan.destination).exists());
    assert_eq!(f.count(&document), 2);
    fs::write(f.root.join("Master_v1.md"), "one").unwrap();
    let dest = f.root.join(&plan.destination);
    fs::create_dir_all(dest.parent().unwrap()).unwrap();
    fs::write(&dest, "unrelated existing archive").unwrap();
    assert!(!f.r.recover_operations(&f.project, true).unwrap().is_empty());
    assert_eq!(
        fs::read_to_string(&dest).unwrap(),
        "unrelated existing archive"
    );
    assert!(f.root.join("Master_v1.md").exists());
    fs::remove_file(&dest).unwrap();
    assert!(f.r.recover_operations(&f.project, true).unwrap().is_empty());
    assert!(!f.root.join("Master_v1.md").exists());
    assert_eq!(fs::read_to_string(dest).unwrap(), "one");
}
#[test]
fn capture_recovers_before_and_after_finalization_without_duplicate_versions() {
    for phase in 0..3 {
        let f = Fixture::new(&[("Master.md", "durable bytes")]);
        let sid = f.r.source_ids(&f.project).unwrap().remove(0);
        let source = f.r.source(&sid).unwrap();
        let root = PathBuf::from(f.r.path(&f.project).unwrap());
        let stage = storage::stage(&root, &source.relative, &f.r.store).unwrap();
        f.r.db
            .execute(
                "UPDATE projects SET tracking_enabled=1,tracking_initialized=1 WHERE id=?1",
                [&f.project],
            )
            .unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let vid = uuid::Uuid::new_v4().to_string();
        let plan = CapturePlan {
            project: f.project.clone(),
            document: source.document_id.clone(),
            source: sid,
            version: vid.clone(),
            number: 1,
            name: source.name,
            relative: source.relative,
            modified: stage.modified,
            size: stage.size,
            hash: stage.hash,
            temporary: stage.temporary,
            snapshot: format!("snapshots/{}/{}/{vid}.md", f.project, source.document_id),
            reason: "initial_tracking".into(),
            captured_at: models::now(),
            previous_version: None,
            previous_source: None,
            may_archive: false,
        };
        f.r.db.execute("INSERT INTO operations(id,project_id,document_id,dedupe_key,kind,payload,created_at,updated_at) VALUES(?1,?2,?3,?1,'capture',?4,?5,?5)",rusqlite::params![id,f.project,plan.document,serde_json::to_string(&plan).unwrap(),models::now()]).unwrap();
        if phase > 0 {
            storage::snapshot_finalize(
                &f.r.store,
                &plan.temporary,
                &plan.snapshot,
                &plan.hash,
                false,
            )
            .unwrap();
        }
        if phase == 2 {
            f.r.db
                .execute("UPDATE operations SET state='finalized' WHERE id=?1", [&id])
                .unwrap();
            storage::snapshot_seal(&f.r.store, &plan.temporary, &plan.snapshot).unwrap();
        }
        let db = f.r.store.join("registry.db");
        drop(f.r);
        let mut r = Registry::open(&db).unwrap();
        let errors = r.recover_operations(&f.project, true).unwrap();
        assert!(errors.is_empty(), "{errors:?}");
        assert!(r.recover_operations(&f.project, true).unwrap().is_empty());
        assert_eq!(r.versions(&plan.document).unwrap().len(), 1);
        assert_eq!(
            fs::read_to_string(r.store.join(&plan.snapshot)).unwrap(),
            "durable bytes"
        );
    }
}
#[test]
fn archive_recovers_after_source_rename_before_database_commit() {
    let mut f = Fixture::new(&[("Master_v1.md", "one")]);
    f.enable(true);
    let document = f.prepare_replacement("Master_v1.md", "Master_v2.md", "two");
    let (_, plan) = f.pending_archive();
    let root = PathBuf::from(f.r.path(&f.project).unwrap());
    let source = storage::checked(&root, "Master_v1.md", false).unwrap();
    let dest = storage::checked(&root, &plan.destination, true).unwrap();
    let old = storage::locked_read(&source.path, true).unwrap();
    storage::rename_verified(&old, &dest.path).unwrap();
    drop(old);
    assert!(f.r.recover_operations(&f.project, true).unwrap().is_empty());
    assert_eq!(
        f.r.versions(&document).unwrap()[1].archive_path.as_deref(),
        Some(dest.path.to_str().unwrap())
    );
    assert_eq!(fs::read_to_string(dest.path).unwrap(), "one");
    assert!(f.r.recover_operations(&f.project, true).unwrap().is_empty());
}
#[test]
fn preexisting_archive_contents_are_excluded_case_insensitively() {
    let mut f = Fixture::new(&[
        ("Master_v1.md", "one"),
        ("OLD CONTEXT FILES/unrelated.md", "user owned"),
        ("nested/Old Context Files/history.md", "old"),
    ]);
    assert_eq!(f.r.snapshot().unwrap().documents.len(), 1);
    f.enable(true);
    fs::write(f.root.join("Master_v2.md"), "two").unwrap();
    f.scan();
    assert_eq!(
        fs::read_to_string(f.root.join("OLD CONTEXT FILES/unrelated.md")).unwrap(),
        "user owned"
    );
    assert_eq!(f.r.snapshot().unwrap().documents.len(), 1);
}
#[cfg(windows)]
#[test]
fn locked_writes_are_retried_and_archive_junctions_are_refused() {
    use std::os::windows::fs::OpenOptionsExt;
    let mut f = Fixture::new(&[("Master_v1.md", "one")]);
    f.enable(true);
    let document = f.document("Master_v1.md");
    let writer = fs::OpenOptions::new()
        .write(true)
        .share_mode(1)
        .open(f.root.join("Master_v1.md"))
        .unwrap();
    assert!(f
        .r
        .reconcile(
            &f.project,
            "watcher",
            &HashSet::from([models::key("Master_v1.md")]),
            false
        )
        .unwrap());
    assert_eq!(f.count(&document), 1);
    drop(writer);
    let outside = f._temp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    let status = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(f.root.join("Old Context Files"))
        .arg(&outside)
        .status()
        .unwrap();
    assert!(status.success());
    fs::write(f.root.join("Master_v2.md"), "two").unwrap();
    f.scan();
    assert!(f.root.join("Master_v1.md").exists());
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
    assert!(!f.r.tracking_view(&f.project).unwrap().operations.is_empty());
    fs::remove_dir(f.root.join("Old Context Files")).unwrap();
}
#[cfg(windows)]
#[test]
fn native_watcher_captures_changes_stops_when_disabled_and_reconciles_after_restart() {
    use std::{
        sync::{Arc, Mutex},
        thread,
        time::{Duration, Instant},
    };
    fn wait_until(mut ready: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(15);
        while !ready() {
            assert!(
                Instant::now() < deadline,
                "Native watcher did not reconcile within its test deadline"
            );
            thread::sleep(Duration::from_millis(100));
        }
    }
    let mut f = Fixture::new(&[("Master.md", "one")]);
    f.enable(false);
    let document = f.document("Master.md");
    let root = f.root.clone();
    let project = f.project.clone();
    let db = f.r.store.join("registry.db");
    let shared = Arc::new(Mutex::new(f.r));
    let watcher = crate::monitor::Monitor::start(shared.clone(), |_| {}).unwrap();
    wait_until(|| {
        shared
            .lock()
            .unwrap()
            .tracking_view(&project)
            .unwrap()
            .watch_status
            == "watching"
    });
    fs::write(root.join("Master.md"), "two").unwrap();
    wait_until(|| shared.lock().unwrap().versions(&document).unwrap().len() == 2);
    fs::write(root.join("Master.md"), "two").unwrap();
    thread::sleep(Duration::from_secs(2));
    assert_eq!(shared.lock().unwrap().versions(&document).unwrap().len(), 2);
    shared
        .lock()
        .unwrap()
        .configure_tracking(&project, false, false, vec![])
        .unwrap();
    fs::write(root.join("Master.md"), "three while paused").unwrap();
    thread::sleep(Duration::from_secs(2));
    assert_eq!(shared.lock().unwrap().versions(&document).unwrap().len(), 2);
    shared
        .lock()
        .unwrap()
        .configure_tracking(&project, true, false, vec![])
        .unwrap();
    assert_eq!(shared.lock().unwrap().versions(&document).unwrap().len(), 3);
    watcher.shutdown();
    drop(shared);
    fs::write(root.join("Master.md"), "four while closed").unwrap();
    let shared = Arc::new(Mutex::new(Registry::open(&db).unwrap()));
    let watcher = crate::monitor::Monitor::start(shared.clone(), |_| {}).unwrap();
    wait_until(|| shared.lock().unwrap().versions(&document).unwrap().len() == 4);
    watcher.shutdown();
}
#[test]
fn archive_edits_do_not_change_snapshots_and_snapshot_collisions_are_non_destructive() {
    let mut f = Fixture::new(&[("説明_設計_v1.md", "original Ω")]);
    f.enable(true);
    let document = f.document("説明_設計_v1.md");
    fs::write(f.root.join("説明_設計_v1.md"), "new content").unwrap();
    f.scan();
    let versions = f.r.versions(&document).unwrap();
    let archive = versions[1].archive_path.as_ref().unwrap();
    fs::write(archive, "manually edited archive").unwrap();
    assert_eq!(
        fs::read_to_string(&versions[1].snapshot_path).unwrap(),
        "original Ω"
    );
    assert!(f.r.version_location(&versions[1].id, true).is_err());
    assert!(f.r.version_location(&versions[1].id, false).is_ok());
    f.scan();
    assert_eq!(f.count(&document), 2);
    let root = PathBuf::from(f.r.path(&f.project).unwrap());
    let stage = storage::stage(&root, "説明_設計_v1.md", &f.r.store).unwrap();
    let relative = PathBuf::from(&versions[1].snapshot_path)
        .strip_prefix(&f.r.store)
        .unwrap()
        .to_string_lossy()
        .to_string();
    assert!(storage::snapshot_finalize(
        &f.r.store,
        &stage.temporary,
        &relative,
        &stage.hash,
        false
    )
    .is_err());
    assert_eq!(
        fs::read_to_string(&versions[1].snapshot_path).unwrap(),
        "original Ω"
    );
}
