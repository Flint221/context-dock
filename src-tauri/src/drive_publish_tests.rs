use crate::{
    drive_api::{self, ApiError, DriveApi, Remote},
    drive_publish as publish,
    registry::Registry,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};
use tempfile::{tempdir, TempDir};

#[derive(Default)]
struct FakeDrive {
    next: Cell<usize>,
    files: RefCell<HashMap<String, (Remote, Vec<u8>)>>,
    uploads: Cell<usize>,
    fail_before: Cell<bool>,
    lose_after: Cell<bool>,
    corrupt: Cell<bool>,
    after_upload: RefCell<Option<Box<dyn FnOnce()>>>,
}
impl FakeDrive {
    fn missing() -> ApiError {
        ApiError {
            missing: true,
            ..ApiError::local("not found")
        }
    }
    fn offline() -> ApiError {
        ApiError {
            retry: true,
            ..ApiError::local("offline")
        }
    }
}
impl DriveApi for FakeDrive {
    fn generate_id(&self) -> drive_api::Result<String> {
        let next = self.next.get() + 1;
        self.next.set(next);
        Ok(format!("file-{next}"))
    }
    fn get(&self, id: &str) -> drive_api::Result<Remote> {
        self.files
            .borrow()
            .get(id)
            .map(|(r, _)| r.clone())
            .ok_or_else(Self::missing)
    }
    fn folders(&self, parent: &str, name: &str) -> drive_api::Result<Vec<Remote>> {
        Ok(self
            .files
            .borrow()
            .values()
            .filter(|(r, _)| {
                r.parents == [parent]
                    && r.name == name
                    && r.mime_type == drive_api::FOLDER
                    && !r.trashed
            })
            .map(|(r, _)| r.clone())
            .collect())
    }
    fn create_folder(
        &self,
        id: &str,
        parent: &str,
        name: &str,
        properties: &BTreeMap<String, String>,
    ) -> drive_api::Result<Remote> {
        let remote = Remote {
            id: id.into(),
            name: name.into(),
            mime_type: drive_api::FOLDER.into(),
            parents: vec![parent.into()],
            app_properties: properties.clone(),
            size: None,
            version: Some("1".into()),
            trashed: false,
            etag: Some("etag-1".into()),
        };
        self.files
            .borrow_mut()
            .insert(id.into(), (remote.clone(), vec![]));
        Ok(remote)
    }
    fn upload(
        &self,
        id: &str,
        metadata: Value,
        file: &Path,
        create: bool,
        _etag: Option<&str>,
    ) -> drive_api::Result<Remote> {
        self.uploads.set(self.uploads.get() + 1);
        if self.fail_before.replace(false) {
            return Err(Self::offline());
        }
        let previous = self.files.borrow().get(id).cloned();
        if create && previous.is_some() {
            return Err(ApiError::local("duplicate create"));
        }
        if !create && previous.is_none() {
            return Err(Self::missing());
        }
        let mut bytes = fs::read(file).unwrap();
        if self.corrupt.replace(false) && !bytes.is_empty() {
            bytes[0] ^= 1;
        }
        let revision = previous
            .as_ref()
            .and_then(|(r, _)| r.version.as_deref())
            .unwrap_or("0")
            .parse::<u64>()
            .unwrap()
            + 1;
        let remote = Remote {
            id: id.into(),
            name: metadata["name"].as_str().unwrap().into(),
            mime_type: metadata["mimeType"].as_str().unwrap().into(),
            parents: previous
                .as_ref()
                .map(|(r, _)| r.parents.clone())
                .unwrap_or_else(|| serde_json::from_value(metadata["parents"].clone()).unwrap()),
            app_properties: serde_json::from_value(metadata["appProperties"].clone()).unwrap(),
            size: Some(bytes.len().to_string()),
            version: Some(revision.to_string()),
            trashed: false,
            etag: Some(format!("etag-{revision}")),
        };
        self.files
            .borrow_mut()
            .insert(id.into(), (remote.clone(), bytes));
        if let Some(hook) = self.after_upload.borrow_mut().take() {
            hook();
        }
        if self.lose_after.replace(false) {
            return Err(Self::offline());
        }
        Ok(remote)
    }
    fn download_hash(&self, id: &str, size: i64) -> drive_api::Result<String> {
        let files = self.files.borrow();
        let (_, bytes) = files.get(id).ok_or_else(Self::missing)?;
        if bytes.len() as i64 != size {
            return Err(ApiError::local("size mismatch"));
        }
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }
}
struct Fixture {
    _temp: TempDir,
    root: PathBuf,
    db: PathBuf,
    registry: Registry,
    project: String,
    api: FakeDrive,
}
impl Fixture {
    fn new(names: &[(&str, &[u8])]) -> Self {
        let temp = tempdir().unwrap();
        let root = temp.path().join("Project");
        fs::create_dir(&root).unwrap();
        for (name, bytes) in names {
            fs::write(root.join(name), bytes).unwrap();
        }
        let app = temp.path().join("App");
        fs::create_dir(&app).unwrap();
        let db = app.join("registry.db");
        let mut registry = Registry::open(&db).unwrap();
        let project = registry
            .add("Sample Project", root.to_str().unwrap())
            .unwrap();
        registry
            .configure_tracking(&project, true, false, vec![])
            .unwrap();
        publish::configure(&registry.db, &project, "account-A", true, false).unwrap();
        registry
            .db
            .execute(
                "UPDATE drive_accounts SET create_root=1 WHERE account_id='account-A'",
                [],
            )
            .unwrap();
        Self {
            _temp: temp,
            root,
            db,
            registry,
            project,
            api: FakeDrive::default(),
        }
    }
    fn current(&self) -> String {
        publish::eligible_documents(&self.registry.db, &self.project).unwrap()[0].clone()
    }
    fn run(&self) -> drive_api::Result<()> {
        let parent =
            publish::ensure_destination(&self.registry.db, &self.project, "account-A", &self.api)?;
        publish::publish(
            &self.registry,
            &self.project,
            "account-A",
            &self.current(),
            &parent,
            &self.api,
        )
    }
    fn saved(&self) -> publish::Publication {
        publish::publications(&self.registry.db, &self.project)
            .unwrap()
            .remove(0)
    }
    fn reconcile(&mut self) {
        self.registry
            .reconcile(&self.project, "manual_refresh", &HashSet::new(), true)
            .unwrap();
    }
    fn cloud_documents(&self) -> usize {
        self.api
            .files
            .borrow()
            .values()
            .filter(|(r, _)| r.mime_type != drive_api::FOLDER)
            .count()
    }
}
#[test]
fn drive_publication_v1_v2_reuses_cloud_file_and_project_folder_ids() {
    let mut f = Fixture::new(&[("ContextDock_Context_v1.md", b"sample v1")]);
    f.run().unwrap();
    let v1 = f.saved();
    let folder: String = f
        .registry
        .db
        .query_row(
            "SELECT folder_id FROM drive_projects WHERE project_id=?1",
            [&f.project],
            |r| r.get(0),
        )
        .unwrap();
    fs::write(f.root.join("ContextDock_Context_v2.md"), b"different v2").unwrap();
    f.reconcile();
    f.run().unwrap();
    let v2 = f.saved();
    assert_eq!(v1.file_id, v2.file_id);
    assert_ne!(v1.verified_hash, v2.verified_hash);
    assert_eq!(
        v2.verified_name.as_deref(),
        Some("ContextDock_Context_v2.md")
    );
    assert_eq!(v2.state, "synced");
    assert_eq!(f.cloud_documents(), 1);
    assert_eq!(f.api.get(&v2.file_id).unwrap().parents, [folder]);
    assert!(f.root.join("ContextDock_Context_v1.md").exists());
}
#[test]
fn drive_publication_simultaneous_versions_publish_only_latest_family_head() {
    let f = Fixture::new(&[
        ("ContextDock_Context_v1.md", b"one"),
        ("ContextDock_Context_v2.md", b"two"),
    ]);
    assert_eq!(
        publish::eligible_documents(&f.registry.db, &f.project)
            .unwrap()
            .len(),
        1
    );
    f.run().unwrap();
    assert_eq!(
        f.saved().verified_name.as_deref(),
        Some("ContextDock_Context_v2.md")
    );
    assert_eq!(f.cloud_documents(), 1);
}
#[test]
fn drive_publication_lost_upload_response_recovers_same_id_after_restart() {
    let mut f = Fixture::new(&[("ContextDock_Context_v1.md", b"one")]);
    f.api.lose_after.set(true);
    assert!(f.run().is_err());
    let pending = f.saved();
    assert!(pending.pending_hash.is_some());
    assert!(pending.verified_hash.is_none());
    f.registry = Registry::open(&f.db).unwrap();
    f.run().unwrap();
    assert_eq!(f.saved().file_id, pending.file_id);
    assert_eq!(f.saved().state, "synced");
    assert_eq!(f.cloud_documents(), 1);
    assert_eq!(f.api.uploads.get(), 1);
}
#[test]
fn drive_publication_failure_preserves_old_verified_file_and_retry_updates_it() {
    let mut f = Fixture::new(&[("ContextDock_Context_v1.md", b"one")]);
    f.run().unwrap();
    let before = f.saved();
    fs::write(f.root.join("ContextDock_Context_v2.md"), b"two").unwrap();
    f.reconcile();
    f.api.fail_before.set(true);
    assert!(f.run().is_err());
    assert_eq!(f.saved().verified_hash, before.verified_hash);
    assert_eq!(
        f.api.get(&before.file_id).unwrap().name,
        "ContextDock_Context_v1.md"
    );
    f.run().unwrap();
    assert_eq!(f.saved().file_id, before.file_id);
    assert_eq!(
        f.saved().verified_name.as_deref(),
        Some("ContextDock_Context_v2.md")
    );
    assert_eq!(f.cloud_documents(), 1);
}
#[test]
fn drive_publication_corrupt_readback_never_marks_synced_or_changes_local_files() {
    let f = Fixture::new(&[("ContextDock_Context_v1.md", b"one")]);
    f.api.corrupt.set(true);
    assert!(f.run().unwrap_err().message.contains("did not match"));
    assert_ne!(f.saved().state, "synced");
    assert!(f.saved().verified_hash.is_none());
    assert_eq!(
        fs::read(f.root.join("ContextDock_Context_v1.md")).unwrap(),
        b"one"
    );
    f.run().unwrap();
    assert_eq!(f.saved().state, "synced");
    assert_eq!(f.cloud_documents(), 1);
}
#[test]
fn drive_publication_foreign_identity_and_external_content_edits_are_not_overwritten() {
    let mut f = Fixture::new(&[("ContextDock_Context_v1.md", b"one")]);
    f.run().unwrap();
    let before = f.saved();
    fs::write(f.root.join("ContextDock_Context_v2.md"), b"two").unwrap();
    f.reconcile();
    f.api
        .files
        .borrow_mut()
        .get_mut(&before.file_id)
        .unwrap()
        .0
        .app_properties
        .insert("document".into(), "foreign-document".into());
    assert!(f.run().unwrap_err().message.contains("different identity"));
    assert_eq!(f.api.uploads.get(), 1);
    f.api
        .files
        .borrow_mut()
        .get_mut(&before.file_id)
        .unwrap()
        .0
        .app_properties
        .insert("document".into(), before.document_id.clone());
    f.api.files.borrow_mut().get_mut(&before.file_id).unwrap().1 = b"external edit".to_vec();
    f.api
        .files
        .borrow_mut()
        .get_mut(&before.file_id)
        .unwrap()
        .0
        .size = Some("13".into());
    assert!(f.run().unwrap_err().message.contains("edited outside"));
    assert_eq!(f.api.uploads.get(), 1);
}
#[test]
fn drive_publication_missing_previously_verified_remote_is_not_duplicated() {
    let f = Fixture::new(&[("ContextDock_Context_v1.md", b"one")]);
    f.run().unwrap();
    let before = f.saved();
    f.api.files.borrow_mut().remove(&before.file_id);
    assert!(f.run().unwrap_err().missing);
    assert_eq!(f.saved().file_id, before.file_id);
    assert_eq!(f.api.uploads.get(), 1);
}
#[test]
fn drive_publication_new_local_save_during_upload_keeps_verified_receipt_but_is_not_synced() {
    let f = Fixture::new(&[("ContextDock_Context_v1.md", b"one")]);
    let source = f.root.join("ContextDock_Context_v1.md");
    *f.api.after_upload.borrow_mut() = Some(Box::new(move || {
        fs::write(source, b"new save not yet tracked").unwrap()
    }));
    f.run().unwrap();
    assert!(f.saved().verified_hash.is_some());
    assert_eq!(f.saved().state, "queued");
}
#[test]
fn drive_publication_scope_and_settings_are_verified_before_remote_mutation() {
    let f = Fixture::new(&[("ContextDock_Context_v1.md", b"one")]);
    assert!(publish::ensure_destination(&f.registry.db, &f.project, "account-B", &f.api).is_err());
    assert!(f.api.files.borrow().is_empty());
    publish::configure(&f.registry.db, &f.project, "account-A", false, false).unwrap();
    assert!(f.run().is_err());
    assert!(f.api.files.borrow().is_empty());
    f.registry.db.execute_batch("CREATE TRIGGER ignore_drive_settings BEFORE UPDATE ON drive_projects BEGIN SELECT RAISE(IGNORE); END;").unwrap();
    assert!(publish::configure(&f.registry.db, &f.project, "account-A", true, false).is_err());
    assert!(f.run().is_err());
}

