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

pub(crate) fn sample_docx(marker: &str) -> Vec<u8> {
    use std::io::{Cursor, Write};
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, text) in [
        ("[Content_Types].xml", r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#.to_owned()),
        ("_rels/.rels", r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#.to_owned()),
        ("word/document.xml",format!(r#"<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>{marker}</w:t></w:r></w:p><w:sectPr/></w:body></w:document>"#))
    ] {
        zip.start_file(name,zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(text.as_bytes()).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

struct FakeBin {
    root: PathBuf,
    calls: Cell<usize>,
    unsupported: bool,
    fail: bool,
    crash: bool,
    before: RefCell<Option<Box<dyn FnOnce()>>>,
}
impl FakeBin {
    fn new(f: &Fixture) -> Self {
        let root = f.root.parent().unwrap().join("Bin");
        fs::create_dir(&root).unwrap();
        Self {
            root,
            calls: Cell::new(0),
            unsupported: false,
            fail: false,
            crash: false,
            before: RefCell::new(None),
        }
    }
}
impl crate::recycle_bin::Bin for FakeBin {
    fn volume(&self, path: &Path) -> Result<crate::recycle_bin::Volume, String> {
        if self.unsupported {
            return Err("This test volume cannot recycle.".into());
        }
        crate::recycle_bin::NativeBin.volume(path)
    }
    fn recycle(&self, path: &Path, hash: &str) -> Result<String, String> {
        self.calls.set(self.calls.get() + 1);
        if let Some(before) = self.before.borrow_mut().take() {
            before();
        }
        assert!(
            !self.crash,
            "simulated process interruption before bin transfer"
        );
        if self.fail {
            return Err("Bin transfer failed.".into());
        }
        crate::storage::verify(path, hash)?;
        let destination = self.root.join(format!(
            "{}-{}",
            uuid::Uuid::new_v4(),
            path.file_name().unwrap().to_string_lossy()
        ));
        fs::rename(path, &destination).map_err(|e| e.to_string())?;
        Ok(destination.to_string_lossy().into())
    }
}
fn enable_recycling(f: &Fixture) {
    crate::drive_recycle::configure(&f.registry.db, &f.project, true).unwrap();
    f.run().unwrap();
}
fn cleanup(f: &Fixture, bin: &dyn crate::recycle_bin::Bin) -> Result<usize, String> {
    let parent: String = f
        .registry
        .db
        .query_row(
            "SELECT folder_id FROM drive_projects WHERE project_id=?1",
            [&f.project],
            |r| r.get(0),
        )
        .unwrap();
    crate::drive_recycle::run(
        &f.registry,
        &f.project,
        "account-A",
        &f.current(),
        &parent,
        &f.api,
        bin,
    )
}
#[test]
fn drive_recycling_is_opt_in_and_keeps_the_newest_local_file() {
    let f = Fixture::new(&[
        ("ContextDock_Context_v1.md", b"one"),
        ("ContextDock_Context_v2.md", b"two"),
    ]);
    f.run().unwrap();
    let bin = FakeBin::new(&f);
    assert_eq!(cleanup(&f, &bin).unwrap(), 0);
    assert_eq!(bin.calls.get(), 0);
    enable_recycling(&f);
    assert_eq!(cleanup(&f, &bin).unwrap(), 1);
    assert!(!f.root.join("ContextDock_Context_v1.md").exists());
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_HIDDEN;

        let staging = fs::metadata(f.root.join(".contextdock")).unwrap();
        assert_ne!(staging.file_attributes() & FILE_ATTRIBUTE_HIDDEN, 0);
        let project = fs::metadata(&f.root).unwrap();
        assert_eq!(project.file_attributes() & FILE_ATTRIBUTE_HIDDEN, 0);
    }
    assert_eq!(
        fs::read_to_string(f.root.join("ContextDock_Context_v2.md")).unwrap(),
        "two"
    );
    assert_eq!(f.saved().state, "synced");
    assert_eq!(
        f.registry
            .db
            .query_row("SELECT state FROM drive_recycle", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        "recycled"
    );
    assert_eq!(cleanup(&f, &bin).unwrap(), 0);
    assert_eq!(bin.calls.get(), 1);
}
#[test]
fn drive_recycling_requires_a_fresh_matching_cloud_readback() {
    let f = Fixture::new(&[
        ("ContextDock_Context_v1.md", b"one"),
        ("ContextDock_Context_v2.md", b"two"),
    ]);
    f.run().unwrap();
    enable_recycling(&f);
    let bin = FakeBin::new(&f);
    f.api
        .files
        .borrow_mut()
        .get_mut(&f.saved().file_id)
        .unwrap()
        .1 = b"bad".to_vec();
    assert!(cleanup(&f, &bin).is_err());
    assert_eq!(bin.calls.get(), 0);
    assert!(f.root.join("ContextDock_Context_v1.md").exists());
    assert_eq!(
        f.registry
            .db
            .query_row("SELECT COUNT(*) FROM drive_recycle", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
#[test]
fn drive_recycling_keeps_changed_older_files_and_unsupported_volumes() {
    let f = Fixture::new(&[
        ("ContextDock_Context_v1.md", b"one"),
        ("ContextDock_Context_v2.md", b"two"),
    ]);
    f.run().unwrap();
    enable_recycling(&f);
    let mut bin = FakeBin::new(&f);
    bin.unsupported = true;
    assert!(cleanup(&f, &bin).is_err());
    assert!(f.root.join("ContextDock_Context_v1.md").exists());
    bin.unsupported = false;
    fs::write(f.root.join("ContextDock_Context_v1.md"), b"new").unwrap();
    assert!(cleanup(&f, &bin).unwrap_err().contains("changed"));
    assert_eq!(bin.calls.get(), 0);
    assert_eq!(
        fs::read(f.root.join("ContextDock_Context_v1.md")).unwrap(),
        b"new"
    );
}
#[test]
fn drive_recycling_failed_bin_transfer_restores_the_original_without_overwrite() {
    let f = Fixture::new(&[
        ("ContextDock_Context_v1.md", b"one"),
        ("ContextDock_Context_v2.md", b"two"),
    ]);
    f.run().unwrap();
    enable_recycling(&f);
    let mut bin = FakeBin::new(&f);
    bin.fail = true;
    assert!(cleanup(&f, &bin).is_err());
    assert_eq!(
        fs::read(f.root.join("ContextDock_Context_v1.md")).unwrap(),
        b"one"
    );
    assert_eq!(
        f.registry
            .db
            .query_row("SELECT state FROM drive_recycle", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        "restored"
    );
    assert_eq!(f.saved().state, "synced");
}
#[test]
fn drive_recycling_restart_restores_a_journaled_safety_file() {
    let mut f = Fixture::new(&[
        ("ContextDock_Context_v1.md", b"one"),
        ("ContextDock_Context_v2.md", b"two"),
    ]);
    f.run().unwrap();
    enable_recycling(&f);
    let mut bin = FakeBin::new(&f);
    bin.crash = true;
    let older = f.root.join("ContextDock_Context_v1.md");
    let exfat = matches!(
        crate::recycle_bin::Bin::volume(&bin, &older).unwrap(),
        crate::recycle_bin::Volume::Exfat
    );
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cleanup(&f, &bin))).is_err());
    assert!(!older.exists());
    let recovery_identity: String = f
        .registry
        .db
        .query_row("SELECT file_identity FROM drive_recycle", [], |r| r.get(0))
        .unwrap();
    assert_eq!(recovery_identity.starts_with("exfat:"), exfat);
    f.registry = Registry::open(&f.db).unwrap();
    assert_eq!(fs::read(older).unwrap(), b"one");
    assert!(f.root.join("ContextDock_Context_v2.md").exists());
    assert_eq!(
        f.registry
            .db
            .query_row("SELECT state FROM drive_recycle", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        "restored"
    );
}
#[test]
fn drive_recycling_recovery_retains_safety_bytes_when_original_path_is_reoccupied() {
    let f = Fixture::new(&[
        ("ContextDock_Context_v1.md", b"one"),
        ("ContextDock_Context_v2.md", b"two"),
    ]);
    f.run().unwrap();
    enable_recycling(&f);
    let mut bin = FakeBin::new(&f);
    bin.fail = true;
    let original = f.root.join("ContextDock_Context_v1.md");
    let replacement = original.clone();
    *bin.before.borrow_mut() = Some(Box::new(move || {
        fs::write(replacement, b"replacement").unwrap()
    }));
    assert!(cleanup(&f, &bin).unwrap_err().contains("occupied"));
    assert_eq!(fs::read(original).unwrap(), b"replacement");
    let relative: String = f
        .registry
        .db
        .query_row("SELECT safety_relative FROM drive_recycle", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(fs::read(f.root.join(relative)).unwrap(), b"one");
    assert_eq!(f.saved().state, "synced");
    crate::drive_recycle::configure(&f.registry.db, &f.project, false).unwrap();
    assert!(crate::drive_recycle::recover_project(&f.registry, &f.project).is_err());
    fs::rename(
        f.root.join("ContextDock_Context_v1.md"),
        f.root.join("ContextDock_Preserved_Replacement_v1.md"),
    )
    .unwrap();
    crate::drive_recycle::recover_project(&f.registry, &f.project).unwrap();
    assert_eq!(
        fs::read(f.root.join("ContextDock_Context_v1.md")).unwrap(),
        b"one"
    );
    assert_eq!(
        fs::read(f.root.join("ContextDock_Preserved_Replacement_v1.md")).unwrap(),
        b"replacement"
    );
    assert_eq!(f.saved().state, "synced");
}
#[test]
fn drive_recycling_opt_out_after_quarantine_restores_file_before_bin_transfer() {
    let f = Fixture::new(&[
        ("ContextDock_Context_v1.md", b"one"),
        ("ContextDock_Context_v2.md", b"two"),
    ]);
    f.run().unwrap();
    enable_recycling(&f);
    let bin = FakeBin::new(&f);
    let db = f.db.clone();
    let project = f.project.clone();
    let mut checks = 0;
    *f.api.checkpoint.borrow_mut() = Some(Box::new(move || {
        checks += 1;
        if checks == 4 {
            let db = crate::cloud::connect(&db).unwrap();
            crate::drive_recycle::configure(&db, &project, false).unwrap();
        }
        Ok(())
    }));
    assert!(cleanup(&f, &bin).unwrap_err().contains("paused"));
    assert_eq!(bin.calls.get(), 0);
    assert!(f.root.join("ContextDock_Context_v1.md").exists());
}
#[test]
fn drive_recycling_keeps_unversioned_files_and_a_newest_file_changed_after_upload() {
    let f = Fixture::new(&[
        ("ContextDock_Context_v1.md", b"one"),
        ("ContextDock_Context_v2.md", b"two"),
    ]);
    f.run().unwrap();
    enable_recycling(&f);
    let bin = FakeBin::new(&f);
    fs::write(f.root.join("ContextDock_Context_v2.md"), b"newest edit").unwrap();
    assert!(cleanup(&f, &bin).is_err());
    assert_eq!(bin.calls.get(), 0);
    assert!(f.root.join("ContextDock_Context_v1.md").exists());
    let unversioned = Fixture::new(&[("ContextDock_Context.md", b"working")]);
    unversioned.run().unwrap();
    enable_recycling(&unversioned);
    let bin2 = FakeBin::new(&unversioned);
    assert_eq!(cleanup(&unversioned, &bin2).unwrap(), 0);
    assert!(unversioned.root.join("ContextDock_Context.md").exists());
}

#[test]
#[cfg(windows)]
#[ignore = "Explicit end-to-end Windows cleanup pilot with simulated Drive and disposable source files"]
fn drive_recycling_native_bin_keeps_latest_and_records_verified_recycle_receipt() {
    let f = Fixture::new(&[
        (
            "ContextDock_Recycling_Pilot_v1.md",
            b"disposable older marker",
        ),
        (
            "ContextDock_Recycling_Pilot_v2.md",
            b"disposable newest marker",
        ),
    ]);
    f.run().unwrap();
    enable_recycling(&f);
    assert_eq!(cleanup(&f, &crate::recycle_bin::NativeBin).unwrap(), 1);
    assert!(!f.root.join("ContextDock_Recycling_Pilot_v1.md").exists());
    assert_eq!(
        fs::read(f.root.join("ContextDock_Recycling_Pilot_v2.md")).unwrap(),
        b"disposable newest marker"
    );
    let (state, receipt): (String, String) = f
        .registry
        .db
        .query_row("SELECT state,bin_receipt FROM drive_recycle", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!(state, "recycled");
    assert!(!receipt.is_empty());
    assert_eq!(f.saved().state, "synced");
}

#[test]
fn drive_pilot_word_and_markdown_survive_versions_offline_renames_and_restart() {
    let word1 = sample_docx("Word pilot one");
    let word2 = sample_docx("Word pilot two");
    assert!(crate::extraction::extract(&word2, "word")
        .unwrap()
        .text
        .contains("Word pilot two"));
    let mut f = Fixture::new(&[
        ("ContextDock_Context_v1.md", b"Markdown one"),
        ("ContextDock_Context_v2.md", b"Markdown two"),
        ("ContextDock_Notes_v1.docx", &word1),
        ("ContextDock_Notes_v2.docx", &word2),
    ]);
    let docs = publish::eligible_documents(&f.registry.db, &f.project).unwrap();
    assert_eq!(docs.len(), 2);
    let parent =
        publish::ensure_destination(&f.registry.db, &f.project, "account-A", &f.api).unwrap();
    let publish_all = |r: &Registry| {
        for doc in publish::eligible_documents(&r.db, &f.project).unwrap() {
            publish::publish(r, &f.project, "account-A", &doc, &parent, &f.api).unwrap();
        }
    };
    publish_all(&f.registry);
    let before = publish::publications(&f.registry.db, &f.project).unwrap();
    fs::create_dir(f.root.join("Moved")).unwrap();
    fs::rename(
        f.root.join("ContextDock_Context_v2.md"),
        f.root.join("Moved/ContextDock_Context_v2.md"),
    )
    .unwrap();
    fs::rename(
        f.root.join("ContextDock_Notes_v2.docx"),
        f.root.join("ContextDock_Technical_Notes_v2.docx"),
    )
    .unwrap();
    f.reconcile();
    let current = publish::eligible_documents(&f.registry.db, &f.project).unwrap();
    assert_eq!(current.len(), 2);
    let word = current
        .iter()
        .find(|doc| {
            publish::current(&f.registry.db, &f.project, doc)
                .unwrap()
                .name
                .ends_with(".docx")
        })
        .unwrap();
    let word3 = sample_docx("Word pilot three");
    fs::write(f.root.join("ContextDock_Technical_Notes_v3.docx"), &word3).unwrap();
    f.reconcile();
    f.api.fail_before.set(true);
    assert!(
        publish::publish(&f.registry, &f.project, "account-A", word, &parent, &f.api)
            .unwrap_err()
            .retry
    );
    let pending = publish::publications(&f.registry.db, &f.project).unwrap();
    assert!(pending.iter().any(|d| d.pending_hash.is_some()));
    f.registry = Registry::open(&f.db).unwrap();
    for doc in publish::eligible_documents(&f.registry.db, &f.project).unwrap() {
        publish::publish(&f.registry, &f.project, "account-A", &doc, &parent, &f.api).unwrap();
    }
    let after = publish::publications(&f.registry.db, &f.project).unwrap();
    assert_eq!(after.len(), 2);
    assert_eq!(f.cloud_documents(), 2);
    for old in before {
        let new = after
            .iter()
            .find(|d| d.document_id == old.document_id)
            .unwrap();
        assert_eq!(old.file_id, new.file_id);
        assert_eq!(new.state, "synced");
        assert_eq!(
            f.api
                .download_hash(&new.file_id, new.verified_size.unwrap())
                .unwrap(),
            new.verified_hash.as_deref().unwrap()
        );
    }
    assert!(f.root.join("ContextDock_Notes_v1.docx").exists());
    assert!(f.root.join("ContextDock_Technical_Notes_v3.docx").exists());
    assert!(!f.registry.store.join("snapshots").exists());
}

type Checkpoint = Box<dyn FnMut() -> drive_api::Result<()>>;
#[derive(Default)]
struct FakeDrive {
    next: Cell<usize>,
    files: RefCell<HashMap<String, (Remote, Vec<u8>)>>,
    uploads: Cell<usize>,
    fail_before: Cell<bool>,
    lose_after: Cell<bool>,
    corrupt: Cell<bool>,
    after_upload: RefCell<Option<Box<dyn FnOnce()>>>,
    checkpoint: RefCell<Option<Checkpoint>>,
    hidden_folders: RefCell<HashSet<String>>,
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
    fn checkpoint(&self, _write: bool) -> drive_api::Result<()> {
        if let Some(check) = self.checkpoint.borrow_mut().as_mut() {
            check()?;
        }
        Ok(())
    }
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
                    && !self.hidden_folders.borrow().contains(&r.id)
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

#[test]
fn drive_folder_choice_prevents_duplicates_when_scope_hides_an_existing_project_folder() {
    let f = Fixture::new(&[("ContextDock_Context_v1.md", b"one")]);
    f.registry
        .db
        .execute("DELETE FROM drive_folder_choices", [])
        .unwrap();
    assert!(
        publish::ensure_destination(&f.registry.db, &f.project, "account-A", &f.api)
            .unwrap_err()
            .message
            .contains("existing Drive project folder")
    );
    let root: String = f
        .registry
        .db
        .query_row("SELECT root_id FROM drive_accounts", [], |r| r.get(0))
        .unwrap();
    f.api
        .create_folder(
            "preexisting-project",
            &root,
            "Sample Project",
            &BTreeMap::new(),
        )
        .unwrap();
    f.api
        .hidden_folders
        .borrow_mut()
        .insert("preexisting-project".into());
    assert!(publish::ensure_destination(&f.registry.db, &f.project, "account-A", &f.api).is_err());
    assert_eq!(
        f.api
            .files
            .borrow()
            .values()
            .filter(|(r, _)| r.name == "Sample Project")
            .count(),
        1
    );
    publish::adopt_project(
        &f.registry.db,
        &f.project,
        "account-A",
        "preexisting-project",
        &f.api,
    )
    .unwrap();
    f.run().unwrap();
    assert_eq!(
        f.api.get(&f.saved().file_id).unwrap().parents,
        ["preexisting-project"]
    );
    assert_eq!(
        f.api
            .files
            .borrow()
            .values()
            .filter(|(r, _)| r.name == "Sample Project")
            .count(),
        1
    );
    let other = f
        .api
        .create_folder(
            "another-project-folder",
            &root,
            "Sample Project",
            &BTreeMap::new(),
        )
        .unwrap();
    assert!(
        publish::adopt_project(&f.registry.db, &f.project, "account-A", &other.id, &f.api)
            .unwrap_err()
            .message
            .contains("mapped cloud files")
    );
}
struct Fixture {
    root: PathBuf,
    db: PathBuf,
    registry: Registry,
    project: String,
    api: FakeDrive,
    // SQLite handles must close before Windows removes the temporary directory.
    _temp: TempDir,
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
                "INSERT INTO drive_folder_choices(project_id,may_create) VALUES(?1,1)",
                [&project],
            )
            .unwrap();
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
fn drive_fixture_drop_removes_database_and_temporary_directory() {
    let fixture = Fixture::new(&[("ContextDock_Context_v1.md", b"cleanup marker")]);
    let directory = fixture._temp.path().to_path_buf();
    assert!(fixture.db.is_file());
    drop(fixture);
    assert!(
        !directory.exists(),
        "fixture must close its SQLite handles before removing its temporary directory"
    );
}

#[test]
fn drive_pending_publication_defers_old_version_path_retirement() {
    let mut f = Fixture::new(&[
        ("ContextDock_Context_v1.md", b"same"),
        ("ContextDock_Context_v2.md", b"same"),
    ]);
    f.api.fail_before.set(true);
    assert!(f.run().is_err());
    fs::remove_file(f.root.join("ContextDock_Context_v1.md")).unwrap();
    f.reconcile();
    assert!(f
        .registry
        .snapshot()
        .unwrap()
        .documents
        .iter()
        .any(|d| d.name.ends_with("v1.md") && d.status == "missing"));
    let before = f.saved();
    f.api.fail_before.set(false);
    f.run().unwrap();
    f.reconcile();
    assert!(f
        .registry
        .snapshot()
        .unwrap()
        .documents
        .iter()
        .all(|d| d.status != "missing"));
    assert_eq!(f.saved().file_id, before.file_id);
}

#[test]
fn drive_version_rename_retires_missing_path_without_replacing_cloud_identity() {
    let mut f = Fixture::new(&[("ContextDock_Context_v1.md", b"rename marker")]);
    f.run().unwrap();
    let before = f.saved();
    fs::rename(
        f.root.join("ContextDock_Context_v1.md"),
        f.root.join("ContextDock_Context_v2.md"),
    )
    .unwrap();
    f.reconcile();
    f.run().unwrap();
    let after = f.saved();
    assert_eq!(before.file_id, after.file_id);
    assert_eq!(before.document_id, after.document_id);
    assert_eq!(
        after.verified_name.as_deref(),
        Some("ContextDock_Context_v2.md")
    );
    assert_eq!(f.cloud_documents(), 1);
    assert!(f
        .registry
        .snapshot()
        .unwrap()
        .documents
        .iter()
        .all(|d| d.status != "missing"));
}

#[test]
fn drive_worker_reopened_storage_publishes_and_replaces_under_the_same_id() {
    let mut f = Fixture::new(&[("ContextDock_Context_v1.md", b"worker v1")]);
    let registry = crate::drive_publication_commands::open(&f.db).unwrap();
    let parent =
        publish::ensure_destination(&registry.db, &f.project, "account-A", &f.api).unwrap();
    publish::publish(
        &registry,
        &f.project,
        "account-A",
        &f.current(),
        &parent,
        &f.api,
    )
    .unwrap();
    let first = f.saved();
    fs::write(f.root.join("ContextDock_Context_v2.md"), b"worker v2").unwrap();
    f.reconcile();
    let reopened = crate::drive_publication_commands::open(&f.db).unwrap();
    publish::publish(
        &reopened,
        &f.project,
        "account-A",
        &f.current(),
        &parent,
        &f.api,
    )
    .unwrap();
    let second = f.saved();
    assert_eq!(first.file_id, second.file_id);
    assert_eq!(
        second.verified_name.as_deref(),
        Some("ContextDock_Context_v2.md")
    );
    assert_eq!(
        second.verified_hash,
        Some(format!("{:x}", Sha256::digest(b"worker v2")))
    );
    assert_eq!(second.state, "synced");
    assert_eq!(f.cloud_documents(), 1);
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
        18
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

#[test]
fn drive_scheduler_debounces_changes_without_hashing_or_sending_requests() {
    let f = Fixture::new(&[("ContextDock_Context_v1.md", b"one")]);
    f.registry
        .db
        .execute("UPDATE drive_projects SET automatic=1", [])
        .unwrap();
    assert!(crate::drive_scheduler::due(&f.registry, 1_000_000)
        .unwrap()
        .is_empty());
    assert!(crate::drive_scheduler::due(&f.registry, 1_002_999)
        .unwrap()
        .is_empty());
    let due = crate::drive_scheduler::due(&f.registry, 1_003_000).unwrap();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].documents, [f.current()]);
    assert!(f.api.files.borrow().is_empty());
    assert_eq!(f.api.uploads.get(), 0);
}

#[test]
fn drive_scheduler_unchanged_verified_files_stay_idle_across_repeated_checks() {
    let f = Fixture::new(&[("ContextDock_Context_v1.md", b"one")]);
    f.run().unwrap();
    f.registry
        .db
        .execute("UPDATE drive_projects SET automatic=1", [])
        .unwrap();
    for n in 0..100 {
        assert!(
            crate::drive_scheduler::due(&f.registry, 1_000_000 + n * 5_000)
                .unwrap()
                .is_empty()
        );
    }
    assert_eq!(f.api.uploads.get(), 1);
    assert_eq!(
        fs::read_dir(f.registry.store.join("staging"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn drive_scheduler_offline_backoff_and_pending_identity_survive_restart() {
    let mut f = Fixture::new(&[("ContextDock_Context_v1.md", b"one")]);
    f.registry
        .db
        .execute("UPDATE drive_projects SET automatic=1", [])
        .unwrap();
    crate::drive_scheduler::due(&f.registry, 1_000_000).unwrap();
    f.api.fail_before.set(true);
    let failure = f.run().unwrap_err();
    let pending = f.saved();
    crate::drive_scheduler::retry_error(
        &f.registry,
        &f.project,
        Some(&pending.document_id),
        &failure,
        1003,
    )
    .unwrap();
    f.registry = Registry::open(&f.db).unwrap();
    assert!(crate::drive_scheduler::due(&f.registry, 1_007_999)
        .unwrap()
        .is_empty());
    assert_eq!(
        crate::drive_scheduler::due(&f.registry, 1_008_000).unwrap()[0].documents,
        std::slice::from_ref(&pending.document_id)
    );
    assert_eq!(f.saved().file_id, pending.file_id);
    f.run().unwrap();
    assert_eq!(f.cloud_documents(), 1);
}

#[test]
fn drive_scheduler_manual_projects_only_retry_requested_work_and_pause_survives_restart() {
    let mut f = Fixture::new(&[("ContextDock_Context_v1.md", b"one")]);
    assert!(crate::drive_scheduler::due(&f.registry, 1_000_000)
        .unwrap()
        .is_empty());
    f.api.fail_before.set(true);
    let failure = f.run().unwrap_err();
    let pending = f.saved();
    crate::drive_scheduler::retry_error(
        &f.registry,
        &f.project,
        Some(&pending.document_id),
        &failure,
        1000,
    )
    .unwrap();
    crate::drive_scheduler::due(&f.registry, 1_005_000).unwrap();
    assert_eq!(
        crate::drive_scheduler::due(&f.registry, 1_008_000)
            .unwrap()
            .len(),
        1
    );
    publish::configure(&f.registry.db, &f.project, "account-A", false, false).unwrap();
    f.registry = Registry::open(&f.db).unwrap();
    assert!(crate::drive_scheduler::due(&f.registry, 9_000_000)
        .unwrap()
        .is_empty());
}

#[test]
fn drive_scheduler_auth_failure_waits_for_explicit_recovery() {
    let f = Fixture::new(&[("ContextDock_Context_v1.md", b"one")]);
    f.registry
        .db
        .execute("UPDATE drive_projects SET automatic=1", [])
        .unwrap();
    f.api.fail_before.set(true);
    assert!(f.run().is_err());
    let pending = f.saved();
    let failure = ApiError {
        auth: true,
        ..ApiError::local("grant revoked")
    };
    crate::drive_scheduler::retry_error(
        &f.registry,
        &f.project,
        Some(&pending.document_id),
        &failure,
        1000,
    )
    .unwrap();
    assert!(crate::drive_scheduler::due(&f.registry, 9_000_000)
        .unwrap()
        .is_empty());
    crate::drive_scheduler::reset(&f.registry, &f.project).unwrap();
    crate::drive_scheduler::due(&f.registry, 9_000_000).unwrap();
    assert_eq!(
        crate::drive_scheduler::due(&f.registry, 9_003_000)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn drive_scheduler_a_conflicted_document_does_not_block_an_unrelated_family() {
    let f = Fixture::new(&[
        ("ContextDock_A_v1.md", b"one"),
        ("ContextDock_B_v1.md", b"two"),
    ]);
    f.run().unwrap();
    let first = f.saved();
    f.registry
        .db
        .execute("UPDATE drive_projects SET automatic=1", [])
        .unwrap();
    crate::drive_scheduler::retry_error(
        &f.registry,
        &f.project,
        Some(&first.document_id),
        &ApiError::local("cloud conflict"),
        1000,
    )
    .unwrap();
    crate::drive_scheduler::due(&f.registry, 1_000_000).unwrap();
    let ready = crate::drive_scheduler::due(&f.registry, 1_003_000).unwrap();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].documents.len(), 1);
    assert_ne!(ready[0].documents[0], first.document_id);
}

#[test]
fn drive_scheduler_schema_fourteen_upgrade_preserves_the_pending_operation() {
    let f = Fixture::new(&[("ContextDock_Context_v1.md", b"one")]);
    f.api.fail_before.set(true);
    assert!(f.run().is_err());
    let before = f.saved();
    f.registry
        .db
        .execute_batch("DROP TABLE drive_schedule; PRAGMA user_version=14;")
        .unwrap();
    let registry = Registry::open(&f.db).unwrap();
    let after = publish::publications(&registry.db, &f.project)
        .unwrap()
        .remove(0);
    assert_eq!(before.file_id, after.file_id);
    assert_eq!(before.pending_hash, after.pending_hash);
    assert_eq!(before.pending_generation, after.pending_generation);
    assert_eq!(
        registry
            .db
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        18
    );
}

#[test]
fn drive_scheduler_honors_provider_backoff_and_distinguishes_editor_waits_from_offline() {
    let f = Fixture::new(&[("ContextDock_Context_v1.md", b"one")]);
    f.api.fail_before.set(true);
    assert!(f.run().is_err());
    let before = f.saved();
    let failure = ApiError {
        retry: true,
        delay: 120,
        ..ApiError::local("rate limited")
    };
    crate::drive_scheduler::retry_error(
        &f.registry,
        &f.project,
        Some(&before.document_id),
        &failure,
        1000,
    )
    .unwrap();
    assert_eq!(f.saved().next_at, 1120);
    assert_eq!(f.saved().state, "offline");
    let waiting = ApiError::from(
        "Filesystem operation could not complete: access denied (os error 32)".to_string(),
    );
    assert!(waiting.retry && waiting.waiting);
    crate::drive_scheduler::retry_error(
        &f.registry,
        &f.project,
        Some(&before.document_id),
        &waiting,
        1000,
    )
    .unwrap();
    assert_eq!(f.saved().state, "waiting_file");
}

#[test]
fn drive_scheduler_reconnecting_the_bound_account_resumes_auth_blocked_documents() {
    let f = Fixture::new(&[("ContextDock_Context_v1.md", b"one")]);
    f.api.fail_before.set(true);
    assert!(f.run().is_err());
    let pending = f.saved();
    f.registry
        .db
        .execute("UPDATE drive_projects SET automatic=1", [])
        .unwrap();
    crate::drive_scheduler::retry_error(
        &f.registry,
        &f.project,
        Some(&pending.document_id),
        &ApiError {
            auth: true,
            ..ApiError::local("expired grant")
        },
        1000,
    )
    .unwrap();
    crate::drive_scheduler::resume_auth(&f.registry, "different-account").unwrap();
    assert!(crate::drive_scheduler::due(&f.registry, 9_000_000)
        .unwrap()
        .is_empty());
    crate::drive_scheduler::resume_auth(&f.registry, "account-A").unwrap();
    assert_eq!(f.saved().state, "queued");
    crate::drive_scheduler::due(&f.registry, 9_000_000).unwrap();
    assert_eq!(
        crate::drive_scheduler::due(&f.registry, 9_003_000)
            .unwrap()
            .len(),
        1
    );
}
