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
fn loss_persists_across_restart_and_returning_root_requires_explicit_reassignment() {
    let mut f = Fixture::new(false);
    let target = f.moved();
    assert_eq!(f.r.check_folders().unwrap(), vec![f.id.clone()]);
    assert_eq!(
        f.r.snapshot().unwrap().projects[0].folder_state,
        "needs_reassignment"
    );
    f.r = Registry::open(&f.db).unwrap();
    fs::rename(&target, &f.root).unwrap();
    f.r.refresh(&f.id).unwrap();
    assert!(folder_recovery::require_ready(&f.r.db, &f.id).is_err());
    let root = f.root.clone();
    f.reassign(&root);
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