#[test]
fn drive_publication_local_rename_and_nested_move_keep_document_and_cloud_identity() {
    let mut f = Fixture::new(&[("ContextDock_Context_v1.md", b"one")]);
    f.run().unwrap();
    let first = f.saved();
    fs::create_dir(f.root.join("Nested")).unwrap();
    fs::rename(
        f.root.join("ContextDock_Context_v1.md"),
        f.root.join("Nested/ContextDock_Renamed_v1.md"),
    )
    .unwrap();
    f.reconcile();
    f.run().unwrap();
    let next = f.saved();
    assert_eq!(first.document_id, next.document_id);
    assert_eq!(first.file_id, next.file_id);
    assert_eq!(
        next.verified_name.as_deref(),
        Some("ContextDock_Renamed_v1.md")
    );
    assert_eq!(f.cloud_documents(), 1);
}

#[test]
fn drive_publication_excluded_moved_source_cannot_update_the_cloud() {
    let mut f = Fixture::new(&[("ContextDock_Context_v1.md", b"one")]);
    f.run().unwrap();
    let first = f.saved();
    fs::create_dir(f.root.join("Excluded")).unwrap();
    f.registry
        .db
        .execute(
            "INSERT INTO path_exclusions(project_id,path,kind) VALUES(?1,'excluded','folder')",
            [&f.project],
        )
        .unwrap();
    fs::rename(
        f.root.join("ContextDock_Context_v1.md"),
        f.root.join("Excluded/ContextDock_Context_v1.md"),
    )
    .unwrap();
    f.reconcile();
    assert!(publish::eligible_documents(&f.registry.db, &f.project)
        .unwrap()
        .is_empty());
    let folder: String = f
        .registry
        .db
        .query_row(
            "SELECT folder_id FROM drive_projects WHERE project_id=?1",
            [&f.project],
            |r| r.get(0),
        )
        .unwrap();
    assert!(publish::publish(
        &f.registry,
        &f.project,
        "account-A",
        &first.document_id,
        &folder,
        &f.api
    )
    .is_err());
    assert_eq!(f.api.uploads.get(), 1);
    assert_eq!(
        f.api.get(&first.file_id).unwrap().name,
        "ContextDock_Context_v1.md"
    );
}

#[test]
fn drive_publication_unsaved_intent_prevents_upload_and_unsaved_receipt_is_not_synced() {
    let f = Fixture::new(&[("ContextDock_Context_v1.md", b"one")]);
    f.registry.db.execute_batch("CREATE TRIGGER ignore_pending BEFORE UPDATE OF pending_hash ON drive_documents BEGIN SELECT RAISE(IGNORE); END;").unwrap();
    assert!(f.run().unwrap_err().message.contains("intent did not save"));
    assert_eq!(f.api.uploads.get(), 0);
    f.registry.db.execute_batch("DROP TRIGGER ignore_pending; CREATE TRIGGER ignore_receipt BEFORE UPDATE OF verified_hash ON drive_documents BEGIN SELECT RAISE(IGNORE); END;").unwrap();
    assert!(f
        .run()
        .unwrap_err()
        .message
        .contains("receipt did not save"));
    assert_ne!(f.saved().state, "synced");
    assert!(f.saved().verified_hash.is_none());
}

#[test]
fn drive_publication_unsaved_folder_identity_prevents_remote_creation() {
    let f = Fixture::new(&[("ContextDock_Context_v1.md", b"one")]);
    f.registry.db.execute_batch("CREATE TRIGGER ignore_folder_id BEFORE UPDATE OF root_id ON drive_accounts BEGIN SELECT RAISE(IGNORE); END;").unwrap();
    assert!(f
        .run()
        .unwrap_err()
        .message
        .contains("identity was not saved"));
    assert!(f.api.files.borrow().is_empty());
}

#[test]
fn drive_publication_reuses_accessible_folders_and_refuses_ambiguous_names() {
    let f = Fixture::new(&[("ContextDock_Context_v1.md", b"one")]);
    f.api
        .create_folder("existing-root", "root", "ContextDock", &BTreeMap::new())
        .unwrap();
    f.api
        .create_folder(
            "existing-project",
            "existing-root",
            "Sample Project",
            &BTreeMap::new(),
        )
        .unwrap();
    f.run().unwrap();
    assert_eq!(
        f.api.get(&f.saved().file_id).unwrap().parents,
        ["existing-project"]
    );
    let other = Fixture::new(&[("ContextDock_Context_v1.md", b"one")]);
    other
        .api
        .create_folder("root-a", "root", "ContextDock", &BTreeMap::new())
        .unwrap();
    other
        .api
        .create_folder("root-b", "root", "ContextDock", &BTreeMap::new())
        .unwrap();
    assert!(other
        .run()
        .unwrap_err()
        .message
        .contains("Multiple matching"));
    assert_eq!(other.api.files.borrow().len(), 2);
}

#[test]
fn drive_publication_schema_thirteen_migration_keeps_ids_snapshots_and_cloud_off() {
    let f = Fixture::new(&[("ContextDock_Context_v1.md", b"one")]);
    let document = f.current();
    let hash: String = f
        .registry
        .db
        .query_row("SELECT sha256 FROM document_versions LIMIT 1", [], |r| {
            r.get(0)
        })
        .unwrap();
    f.registry.db.execute_batch("DROP TABLE drive_documents; DROP TABLE drive_projects; DROP TABLE drive_accounts; PRAGMA user_version=13;").unwrap();
    let r = Registry::open(&f.db).unwrap();
    assert_eq!(
        publish::eligible_documents(&r.db, &f.project).unwrap(),
        [document]
    );
    assert_eq!(
        r.db.query_row("SELECT sha256 FROM document_versions LIMIT 1", [], |r| {
            r.get::<_, String>(0)
        })
        .unwrap(),
        hash
    );
    assert_eq!(
        r.db.query_row("SELECT COUNT(*) FROM drive_projects", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        r.db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        14
    );
}

#[test]
fn drive_publication_unseen_root_requires_explicit_creation_or_picker_adoption() {
    let f = Fixture::new(&[("ContextDock_Context_v1.md", b"one")]);
    f.registry
        .db
        .execute("UPDATE drive_accounts SET create_root=0", [])
        .unwrap();
    assert!(f
        .run()
        .unwrap_err()
        .message
        .contains("existing ContextDock folder"));
    assert!(f.api.files.borrow().is_empty());
    f.api
        .create_folder("selected-root", "root", "ContextDock", &BTreeMap::new())
        .unwrap();
    publish::adopt_root(&f.registry.db, "account-A", "selected-root", &f.api).unwrap();
    f.run().unwrap();
    let mapped: String = f
        .registry
        .db
        .query_row(
            "SELECT root_id FROM drive_accounts WHERE account_id='account-A'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(mapped, "selected-root");
    f.api
        .create_folder("other-root", "root", "ContextDock", &BTreeMap::new())
        .unwrap();
    assert!(publish::adopt_root(&f.registry.db, "account-A", "other-root", &f.api).is_err());
}
