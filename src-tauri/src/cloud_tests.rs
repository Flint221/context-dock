//! Entirely mocked provider. These tests never load credentials or use a network client.
use crate::{
    cloud::{self, n, s},
    cloud_api::{self, ApiError, CloudApi},
    cloud_cleanup, cloud_commands,
    cloud_models::CloudDraft,
    cloud_sync,
    registry::Registry,
};
use rusqlite::params;
use serde_json::{json, Value};
use std::{cell::RefCell, collections::HashSet, fs, path::PathBuf};
use tempfile::{tempdir, TempDir};
#[derive(Default)]
struct Remote {
    store: Option<Value>,
    files: Vec<(Value, Vec<u8>)>,
    associations: Vec<Value>,
    calls: Vec<String>,
    fail: Option<ApiError>,
    processing: bool,
    ignore_store_delete: bool,
    shared_file: bool,
    lose_file_delete_reply: bool,
    ignore_association_delete: bool,
    bad_association_receipt: bool,
    extra_stores: Vec<Value>,
    deny_file_content: bool,
    vanish_store_on_listing: bool,
}
#[derive(Default)]
struct Mock(RefCell<Remote>);
impl CloudApi for Mock {
    fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> std::result::Result<Value, ApiError> {
        let mut r = self.0.borrow_mut();
        r.calls.push(format!("{method} {path}"));
        if let Some(e) = r.fail.take() {
            return Err(e);
        }
        if path.starts_with("/vector_stores?") {
            let mut stores = r.store.iter().cloned().collect::<Vec<_>>();
            stores.extend(r.extra_stores.clone());
            if r.shared_file {
                stores.push(json!({"id":"vs_other","metadata":{}}));
            }
            return Ok(json!({"data":stores,"has_more":false}));
        }
        if path.starts_with("/files?") {
            return Ok(
                json!({"data":r.files.iter().map(|f|&f.0).collect::<Vec<_>>(),"has_more":false}),
            );
        }
        if path == "/vector_stores" && method == "POST" {
            let mut store = body.unwrap();
            store["id"] = json!("vs_test");
            store["status"] = json!("completed");
            store["usage_bytes"] = json!(42);
            r.store = Some(store.clone());
            return Ok(store);
        }
        if path == "/vector_stores/vs_test" {
            if method == "DELETE" {
                if !r.ignore_store_delete {
                    r.store = None;
                    r.associations.clear();
                }
                return Ok(json!({"deleted":true}));
            }
            return r.store.clone().ok_or_else(missing);
        }
        if path.starts_with("/vector_stores/vs_other/files?") {
            return Ok(json!({"data":r.associations,"has_more":false}));
        }
        if path.starts_with("/vector_stores/vs_older/files?")
            || path.starts_with("/vector_stores/vs_unrelated/files?")
        {
            return Ok(json!({"data":[],"has_more":false}));
        }
        if path == "/vector_stores/vs_older" || path == "/vector_stores/vs_unrelated" {
            let index = r
                .extra_stores
                .iter()
                .position(|v| path == format!("/vector_stores/{}", s(v, "id")))
                .ok_or_else(missing)?;
            if method == "DELETE" {
                r.extra_stores.remove(index);
                return Ok(json!({"deleted":true}));
            }
            return Ok(r.extra_stores[index].clone());
        }
        if path.starts_with("/vector_stores/vs_test/files?") {
            if r.vanish_store_on_listing {
                r.store = None;
                r.associations.clear();
                return Err(missing());
            }
            return Ok(json!({"data":r.associations,"has_more":false}));
        }
        if path == "/vector_stores/vs_test/files" && method == "POST" {
            let body = body.unwrap();
            let v = json!({"id":body["file_id"],"vector_store_id":"vs_test","attributes":body["attributes"],"status":if r.processing{"in_progress"}else{"completed"},"usage_bytes":42});
            r.associations.push(v.clone());
            return Ok(v);
        }
        if let Some(file) = path.strip_prefix("/vector_stores/vs_test/files/") {
            let index = r
                .associations
                .iter()
                .position(|a| s(a, "id") == file)
                .ok_or_else(missing)?;
            return match method {
                "DELETE" => {
                    if !r.ignore_association_delete && !r.bad_association_receipt {
                        r.associations.remove(index);
                    }
                    Ok(json!({"deleted":!r.bad_association_receipt}))
                }
                "POST" => {
                    r.associations[index]["attributes"] = body.unwrap()["attributes"].clone();
                    Ok(r.associations[index].clone())
                }
                _ => Ok(r.associations[index].clone()),
            };
        }
        if let Some(file) = path.strip_prefix("/files/") {
            let index = r
                .files
                .iter()
                .position(|f| s(&f.0, "id") == file)
                .ok_or_else(missing)?;
            if method == "DELETE" {
                r.files.remove(index);
                if r.lose_file_delete_reply {
                    r.lose_file_delete_reply = false;
                    return Err(ApiError {
                        message: "Synthetic lost delete response".into(),
                        ambiguous: true,
                        retry: false,
                        missing: false,
                        delay: 0,
                    });
                }
                return Ok(json!({"deleted":true}));
            }
            return Ok(r.files[index].0.clone());
        }
        if path == "/vector_stores/vs_test/search" {
            return Ok(
                json!({"data":r.associations.iter().map(|a|json!({"file_id":a["id"],"attributes":a["attributes"],"score":0.9,"content":[{"type":"text","text":"retrieved test content"}]})).collect::<Vec<_>>()}),
            );
        }
        panic!("Unexpected mocked API call: {method} {path}");
    }
    fn upload(&self, name: &str, bytes: Vec<u8>) -> std::result::Result<Value, ApiError> {
        let mut r = self.0.borrow_mut();
        r.calls.push("UPLOAD".into());
        let id = format!("file-{}", r.files.len() + 1);
        let file = json!({"id":id,"filename":name,"bytes":bytes.len(),"purpose":"assistants"});
        r.files.push((file.clone(), bytes));
        if let Some(e) = r.fail.take() {
            return Err(e);
        }
        Ok(file)
    }
    fn content(&self, id: &str) -> std::result::Result<Vec<u8>, ApiError> {
        if self.0.borrow().deny_file_content {
            return Err(ApiError::local("OpenAI rejected this file or request. Check supported type, size and account limits."));
        }
        self.0
            .borrow()
            .files
            .iter()
            .find(|f| s(&f.0, "id") == id)
            .map(|f| f.1.clone())
            .ok_or_else(missing)
    }
}
fn missing() -> ApiError {
    ApiError {
        message: "Remote resource missing".into(),
        missing: true,
        retry: false,
        ambiguous: false,
        delay: 0,
    }
}
struct Fixture {
    _temp: TempDir,
    root: PathBuf,
    r: Registry,
    id: String,
    doc: String,
}

fn word_package(media_size: usize) -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    zip.start_file("word/document.xml", options).unwrap();
    zip.write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Large illustrated document</w:t></w:r></w:p></w:body></w:document>"#).unwrap();
    zip.start_file("word/media/image.png", options).unwrap();
    zip.write_all(&vec![42; media_size]).unwrap();
    zip.finish().unwrap().into_inner()
}

#[test]
fn larger_word_allowance_retains_expanded_package_safety_boundary() {
    let mut bytes = word_package(1);
    // Declare an oversized media entry without allocating a huge test fixture.
    let central = bytes.windows(4).rposition(|w| w == b"PK\x01\x02").unwrap();
    bytes[central + 24..central + 28].copy_from_slice(&(129_u32 * 1024 * 1024).to_le_bytes());
    assert!(crate::extraction::extract(&bytes, "word")
        .unwrap_err()
        .contains("128 MiB expanded-package"));
}

#[test]
fn file_backoff_does_not_schedule_a_busy_loop_with_automatic_sync() {
    let mut f = Fixture::new();
    let mut draft = f.draft();
    draft.automatic = true;
    f.apply(draft);
    f.r.db
        .execute(
            "UPDATE cloud_projects SET paused=0,started=1,sync_requested=1",
            [],
        )
        .unwrap();
    let api = Mock::default();
    f.step(&api);
    cloud::queue(&mut f.r.db, &f.id).unwrap();
    f.r.db
        .execute(
            "UPDATE cloud_jobs SET next_at=?1",
            [chrono::Utc::now().timestamp() + 120],
        )
        .unwrap();
    let p = cloud::project(&f.r.db, &f.id).unwrap();
    assert!(!cloud_commands::needed(&f.r.db, &p).unwrap());
    f.r.db
        .execute("UPDATE cloud_jobs SET next_at=0", [])
        .unwrap();
    assert!(cloud_commands::needed(&f.r.db, &p).unwrap());
}

#[test]
fn illustrated_word_above_old_four_mib_package_screen_syncs_without_changing_local_bytes() {
    let mut f = Fixture::new();
    let bytes = word_package(5 * 1024 * 1024);
    fs::write(f.root.join("Illustrated.docx"), &bytes).unwrap();
    f.r.reconcile(&f.id, "manual_refresh", &HashSet::new(), true)
        .unwrap();
    let doc =
        f.r.snapshot()
            .unwrap()
            .documents
            .into_iter()
            .find(|d| d.name == "Illustrated.docx")
            .unwrap()
            .document_id;
    let mut draft = f.draft();
    draft.documents.push(doc.clone());
    f.apply(draft);
    f.r.db
        .execute(
            "UPDATE cloud_projects SET paused=0,started=1,sync_requested=1",
            [],
        )
        .unwrap();
    let api = Mock::default();
    for _ in 0..12 {
        f.step(&api);
    }
    assert!(f.jobs().iter().all(|j| s(j, "state") == "indexed"));
    assert_eq!(api.0.borrow().associations.len(), 2);
    assert!(api.0.borrow().files.iter().any(|(_, b)| b == &bytes));
    assert_eq!(fs::read(f.root.join("Illustrated.docx")).unwrap(), bytes);
    assert_eq!(f.r.versions(&doc).unwrap().len(), 1);
}

#[test]
fn bad_word_reports_path_and_reason_while_other_files_finish_without_retry_loop() {
    let mut f = Fixture::new();
    fs::create_dir(f.root.join("Drafts")).unwrap();
    let bad = b"This is not a DOCX ZIP";
    fs::write(f.root.join("Drafts/Bad.docx"), bad).unwrap();
    f.r.reconcile(&f.id, "manual_refresh", &HashSet::new(), true)
        .unwrap();
    let doc =
        f.r.snapshot()
            .unwrap()
            .documents
            .into_iter()
            .find(|d| d.name == "Bad.docx")
            .unwrap()
            .document_id;
    let mut draft = f.draft();
    draft.automatic = true;
    draft.documents.push(doc.clone());
    f.apply(draft);
    f.r.db
        .execute(
            "UPDATE cloud_projects SET paused=0,started=1,sync_requested=1",
            [],
        )
        .unwrap();
    let api = Mock::default();
    for _ in 0..12 {
        f.step(&api);
    }
    let jobs = f.jobs();
    let failed = jobs.iter().find(|j| s(j, "document_id") == doc).unwrap();
    assert_eq!(s(failed, "state"), "failed");
    assert!(
        s(failed, "error").contains("Drafts\\Bad.docx")
            || s(failed, "error").contains("Drafts/Bad.docx")
    );
    assert!(s(failed, "error").contains("version 1"));
    assert!(s(failed, "error").contains("end directory is missing"));
    assert_eq!(
        s(
            jobs.iter().find(|j| s(j, "document_id") == f.doc).unwrap(),
            "state"
        ),
        "indexed"
    );
    let view = cloud::view(&f.r.db, &f.id).unwrap();
    assert!(s(&view, "error").contains("Bad.docx"));
    assert_eq!(view["indexed"], 1);
    assert_eq!(view["failed"], 1);
    assert_eq!(n(&view, "next_at"), 0);
    assert!(!cloud_commands::needed(&f.r.db, &cloud::project(&f.r.db, &f.id).unwrap()).unwrap());
    assert_eq!(fs::read(f.root.join("Drafts/Bad.docx")).unwrap(), bad);
    assert_eq!(f.r.versions(&doc).unwrap().len(), 1);
    assert!(crate::chat::authorize(&f.r.db, &f.id, "bound").is_err());
}

#[test]
fn missing_selected_source_blocks_queued_uploads_and_recovers_without_losing_cloud_identity() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.sync(&api);
    let store = s(&cloud::project(&f.r.db, &f.id).unwrap(), "store_id").to_string();
    let bytes = fs::read(f.root.join("Master_Context.md")).unwrap();
    fs::remove_file(f.root.join("Master_Context.md")).unwrap();
    api.0.borrow_mut().calls.clear();
    f.step(&api);
    assert!(api.0.borrow().calls.is_empty());
    assert!(crate::chat::authorize(&f.r.db, &f.id, "bound")
        .unwrap_err()
        .contains("missing or inaccessible"));
    let view = cloud::view(&f.r.db, &f.id).unwrap();
    assert_eq!(view["localReady"], false);
    assert!(view["localError"].as_str().unwrap().contains("missing"));
    fs::write(f.root.join("Master_Context.md"), bytes).unwrap();
    f.r.reconcile(&f.id, "manual_refresh", &HashSet::new(), true)
        .unwrap();
    assert_eq!(cloud::view(&f.r.db, &f.id).unwrap()["localReady"], true);
    assert_eq!(
        s(&cloud::project(&f.r.db, &f.id).unwrap(), "store_id"),
        store
    );
    assert_eq!(f.r.versions(&f.doc).unwrap().len(), 1);
}

#[test]
fn reassignment_stops_old_request_guard_and_questions_wait_for_local_reconciliation() {
    use std::sync::{atomic::AtomicBool, Arc};
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.sync(&api);
    let guard = cloud_commands::ProjectApi::new(
        &api,
        &f.r.store.join("test.db"),
        &f.id,
        "bound",
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap();
    let target = f.root.parent().unwrap().join("Relocated");
    fs::rename(&f.root, &target).unwrap();
    let plan = f.r.folder_plan(&f.id, target.to_str().unwrap()).unwrap();
    f.r.reassign_folder(&f.id, &plan.path, &plan.token).unwrap();
    api.0.borrow_mut().calls.clear();
    assert!(guard
        .request("GET", "/vector_stores?limit=1", None)
        .unwrap_err()
        .message
        .contains("folder changed"));
    assert!(api.0.borrow().calls.is_empty());
    assert!(crate::chat::authorize(&f.r.db, &f.id, "bound")
        .unwrap_err()
        .contains("Recheck"));
    fs::write(target.join("Master_Context.md"), "Changed while moving").unwrap();
    f.r.reconcile(&f.id, "manual_refresh", &HashSet::new(), true)
        .unwrap();
    assert_eq!(f.r.versions(&f.doc).unwrap().len(), 2);
    assert!(crate::chat::authorize(&f.r.db, &f.id, "bound")
        .unwrap_err()
        .contains("still syncing"));
}
impl Fixture {
    fn new() -> Self {
        let temp = tempdir().unwrap();
        let root = temp.path().join("source");
        let store = temp.path().join("app");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&store).unwrap();
        fs::write(
            root.join("Master_Context.md"),
            "# Master Context\nProject goal: local indexing.\n",
        )
        .unwrap();
        let mut r = Registry::open(&store.join("test.db")).unwrap();
        let id = r.add("Disposable", root.to_str().unwrap()).unwrap();
        let sources = r.source_ids(&id).unwrap();
        r.configure_tracking(&id, true, false, sources).unwrap();
        let doc = r.snapshot().unwrap().documents[0].document_id.clone();
        r.db.execute(
            "UPDATE cloud_connection SET status='connected',scope='bound'",
            [],
        )
        .unwrap();
        Self {
            _temp: temp,
            root,
            r,
            id,
            doc,
        }
    }
    fn draft(&self) -> CloudDraft {
        CloudDraft {
            enabled: true,
            automatic: false,
            mode: "current".into(),
            documents: vec![self.doc.clone()],
            soft_limit: 52428800,
        }
    }
    fn apply(&mut self, draft: CloudDraft) {
        let plan = cloud::preview(&self.r.db, &self.id, &draft).unwrap();
        cloud::apply(&mut self.r.db, &self.id, &draft, &plan.token).unwrap();
    }
    fn enable(&mut self) {
        self.apply(self.draft());
        self.r
            .db
            .execute(
                "UPDATE cloud_projects SET paused=0,started=1,sync_requested=1 WHERE project_id=?1",
                [&self.id],
            )
            .unwrap();
    }
    fn step(&mut self, api: &Mock) {
        cloud_sync::step(&mut self.r.db, &self.r.store, &self.id, api, "bound").unwrap();
    }
    fn sync(&mut self, api: &Mock) {
        for _ in 0..7 {
            self.step(api);
        }
    }
    fn edit(&mut self) {
        fs::write(
            self.root.join("Master_Context.md"),
            "# Master Context\nProject goal: indexed replacement.\nNew feature: cloud search.\n",
        )
        .unwrap();
        self.r
            .reconcile(&self.id, "manual_refresh", &HashSet::new(), true)
            .unwrap();
    }
    fn jobs(&self) -> Vec<Value> {
        cloud::jobs(&self.r.db, &self.id).unwrap()
    }
}
#[test]
#[cfg(windows)]
fn cloud_runtime_normalizes_windows_database_path_before_guarded_upload() {
    let mut f = Fixture::new();
    f.enable();
    let ordinary_path = PathBuf::from(crate::scanner::explorer_path(
        f.r.store.join("test.db").to_str().unwrap(),
    ));
    assert!(!ordinary_path.to_str().unwrap().starts_with(r"\\?\"));
    let path = cloud_commands::runtime_database_path(ordinary_path).unwrap();
    let root = path.parent().unwrap();
    assert_eq!(root, f.r.store);
    let mut db = cloud::connect(&path).unwrap();
    let api = Mock::default();
    for _ in 0..7 {
        cloud_sync::step(&mut db, root, &f.id, &api, "bound").unwrap();
    }
    assert_eq!(s(&f.jobs()[0], "state"), "indexed");
    let remote = api.0.borrow();
    assert_eq!(remote.files.len(), 1);
    assert_eq!(
        remote.files[0].1,
        fs::read(f.root.join("Master_Context.md")).unwrap()
    );
}
#[test]
fn folder_selected_tracked_document_does_not_require_legacy_cloud_selection() {
    let mut f = Fixture::new();
    f.enable();
    f.r.db
        .execute("DELETE FROM cloud_documents WHERE project_id=?1", [&f.id])
        .unwrap();
    let draft = cloud::draft(&f.r.db, &cloud::project(&f.r.db, &f.id).unwrap()).unwrap();
    assert_eq!(draft.documents, vec![f.doc.clone()]);
    let api = Mock::default();
    f.sync(&api);
    assert_eq!(
        f.jobs()
            .iter()
            .filter(|j| s(j, "state") == "indexed")
            .count(),
        1
    );
}
#[test]
fn legacy_cloud_list_cannot_bypass_source_validation() {
    let mut f = Fixture::new();
    f.enable();
    fs::remove_file(f.root.join("Master_Context.md")).unwrap();
    f.r.refresh(&f.id).unwrap();
    let mut old_client = f.draft();
    old_client.automatic = true;
    old_client.documents.clear();
    assert!(!cloud::selection_reduction(
        &f.r.db,
        &cloud::project(&f.r.db, &f.id).unwrap(),
        &old_client
    )
    .unwrap());
    assert!(cloud::preview(&f.r.db, &f.id, &old_client).is_err());
}

#[test]
fn tracking_off_prevents_sync_and_next_guarded_upload() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    let guard = cloud_commands::ProjectApi::new(
        &api,
        &f.r.store.join("test.db"),
        &f.id,
        "bound",
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
    )
    .unwrap();
    f.r.configure_tracking(&f.id, false, false, vec![]).unwrap();
    assert!(guard.upload("blocked.md", b"private".to_vec()).is_err());
    assert!(!cloud_commands::needed(&f.r.db, &cloud::project(&f.r.db, &f.id).unwrap()).unwrap());
    f.step(&api);
    assert!(api.0.borrow().calls.is_empty());
}
#[test]
fn migration_defaults_cloud_off_and_preserves_local_preferences() {
    let f = Fixture::new();
    let p = cloud::project(&f.r.db, &f.id).unwrap();
    assert_eq!(n(&p, "enabled"), 0);
    assert_eq!(n(&p, "auto_sync"), 0);
    assert!(f.r.snapshot().unwrap().projects[0].tracking_enabled);
    assert_eq!(f.r.versions(&f.doc).unwrap().len(), 1);
}
#[test]
fn schema_two_database_migrates_without_cloud_opt_in() {
    let t = tempdir().unwrap();
    let path = t.path().join("old.db");
    {
        let db = rusqlite::Connection::open(&path).unwrap();
        db.execute_batch(include_str!("../migrations/001_registry.sql"))
            .unwrap();
        db.execute_batch(include_str!("../migrations/002_tracking.sql"))
            .unwrap();
        db.execute("INSERT INTO projects(id,name,path,path_key,created_at,tracking_enabled,archive_enabled) VALUES('old','Old','C:/old','old','now',1,1)",[]).unwrap();
        db.pragma_update(None, "user_version", 2).unwrap();
    }
    let r = Registry::open(&path).unwrap();
    assert_eq!(n(&cloud::project(&r.db, "old").unwrap(), "enabled"), 0);
    let flags: (i64, i64) =
        r.db.query_row(
            "SELECT tracking_enabled,archive_enabled FROM projects",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(flags, (1, 1));
}
#[test]
fn credentials_are_validated_without_echoing_input() {
    let secret = "secret with spaces";
    assert!(!crate::cloud_credentials::validate(secret.into())
        .unwrap_err()
        .contains(secret));
    assert!(crate::cloud_credentials::validate("a".repeat(40)).is_ok());
    assert!(crate::cloud_credentials::validate("a".repeat(5000)).is_err());
}

#[test]
fn history_policy_round_trip_detaches_and_restores_original_captures_without_duplicate_files() {
    let mut f = Fixture::new();
    f.edit();
    let api = Mock::default();
    let mut history = f.draft();
    history.mode = "history".into();
    f.apply(history.clone());
    f.r.db
        .execute(
            "UPDATE cloud_projects SET started=1,paused=0,sync_requested=1",
            [],
        )
        .unwrap();
    for _ in 0..12 {
        f.step(&api);
    }
    assert_eq!(api.0.borrow().associations.len(), 2);
    let files = api
        .0
        .borrow()
        .files
        .iter()
        .map(|(v, _)| s(v, "id").to_string())
        .collect::<Vec<_>>();
    f.apply(f.draft());
    f.sync(&api);
    assert_eq!(api.0.borrow().associations.len(), 1);
    assert_eq!(
        api.0.borrow().associations[0]["attributes"]["is_current"],
        true
    );
    f.apply(history);
    f.sync(&api);
    assert_eq!(api.0.borrow().associations.len(), 2);
    assert_eq!(
        api.0
            .borrow()
            .files
            .iter()
            .map(|(v, _)| s(v, "id").to_string())
            .collect::<Vec<_>>(),
        files
    );
    assert_eq!(f.r.versions(&f.doc).unwrap().len(), 2);
}

#[test]
fn cloud_off_stop_intent_survives_an_in_flight_document_removal_failure() {
    struct BeginCleanup {
        db: rusqlite::Connection,
        id: String,
        payload: Value,
    }
    impl CloudApi for BeginCleanup {
        fn request(&self, _: &str, _: &str, _: Option<Value>) -> Result<Value, ApiError> {
            cloud_cleanup::begin(&self.db, &self.id, &self.payload).unwrap();
            Err(ApiError::local(
                "Ordinary request stopped for full cleanup.",
            ))
        }
        fn upload(&self, _: &str, _: Vec<u8>) -> Result<Value, ApiError> {
            panic!("Cleanup must not upload");
        }
        fn content(&self, _: &str) -> Result<Vec<u8>, ApiError> {
            panic!("No content request is expected");
        }
    }
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.sync(&api);
    let mut none = f.draft();
    none.documents.clear();
    f.apply(none);
    let p = cloud::project(&f.r.db, &f.id).unwrap();
    let interrupt = BeginCleanup {
        db: cloud::connect(&f.r.store.join("test.db")).unwrap(),
        id: f.id.clone(),
        payload: json!({"confirm":true,"storeId":p["store_id"],"confirmation":p["store_name"]}),
    };
    cloud_sync::step(&mut f.r.db, &f.r.store, &f.id, &interrupt, "bound").unwrap();
    assert_eq!(
        s(&cloud::project(&f.r.db, &f.id).unwrap(), "status"),
        "cleanup_pending"
    );
    cloud_cleanup::run(&mut f.r.db, &f.id, &api, "bound").unwrap();
    assert_eq!(
        s(&cloud::project(&f.r.db, &f.id).unwrap(), "status"),
        "cloud_deleted"
    );
    assert!(api.0.borrow().store.is_none());
    assert!(api.0.borrow().files.is_empty());
}

#[test]
fn document_checkbox_round_trip_removes_all_family_versions_while_paused_and_restores_the_same_store(
) {
    use std::sync::{atomic::AtomicBool, Arc};
    let mut f = Fixture::new();
    f.edit();
    fs::write(f.root.join("Other.md"), "Independent family").unwrap();
    f.r.reconcile(&f.id, "manual_refresh", &HashSet::new(), true)
        .unwrap();
    let other =
        f.r.snapshot()
            .unwrap()
            .documents
            .iter()
            .find(|d| d.name == "Other.md")
            .unwrap()
            .document_id
            .clone();
    let mut full = f.draft();
    full.mode = "history".into();
    full.documents.push(other.clone());
    f.apply(full.clone());
    f.r.db
        .execute(
            "UPDATE cloud_projects SET started=1,paused=0,sync_requested=1",
            [],
        )
        .unwrap();
    let api = Mock::default();
    for _ in 0..20 {
        f.step(&api);
    }
    assert_eq!(api.0.borrow().associations.len(), 3);
    let store = s(&cloud::project(&f.r.db, &f.id).unwrap(), "store_id").to_string();
    f.r.db
        .execute("UPDATE cloud_projects SET paused=1,sync_requested=0", [])
        .unwrap();
    let mut reduced = full.clone();
    reduced.documents = vec![other.clone()];
    f.apply(reduced);
    assert_eq!(cloud::selection_removals(&f.r.db, &f.id).unwrap().len(), 2);
    assert_eq!(
        n(&cloud::project(&f.r.db, &f.id).unwrap(), "sync_requested"),
        0
    );
    api.0.borrow_mut().calls.clear();
    let guarded = cloud_commands::ProjectApi::new(
        &api,
        &f.r.store.join("test.db"),
        &f.id,
        "bound",
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap();
    for _ in 0..2 {
        cloud_sync::step(&mut f.r.db, &f.r.store, &f.id, &guarded, "bound").unwrap();
    }
    assert_eq!(api.0.borrow().associations.len(), 1);
    assert_eq!(
        s(&api.0.borrow().associations[0]["attributes"], "document"),
        other
    );
    assert!(api
        .0
        .borrow()
        .calls
        .iter()
        .all(|c| c.starts_with("GET ") || c.starts_with("DELETE /vector_stores/vs_test/files/")));
    assert_eq!(api.0.borrow().files.len(), 3);
    assert_eq!(n(&cloud::project(&f.r.db, &f.id).unwrap(), "paused"), 1);
    f.apply(full);
    f.step(&api);
    assert_eq!(api.0.borrow().associations.len(), 1);
    f.r.db
        .execute("UPDATE cloud_projects SET paused=0,sync_requested=1", [])
        .unwrap();
    for _ in 0..20 {
        f.step(&api);
    }
    assert_eq!(api.0.borrow().associations.len(), 3);
    assert_eq!(api.0.borrow().files.len(), 3);
    assert!(!api.0.borrow().calls.iter().any(|c| c == "UPLOAD"));
    assert_eq!(
        s(&cloud::project(&f.r.db, &f.id).unwrap(), "store_id"),
        store
    );
    assert_eq!(f.r.versions(&f.doc).unwrap().len(), 2);
}

#[test]
fn opted_out_documents_are_removed_before_unrelated_additions_even_if_new_source_goes_missing() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.sync(&api);
    fs::write(f.root.join("Replacement.md"), "New family").unwrap();
    f.r.reconcile(&f.id, "manual_refresh", &HashSet::new(), true)
        .unwrap();
    let other =
        f.r.snapshot()
            .unwrap()
            .documents
            .iter()
            .find(|d| d.name == "Replacement.md")
            .unwrap()
            .document_id
            .clone();
    let mut changed = f.draft();
    changed.documents = vec![other];
    f.apply(changed);
    fs::remove_file(f.root.join("Replacement.md")).unwrap();
    api.0.borrow_mut().calls.clear();
    f.step(&api);
    assert!(api.0.borrow().associations.is_empty());
    assert!(!api.0.borrow().calls.iter().any(|c| c == "UPLOAD"));
    assert_eq!(api.0.borrow().files.len(), 1);
    assert!(cloud::selection_removals(&f.r.db, &f.id)
        .unwrap()
        .is_empty());
}

#[test]
fn selection_removal_verifies_acknowledgement_and_absence_and_survives_restart_for_retry() {
    for bad_receipt in [true, false] {
        let mut f = Fixture::new();
        f.enable();
        let api = Mock::default();
        f.sync(&api);
        f.r.db
            .execute("UPDATE cloud_projects SET paused=1", [])
            .unwrap();
        let mut none = f.draft();
        none.documents.clear();
        f.apply(none);
        api.0.borrow_mut().bad_association_receipt = bad_receipt;
        api.0.borrow_mut().ignore_association_delete = !bad_receipt;
        f.step(&api);
        assert_eq!(cloud::selection_removals(&f.r.db, &f.id).unwrap().len(), 1);
        assert_eq!(
            s(&cloud::project(&f.r.db, &f.id).unwrap(), "status"),
            "selection_removal_failed"
        );
        assert_eq!(
            cloud::view(&f.r.db, &f.id).unwrap()["status"],
            "selection_removal_failed"
        );
        let db = f.r.store.join("test.db");
        f.r = Registry::open(&db).unwrap();
        assert!(
            !cloud_commands::needed(&f.r.db, &cloud::project(&f.r.db, &f.id).unwrap()).unwrap()
        );
        assert!(!s(&f.jobs()[0], "file_id").is_empty());
        api.0.borrow_mut().bad_association_receipt = false;
        api.0.borrow_mut().ignore_association_delete = false;
        cloud_commands::action_impl(
            &mut f.r.db,
            Some(&f.id),
            "retry_selection_removals",
            &json!({}),
        )
        .unwrap();
        f.step(&api);
        assert!(api.0.borrow().associations.is_empty());
        assert_eq!(n(&cloud::project(&f.r.db, &f.id).unwrap(), "paused"), 1);
        assert_eq!(api.0.borrow().files.len(), 1);
    }
}

#[test]
fn pure_opt_out_can_be_saved_without_source_or_credentials_and_only_authorizes_narrow_deletion() {
    use std::sync::{atomic::AtomicBool, Arc};
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.sync(&api);
    fs::rename(&f.root, f.root.with_file_name("unavailable")).unwrap();
    f.r.db
        .execute("UPDATE cloud_projects SET paused=1", [])
        .unwrap();
    f.r.db
        .execute("UPDATE cloud_connection SET status='disconnected'", [])
        .unwrap();
    let mut none = f.draft();
    none.documents.clear();
    let plan =
        cloud_commands::action_impl(&mut f.r.db, Some(&f.id), "preview", &json!({"draft":none}))
            .unwrap();
    cloud_commands::action_impl(
        &mut f.r.db,
        Some(&f.id),
        "apply",
        &json!({"draft":none,"token":plan["token"],"consent":true}),
    )
    .unwrap();
    assert_eq!(cloud::selection_removals(&f.r.db, &f.id).unwrap().len(), 1);
    let guarded = cloud_commands::ProjectApi::new(
        &api,
        &f.r.store.join("test.db"),
        &f.id,
        "bound",
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap();
    api.0.borrow_mut().calls.clear();
    assert!(guarded
        .request("GET", "/vector_stores/vs_test", None)
        .is_err());
    f.r.db
        .execute("UPDATE cloud_connection SET status='connected'", [])
        .unwrap();
    assert!(guarded
        .upload("blocked.md", b"Never upload".to_vec())
        .is_err());
    assert!(guarded
        .request("DELETE", "/vector_stores/vs_unrelated/files/file-1", None)
        .is_err());
    assert!(api.0.borrow().calls.is_empty());
    cloud_sync::step(&mut f.r.db, &f.r.store, &f.id, &guarded, "bound").unwrap();
    assert!(api.0.borrow().associations.is_empty());
}

#[test]
fn lost_upload_response_is_recovered_after_document_opt_out_and_reselection_without_duplicate_upload(
) {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.step(&api);
    api.0.borrow_mut().fail = Some(ApiError {
        message: "Lost upload receipt".into(),
        ambiguous: true,
        retry: false,
        missing: false,
        delay: 0,
    });
    // The next step checks the store before upload; inject the ambiguous upload state
    // using the mock's upload path directly to reproduce a sent-but-unacknowledged upload.
    cloud::queue(&mut f.r.db, &f.id).unwrap();
    let job = f.jobs().remove(0);
    let version = cloud::version(&f.r.db, s(&job, "version_id")).unwrap();
    assert!(api
        .upload(
            s(&job, "remote_name"),
            cloud::verified(&f.r.store, &version, 52428800).unwrap()
        )
        .is_err());
    f.r.db
        .execute(
            "UPDATE cloud_jobs SET state='uncertain',phase='recover'",
            [],
        )
        .unwrap();
    let mut none = f.draft();
    none.documents.clear();
    f.apply(none);
    f.sync(&api);
    assert_eq!(s(&f.jobs()[0], "state"), "removed");
    f.apply(f.draft());
    f.sync(&api);
    assert_eq!(api.0.borrow().files.len(), 1);
    assert_eq!(
        api.0
            .borrow()
            .calls
            .iter()
            .filter(|c| *c == "UPLOAD")
            .count(),
        1
    );
    assert_eq!(s(&f.jobs()[0], "state"), "indexed");
}

#[test]
fn cloud_off_deletes_all_owned_old_stores_preserves_foreign_store_and_reenable_restores_saved_selection(
) {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.sync(&api);
    let install = cloud::install(&f.r.db).unwrap();
    api.0.borrow_mut().extra_stores = vec![
        json!({"id":"vs_older","metadata":{"contextdock":install,"project":f.id,"operation":"old-operation"}}),
        json!({"id":"vs_unrelated","metadata":{"contextdock":install,"project":"other-project","operation":"unrelated"}}),
    ];
    let p = cloud::project(&f.r.db, &f.id).unwrap();
    let payload = json!({"confirm":true,"storeId":p["store_id"],"confirmation":p["store_name"]});
    cloud_cleanup::begin(&f.r.db, &f.id, &payload).unwrap();
    cloud_cleanup::run(&mut f.r.db, &f.id, &api, "bound").unwrap();
    assert!(api.0.borrow().store.is_none());
    assert!(api.0.borrow().files.is_empty());
    assert_eq!(api.0.borrow().extra_stores.len(), 1);
    assert_eq!(s(&api.0.borrow().extra_stores[0], "id"), "vs_unrelated");
    let saved = cloud::draft(&f.r.db, &cloud::project(&f.r.db, &f.id).unwrap()).unwrap();
    assert_eq!(saved.documents, vec![f.doc.clone()]);
    f.enable();
    f.sync(&api);
    assert_eq!(api.0.borrow().associations.len(), 1);
    assert_eq!(
        s(&api.0.borrow().associations[0]["attributes"], "project"),
        f.id
    );
    assert_eq!(f.r.versions(&f.doc).unwrap().len(), 1);
    for status in ["cleanup_pending", "cleanup_failed"] {
        f.r.db
            .execute(
                "UPDATE cloud_projects SET paused=1,status=?1,error='Cleanup needs verification'",
                [status],
            )
            .unwrap();
        assert_eq!(cloud::view(&f.r.db, &f.id).unwrap()["status"], status);
    }
}
#[test]
fn toggle_commands_reject_a_database_write_that_did_not_save_the_requested_state() {
    let mut f = Fixture::new();
    f.enable();
    f.r.db.execute_batch("CREATE TRIGGER reject_cloud_off AFTER UPDATE OF enabled ON cloud_projects WHEN NEW.enabled=0 BEGIN UPDATE cloud_projects SET enabled=1 WHERE project_id=NEW.project_id; END;").unwrap();
    let err = cloud_commands::action_impl(
        &mut f.r.db,
        Some(&f.id),
        "disconnect",
        &json!({"retainCloud":true}),
    )
    .unwrap_err();
    assert!(err.contains("verification failed"));
    assert_eq!(n(&cloud::project(&f.r.db, &f.id).unwrap(), "enabled"), 1);
    f.r.db
        .execute_batch("DROP TRIGGER reject_cloud_off;")
        .unwrap();
    cloud_commands::action_impl(
        &mut f.r.db,
        Some(&f.id),
        "disconnect",
        &json!({"retainCloud":true}),
    )
    .unwrap();
    cloud::verify_flags(&f.r.db, &f.id, &[("enabled", 0), ("paused", 1)]).unwrap();
    assert_eq!(f.r.versions(&f.doc).unwrap().len(), 1);
}
#[test]
fn cloud_configuration_verification_rolls_back_a_silently_ignored_save() {
    let mut f = Fixture::new();
    f.enable();
    let mut draft = f.draft();
    draft.automatic = true;
    let plan = cloud::preview(&f.r.db, &f.id, &draft).unwrap();
    f.r.db.execute_batch("CREATE TRIGGER ignore_cloud_settings BEFORE UPDATE ON cloud_projects BEGIN SELECT RAISE(IGNORE); END;").unwrap();
    assert!(cloud::apply(&mut f.r.db, &f.id, &draft, &plan.token)
        .unwrap_err()
        .contains("verification failed"));
    assert_eq!(n(&cloud::project(&f.r.db, &f.id).unwrap(), "auto_sync"), 0);
}
#[test]
fn no_api_requests_for_local_only_or_paused() {
    let mut f = Fixture::new();
    let api = Mock::default();
    f.step(&api);
    assert!(api.0.borrow().calls.is_empty());
    f.enable();
    f.r.db
        .execute("UPDATE cloud_projects SET paused=1", [])
        .unwrap();
    f.step(&api);
    assert!(api.0.borrow().calls.is_empty());
}
#[test]
fn credential_scope_cannot_cross_project_binding() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    cloud_sync::step(&mut f.r.db, &f.r.store, &f.id, &api, "different-key").unwrap();
    assert!(api.0.borrow().calls.is_empty());
}
#[test]
fn selection_is_project_scoped_and_untracked_is_ineligible() {
    let f = Fixture::new();
    let mut d = f.draft();
    d.documents = vec!["foreign-document".into()];
    assert!(cloud::preview(&f.r.db, &f.id, &d).is_err());
    f.r.db
        .execute("UPDATE documents SET tracking_state='ignored'", [])
        .unwrap();
    assert!(cloud::preview(&f.r.db, &f.id, &f.draft()).is_err());
}
#[test]
fn preview_is_invalidated_by_new_capture() {
    let mut f = Fixture::new();
    let d = f.draft();
    let p = cloud::preview(&f.r.db, &f.id, &d).unwrap();
    f.edit();
    assert!(cloud::apply(&mut f.r.db, &f.id, &d, &p.token).is_err());
}
#[test]
fn consent_does_not_start_uploads() {
    let mut f = Fixture::new();
    f.apply(f.draft());
    let api = Mock::default();
    f.sync(&api);
    assert!(api.0.borrow().files.is_empty());
    assert!(api.0.borrow().store.is_none());
}
#[test]
fn snapshot_hash_mismatch_blocks_upload_without_source_fallback() {
    let mut f = Fixture::new();
    f.enable();
    let v = cloud::desired(&f.r.db, &f.id, &f.draft())
        .unwrap()
        .remove(0);
    let path = f.r.store.join(v.snapshot);
    fs::remove_file(&path).unwrap();
    fs::write(path, "tampered").unwrap();
    let api = Mock::default();
    f.sync(&api);
    assert!(api.0.borrow().files.is_empty());
    assert!(s(&f.jobs()[0], "error").contains("integrity"));
    assert!(f.root.join("Master_Context.md").exists());
}
#[test]
fn missing_snapshot_blocks_upload() {
    let mut f = Fixture::new();
    f.enable();
    let v = cloud::desired(&f.r.db, &f.id, &f.draft())
        .unwrap()
        .remove(0);
    fs::remove_file(f.r.store.join(v.snapshot)).unwrap();
    let api = Mock::default();
    f.sync(&api);
    assert!(api.0.borrow().files.is_empty());
}
#[test]
fn soft_limit_blocks_snapshot() {
    let f = Fixture::new();
    let v = cloud::desired(&f.r.db, &f.id, &f.draft())
        .unwrap()
        .remove(0);
    assert!(cloud::verified(&f.r.store, &v, 1).is_err());
}
#[test]
fn uploaded_bytes_are_original_snapshot_and_not_live_source() {
    let mut f = Fixture::new();
    f.enable();
    let expected = fs::read(f.root.join("Master_Context.md")).unwrap();
    fs::write(f.root.join("Master_Context.md"), "not yet captured").unwrap();
    let api = Mock::default();
    f.sync(&api);
    assert_eq!(api.0.borrow().files[0].1, expected);
    assert!(!s(&api.0.borrow().files[0].0, "filename").contains("Master_Context"));
}
#[test]
fn upload_does_not_imply_indexed_until_remote_processing_completes() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    api.0.borrow_mut().processing = true;
    f.sync(&api);
    assert_eq!(s(&f.jobs()[0], "state"), "indexing");
    assert_eq!(n(&cloud::view(&f.r.db, &f.id).unwrap(), "indexed"), 0);
    api.0.borrow_mut().associations[0]["status"] = json!("completed");
    f.r.db
        .execute("UPDATE cloud_jobs SET next_at=0", [])
        .unwrap();
    f.sync(&api);
    assert_eq!(s(&f.jobs()[0], "state"), "indexed");
}
#[test]
fn repeated_sync_deduplicates_store_files_and_associations() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.sync(&api);
    f.r.db
        .execute("UPDATE cloud_projects SET sync_requested=1", [])
        .unwrap();
    f.sync(&api);
    let r = api.0.borrow();
    assert_eq!(r.files.len(), 1);
    assert_eq!(r.associations.len(), 1);
    assert_eq!(
        r.calls
            .iter()
            .filter(|c| *c == "POST /vector_stores")
            .count(),
        1
    );
}
#[test]
fn current_only_replacement_waits_for_completed_indexing() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.sync(&api);
    f.edit();
    f.r.db
        .execute("UPDATE cloud_projects SET sync_requested=1", [])
        .unwrap();
    api.0.borrow_mut().processing = true;
    f.sync(&api);
    assert_eq!(api.0.borrow().associations.len(), 2);
    assert_eq!(api.0.borrow().files.len(), 2);
    let mut remote = api.0.borrow_mut();
    for a in &mut remote.associations {
        a["status"] = json!("completed");
    }
    drop(remote);
    f.r.db
        .execute("UPDATE cloud_jobs SET next_at=0", [])
        .unwrap();
    f.sync(&api);
    assert_eq!(api.0.borrow().associations.len(), 1);
    assert_eq!(api.0.borrow().files.len(), 2);
    assert_eq!(s(&f.jobs()[0], "state"), "removed");
}
#[test]
fn history_policy_reattaches_a_completed_current_only_replacement() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.sync(&api);
    f.edit();
    f.r.db
        .execute("UPDATE cloud_projects SET sync_requested=1", [])
        .unwrap();
    f.sync(&api);
    assert_eq!(s(&f.jobs()[0], "state"), "removed");
    let old_file = s(&f.jobs()[0], "file_id").to_string();
    let mut draft = f.draft();
    draft.mode = "history".into();
    f.apply(draft);
    f.sync(&api);
    assert_eq!(api.0.borrow().associations.len(), 2);
    assert_eq!(api.0.borrow().files.len(), 2);
    assert_eq!(s(&f.jobs()[0], "state"), "indexed");
    assert_eq!(s(&f.jobs()[0], "file_id"), old_file);
}
#[test]
fn history_mode_keeps_both_versions_and_sets_current_metadata() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.sync(&api);
    f.edit();
    let mut d = f.draft();
    d.mode = "history".into();
    f.apply(d);
    f.sync(&api);
    assert_eq!(api.0.borrow().associations.len(), 2);
    let r = api.0.borrow();
    assert_eq!(
        r.associations
            .iter()
            .filter(|a| a["attributes"]["is_current"] == true)
            .count(),
        1
    );
}
#[test]
fn auto_sync_off_requires_sync_now_for_new_versions() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.sync(&api);
    f.edit();
    assert!(!cloud_commands::needed(&f.r.db, &cloud::project(&f.r.db, &f.id).unwrap()).unwrap());
    assert_eq!(api.0.borrow().files.len(), 1);
}
#[test]
fn auto_sync_detects_new_committed_version() {
    let mut f = Fixture::new();
    f.enable();
    f.r.db
        .execute("UPDATE cloud_projects SET auto_sync=1", [])
        .unwrap();
    let api = Mock::default();
    f.sync(&api);
    f.edit();
    assert!(cloud_commands::needed(&f.r.db, &cloud::project(&f.r.db, &f.id).unwrap()).unwrap());
    f.sync(&api);
    assert_eq!(api.0.borrow().files.len(), 2);
}
#[test]
fn interrupted_create_adopts_owned_index_instead_of_creating_duplicate() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.step(&api);
    f.r.db
        .execute(
            "UPDATE cloud_projects SET store_id=NULL,store_state='creating'",
            [],
        )
        .unwrap();
    f.step(&api);
    assert_eq!(
        s(&cloud::project(&f.r.db, &f.id).unwrap(), "store_id"),
        "vs_test"
    );
    assert_eq!(
        api.0
            .borrow()
            .calls
            .iter()
            .filter(|c| *c == "POST /vector_stores")
            .count(),
        1
    );
}
#[test]
fn interrupted_create_with_no_match_does_not_blindly_create() {
    let mut f = Fixture::new();
    f.enable();
    f.r.db
        .execute("UPDATE cloud_projects SET store_state='creating'", [])
        .unwrap();
    let api = Mock::default();
    f.step(&api);
    assert!(api.0.borrow().store.is_none());
    assert_eq!(
        s(&cloud::project(&f.r.db, &f.id).unwrap(), "status"),
        "needs_attention"
    );
}
#[test]
fn interrupted_upload_recovers_exact_file_by_name_size_and_hash() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.step(&api);
    f.step(&api);
    f.r.db
        .execute("UPDATE cloud_jobs SET file_id=NULL,state='uploading'", [])
        .unwrap();
    f.sync(&api);
    assert_eq!(api.0.borrow().files.len(), 1);
    assert_eq!(s(&f.jobs()[0], "state"), "indexed");
}
#[test]
fn mismatched_recovered_upload_bytes_are_rejected() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.step(&api);
    f.step(&api);
    f.r.db
        .execute("UPDATE cloud_jobs SET file_id=NULL,state='uploading'", [])
        .unwrap();
    api.0.borrow_mut().files[0].1 = b"bad bytes".to_vec();
    f.step(&api);
    assert!(api.0.borrow().associations.is_empty());
    assert!(s(&f.jobs()[0], "error").contains("different bytes"));
}
#[test]
fn foreign_index_identity_prevents_all_uploads_and_deletions() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.step(&api);
    api.0.borrow_mut().store.as_mut().unwrap()["metadata"]["project"] = json!("other");
    f.sync(&api);
    assert!(api.0.borrow().files.is_empty());
    assert_eq!(
        s(&cloud::project(&f.r.db, &f.id).unwrap(), "status"),
        "needs_attention"
    );
}
#[test]
fn rate_limit_respects_retry_after_without_tight_loop() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    api.0.borrow_mut().fail = Some(ApiError {
        message: "Rate limited".into(),
        retry: true,
        ambiguous: false,
        missing: false,
        delay: 120,
    });
    f.step(&api);
    let p = cloud::project(&f.r.db, &f.id).unwrap();
    assert!(n(&p, "next_at") >= chrono::Utc::now().timestamp() + 119);
    f.step(&api);
    assert_eq!(api.0.borrow().calls.len(), 1);
    assert_eq!(s(&p, "store_state"), "pending");
}
#[test]
fn auth_failure_requires_explicit_retry() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    api.0.borrow_mut().fail = Some(ApiError::local("Key rejected"));
    f.step(&api);
    assert_eq!(
        n(&cloud::project(&f.r.db, &f.id).unwrap(), "next_at"),
        i64::MAX
    );
    f.step(&api);
    assert_eq!(api.0.borrow().calls.len(), 1);
}
#[test]
fn missing_remote_store_never_creates_silent_replacement() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.sync(&api);
    api.0.borrow_mut().store = None;
    f.r.db
        .execute("UPDATE cloud_projects SET reconcile_requested=1", [])
        .unwrap();
    f.step(&api);
    assert_eq!(
        api.0
            .borrow()
            .calls
            .iter()
            .filter(|c| *c == "POST /vector_stores")
            .count(),
        1
    );
    assert_eq!(n(&cloud::view(&f.r.db, &f.id).unwrap(), "indexed"), 0);
}
#[test]
fn missing_association_is_reconciled_using_existing_file() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.sync(&api);
    api.0.borrow_mut().associations.clear();
    f.r.db
        .execute("UPDATE cloud_projects SET reconcile_requested=1", [])
        .unwrap();
    f.sync(&api);
    assert_eq!(api.0.borrow().files.len(), 1);
    assert_eq!(api.0.borrow().associations.len(), 1);
}
#[test]
fn explicit_removal_excludes_version_and_retains_file_and_local_history() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.sync(&api);
    let version = f.jobs()[0]["version_id"].clone();
    cloud_commands::action_impl(
        &mut f.r.db,
        Some(&f.id),
        "remove",
        &json!({"version":version,"confirm":true}),
    )
    .unwrap();
    f.sync(&api);
    assert!(api.0.borrow().associations.is_empty());
    assert_eq!(api.0.borrow().files.len(), 1);
    assert_eq!(f.r.versions(&f.doc).unwrap().len(), 1);
    assert!(f.root.join("Master_Context.md").exists());
}
#[test]
fn disconnect_and_unregister_do_not_delete_remote_resources() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.sync(&api);
    cloud_commands::action_impl(
        &mut f.r.db,
        Some(&f.id),
        "disconnect",
        &json!({"retainCloud":true}),
    )
    .unwrap();
    let count = api.0.borrow().calls.len();
    f.step(&api);
    assert_eq!(api.0.borrow().calls.len(), count);
    f.r.remove(&f.id).unwrap();
    assert!(api.0.borrow().store.is_some());
    assert_eq!(api.0.borrow().files.len(), 1);
}
#[test]
fn search_uses_direct_endpoint_with_local_provenance_without_storing_query() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.sync(&api);
    let hits = cloud_sync::search(&f.r.db, &f.id, "private test query", &api, "bound").unwrap();
    assert_eq!(hits[0]["familyName"], "Master Context");
    let stored: String =
        f.r.db
            .query_row("SELECT results FROM cloud_retrievals", [], |r| r.get(0))
            .unwrap();
    assert!(!stored.contains("private test query"));
    assert!(!stored.contains("retrieved test content"));
    assert!(api
        .0
        .borrow()
        .calls
        .iter()
        .all(|c| !c.contains("responses")));
}
#[test]
fn pagination_and_remote_identifiers_fail_closed() {
    assert!(cloud_api::valid_id("vs_foo/../../files", "vs_").is_err());
    assert!(cloud_api::valid_id("file-good_123", "file").is_ok());
}
#[test]
fn foreign_association_blocks_index_deletion() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.sync(&api);
    api.0
        .borrow_mut()
        .associations
        .push(json!({"id":"file-foreign","vector_store_id":"vs_test","attributes":{}}));
    f.r.db
        .execute("UPDATE cloud_projects SET delete_requested=1,paused=1", [])
        .unwrap();
    f.step(&api);
    assert!(api.0.borrow().store.is_some());
    assert_eq!(api.0.borrow().associations.len(), 2);
}
#[test]
fn index_deletion_confirms_display_name_and_rejects_stale_or_missing_identity() {
    let p = json!({"store_name":"ContextDock - Project Ω", "store_id":"vs_original"});
    let payload = json!({"confirmation":"ContextDock - Project Ω", "storeId":"vs_original"});
    assert!(cloud_commands::validate_index_deletion(&p, &payload).is_ok());
    for invalid in [
        json!({"confirmation":"DELETE vs_original", "storeId":"vs_original"}),
        json!({"confirmation":"ContextDock - Project Ω", "storeId":"vs_other"}),
        json!({"confirmation":"ContextDock - Project Ω"}),
        json!({"confirmation":"ContextDock - Wrong project", "storeId":"vs_original"}),
    ] {
        assert!(cloud_commands::validate_index_deletion(&p, &invalid).is_err());
    }
    let renamed = json!({"store_name":"Renamed index", "store_id":"vs_original"});
    assert!(cloud_commands::validate_index_deletion(&renamed, &payload).is_err());
    let missing = json!({"store_name":"", "store_id":""});
    assert!(cloud_commands::validate_index_deletion(
        &missing,
        &json!({"confirmation":"", "storeId":""})
    )
    .is_err());
}
#[test]
fn delete_index_retains_files_and_local_history() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.sync(&api);
    f.r.db
        .execute("UPDATE cloud_projects SET delete_requested=1,paused=1", [])
        .unwrap();
    f.sync(&api);
    assert!(api.0.borrow().store.is_none());
    assert_eq!(api.0.borrow().files.len(), 1);
    assert_eq!(f.r.versions(&f.doc).unwrap().len(), 1);
    assert_eq!(
        s(&cloud::project(&f.r.db, &f.id).unwrap(), "status"),
        "deleted"
    );
}
#[test]
fn current_pointer_reuses_historical_identity() {
    let mut f = Fixture::new();
    f.edit();
    let oldest: String =
        f.r.db
            .query_row(
                "SELECT id FROM document_versions WHERE document_id=?1 ORDER BY number LIMIT 1",
                [&f.doc],
                |r| r.get(0),
            )
            .unwrap();
    f.r.db
        .execute(
            "UPDATE documents SET current_version_id=?2 WHERE id=?1",
            params![f.doc, oldest],
        )
        .unwrap();
    let desired = cloud::desired(&f.r.db, &f.id, &f.draft()).unwrap();
    assert_eq!(desired.len(), 1);
    assert_eq!(desired[0].number, 1);
}
#[test]
fn failed_processing_can_retry_attachment_without_reuploading_file() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    api.0.borrow_mut().processing = true;
    f.sync(&api);
    api.0.borrow_mut().associations[0]["status"] = json!("failed");
    f.r.db
        .execute("UPDATE cloud_jobs SET next_at=0", [])
        .unwrap();
    f.step(&api);
    assert_eq!(s(&f.jobs()[0], "state"), "failed");
    assert_eq!(s(&f.jobs()[0], "phase"), "retry_attach");
    api.0.borrow_mut().processing = false;
    f.r.db
        .execute("UPDATE cloud_projects SET next_at=0,attempts=0", [])
        .unwrap();
    f.r.db
        .execute("UPDATE cloud_jobs SET state='uploaded',next_at=0", [])
        .unwrap();
    f.sync(&api);
    assert_eq!(s(&f.jobs()[0], "state"), "indexed");
    assert_eq!(api.0.borrow().files.len(), 1);
    assert_eq!(api.0.borrow().associations.len(), 1);
}
#[test]
fn explicit_absent_operation_retry_refuses_existing_remote_copy() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.step(&api);
    f.r.db
        .execute(
            "UPDATE cloud_projects SET store_id=NULL,store_state='creating'",
            [],
        )
        .unwrap();
    let p = cloud::project(&f.r.db, &f.id).unwrap();
    let consent = json!({"confirmation":"RETRY ABSENT OPERATION","noOtherReferences":true});
    assert!(cloud_commands::retry_absent(&f.r.db, &p, &consent, &api).is_err());
    api.0.borrow_mut().store = None;
    cloud_commands::retry_absent(&f.r.db, &p, &consent, &api).unwrap();
    assert_eq!(
        s(&cloud::project(&f.r.db, &f.id).unwrap(), "store_state"),
        "pending"
    );
}
#[test]
fn deleted_remote_file_has_explicit_recovery_with_owned_index() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.sync(&api);
    api.0.borrow_mut().files.clear();
    api.0.borrow_mut().associations.clear();
    f.r.db
        .execute("UPDATE cloud_projects SET reconcile_requested=1", [])
        .unwrap();
    f.step(&api);
    let p = cloud::project(&f.r.db, &f.id).unwrap();
    let job = f.jobs().remove(0);
    cloud_commands::retry_absent(&f.r.db,&p,&json!({"confirmation":"RETRY ABSENT OPERATION","noOtherReferences":true,"version":job["version_id"]}),&api).unwrap();
    f.sync(&api);
    assert_eq!(api.0.borrow().files.len(), 1);
    assert_eq!(s(&f.jobs()[0], "state"), "indexed");
}
#[test]
fn explicit_file_purge_checks_references_hash_and_preserves_local_snapshot() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.sync(&api);
    let job = f.jobs().remove(0);
    cloud_commands::action_impl(
        &mut f.r.db,
        Some(&f.id),
        "remove",
        &json!({"version":job["version_id"],"confirm":true}),
    )
    .unwrap();
    f.sync(&api);
    let p = cloud::project(&f.r.db, &f.id).unwrap();
    let payload = json!({"version":job["version_id"],"confirmation":format!("DELETE {}",s(&job,"file_id")),"noOtherReferences":true});
    let original = api.0.borrow().files[0].1.clone();
    api.0.borrow_mut().files[0].1 = b"different bytes".to_vec();
    assert!(cloud_commands::purge_owned(&f.r.db, &p, &payload, &api).is_err());
    assert_eq!(api.0.borrow().files.len(), 1);
    api.0.borrow_mut().files[0].1 = original;
    cloud_commands::purge_owned(&f.r.db, &p, &payload, &api).unwrap();
    assert!(api.0.borrow().files.is_empty());
    assert_eq!(s(&f.jobs()[0], "state"), "purged");
    assert_eq!(f.r.versions(&f.doc).unwrap().len(), 1);
}
#[test]
fn ambiguity_freezes_existing_index_instead_of_silently_removing_selection() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.sync(&api);
    f.r.db
        .execute("UPDATE documents SET tracking_state='ignored'", [])
        .unwrap();
    f.r.db
        .execute("UPDATE cloud_projects SET sync_requested=1", [])
        .unwrap();
    f.step(&api);
    assert_eq!(api.0.borrow().associations.len(), 1);
    assert_eq!(
        s(&cloud::project(&f.r.db, &f.id).unwrap(), "status"),
        "needs_attention"
    );
}
#[test]
fn resumed_store_before_initial_start_becomes_idle_without_uploading() {
    let mut f = Fixture::new();
    f.apply(f.draft());
    f.r.db
        .execute("UPDATE cloud_projects SET paused=0,sync_requested=1", [])
        .unwrap();
    let api = Mock::default();
    f.step(&api);
    f.step(&api);
    assert!(api.0.borrow().files.is_empty());
    assert!(api.0.borrow().store.is_some());
    assert!(!cloud_commands::needed(&f.r.db, &cloud::project(&f.r.db, &f.id).unwrap()).unwrap());
}
#[test]
fn confirmed_stop_controls_block_queued_work_without_credentials_and_keep_history() {
    for action in ["disconnect", "stop_automatic"] {
        let mut f = Fixture::new();
        f.enable();
        let api = Mock::default();
        f.sync(&api);
        f.edit();
        cloud::queue(&mut f.r.db, &f.id).unwrap();
        f.r.db
            .execute(
                "UPDATE cloud_projects SET auto_sync=1,sync_requested=1,delete_requested=1",
                [],
            )
            .unwrap();
        f.r.db
            .execute("UPDATE cloud_connection SET status='disconnected'", [])
            .unwrap();
        let before = api.0.borrow().calls.len();
        cloud_commands::action_impl(
            &mut f.r.db,
            Some(&f.id),
            action,
            &json!({"retainCloud":true}),
        )
        .unwrap();
        let project = cloud::project(&f.r.db, &f.id).unwrap();
        assert_eq!(n(&project, "paused"), 1);
        assert_eq!(n(&project, "delete_requested"), 0);
        if action == "stop_automatic" {
            assert_eq!(n(&project, "auto_sync"), 1);
            assert_eq!(n(&project, "sync_requested"), 0);
            assert_eq!(n(&project, "enabled"), 1);
        } else {
            assert_eq!(n(&project, "enabled"), 0);
        }
        // Restoring key availability cannot bypass the user's persisted stop.
        f.r.db
            .execute("UPDATE cloud_connection SET status='connected'", [])
            .unwrap();
        let guard = cloud_commands::ProjectApi::new(
            &api,
            &f.r.store.join("test.db"),
            &f.id,
            "bound",
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        )
        .unwrap();
        assert!(guard.upload("blocked.md", b"not sent".to_vec()).is_err());
        f.sync(&api);
        assert_eq!(api.0.borrow().calls.len(), before);
        assert_eq!(api.0.borrow().associations.len(), 1);
        assert_eq!(api.0.borrow().files.len(), 1);
        assert_eq!(f.r.versions(&f.doc).unwrap().len(), 2);
        assert!(f.r.snapshot().unwrap().projects[0].tracking_enabled);
    }
}
#[test]
fn request_guard_stops_next_http_call_on_pause_disconnect_or_shutdown() {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    let stop = Arc::new(AtomicBool::new(false));
    let guard = cloud_commands::ProjectApi::new(
        &api,
        &f.r.store.join("test.db"),
        &f.id,
        "bound",
        stop.clone(),
    )
    .unwrap();
    guard
        .request("GET", "/vector_stores?limit=1", None)
        .unwrap();
    f.r.db
        .execute("UPDATE cloud_projects SET paused=1", [])
        .unwrap();
    assert!(guard
        .request("GET", "/vector_stores?limit=1", None)
        .is_err());
    assert!(guard.upload("not-sent.md", b"blocked".to_vec()).is_err());
    f.r.db
        .execute("UPDATE cloud_projects SET paused=0", [])
        .unwrap();
    f.r.db
        .execute("UPDATE cloud_connection SET status='disconnected'", [])
        .unwrap();
    assert!(guard
        .request("GET", "/vector_stores?limit=1", None)
        .is_err());
    f.r.db
        .execute("UPDATE cloud_connection SET status='connected'", [])
        .unwrap();
    stop.store(true, Ordering::Relaxed);
    assert!(guard
        .request("GET", "/vector_stores?limit=1", None)
        .is_err());
    assert_eq!(api.0.borrow().calls.len(), 1);
}
#[test]
fn processing_polls_are_bounded_without_reuploading_or_deleting_file() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    api.0.borrow_mut().processing = true;
    f.sync(&api);
    f.r.db
        .execute("UPDATE cloud_jobs SET poll_count=120,next_at=0", [])
        .unwrap();
    f.step(&api);
    assert_eq!(s(&f.jobs()[0], "state"), "failed");
    assert!(s(&f.jobs()[0], "error").contains("120 polls"));
    assert_eq!(api.0.borrow().files.len(), 1);
    assert_eq!(api.0.borrow().associations.len(), 1);
}
#[test]
fn explicit_association_cleanup_works_while_cloud_sync_is_disconnected() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.sync(&api);
    let job = f.jobs().remove(0);
    cloud_commands::action_impl(
        &mut f.r.db,
        Some(&f.id),
        "disconnect",
        &json!({"retainCloud":true}),
    )
    .unwrap();
    cloud_commands::action_impl(
        &mut f.r.db,
        Some(&f.id),
        "remove",
        &json!({"version":job["version_id"],"confirm":true}),
    )
    .unwrap();
    let p = cloud::project(&f.r.db, &f.id).unwrap();
    cloud_sync::remove_owned(&f.r.db, &p, &job, &api).unwrap();
    assert_eq!(n(&cloud::project(&f.r.db, &f.id).unwrap(), "enabled"), 0);
    assert!(api.0.borrow().associations.is_empty());
    assert_eq!(api.0.borrow().files.len(), 1);
    assert_eq!(f.r.versions(&f.doc).unwrap().len(), 1);
}
#[test]
fn search_waits_for_deselected_associations_to_be_removed() {
    let mut f = Fixture::new();
    f.enable();
    let api = Mock::default();
    f.sync(&api);
    let mut draft = f.draft();
    draft.documents.clear();
    f.apply(draft);
    assert!(cloud_sync::search(&f.r.db, &f.id, "old selected content", &api, "bound").is_err());
    assert!(api.0.borrow().calls.iter().all(|c| !c.ends_with("/search")));
}
#[test]
fn initial_empty_selection_is_rejected_without_enabling_project() {
    let mut f = Fixture::new();
    let mut draft = f.draft();
    draft.documents.clear();
    assert!(cloud::preview(&f.r.db, &f.id, &draft)
        .err()
        .unwrap()
        .contains("Select at least one"));
    assert_eq!(n(&cloud::project(&f.r.db, &f.id).unwrap(), "enabled"), 0);
    // Emptying an existing index remains an explicit supported selection change.
    f.enable();
    assert_eq!(cloud::preview(&f.r.db, &f.id, &draft).unwrap().files, 0);
}
#[test]
fn untested_connection_is_reported_before_consent_without_remote_requests() {
    let mut f = Fixture::new();
    f.r.db
        .execute("UPDATE cloud_connection SET status='untested'", [])
        .unwrap();
    let draft = f.draft();
    let result =
        cloud_commands::action_impl(&mut f.r.db, Some(&f.id), "preview", &json!({"draft":draft}));
    assert!(result.unwrap_err().contains("Test the saved OpenAI key"));
    let view = cloud::view(&f.r.db, &f.id).unwrap();
    assert_eq!(view["connectionStatus"], "untested");
    assert_eq!(view["enabled"], 0);
}

fn cleanup_confirm(f: &Fixture) -> Value {
    let p = cloud::project(&f.r.db, &f.id).unwrap();
    json!({"confirm":true,"storeId":p["store_id"],"confirmation":p["store_name"]})
}
#[test]
fn saved_path_removal_can_finish_on_retained_cloud_data_without_enabling_uploads() {
    use crate::path_selection::{self, Rule};
    let mut f = Fixture::new();
    let api = Mock::default();
    f.enable();
    f.sync(&api);
    cloud_commands::action_impl(
        &mut f.r.db,
        Some(&f.id),
        "disconnect",
        &json!({"retainCloud":true}),
    )
    .unwrap();
    let revision = path_selection::revision(&f.r.db, &f.id).unwrap();
    f.r.save_path_selection(
        &f.id,
        revision,
        vec![Rule {
            path: String::new(),
            kind: "folder".into(),
        }],
    )
    .unwrap();
    let p = cloud::project(&f.r.db, &f.id).unwrap();
    assert_eq!(p["enabled"], 0);
    assert!(cloud_commands::needed(&f.r.db, &p).unwrap());
    let guard = cloud_commands::ProjectApi::new(
        &api,
        &f.r.store.join("test.db"),
        &f.id,
        "bound",
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
    )
    .unwrap();
    let job = &cloud::selection_removals(&f.r.db, &f.id).unwrap()[0];
    assert!(guard
        .request(
            "GET",
            &format!(
                "/vector_stores/{}/files/{}",
                s(&p, "store_id"),
                s(job, "file_id")
            ),
            None
        )
        .is_ok());
    assert!(guard.upload("blocked.md", b"never sent".to_vec()).is_err());
    api.0.borrow_mut().ignore_association_delete = true;
    f.step(&api);
    assert_eq!(
        cloud::project(&f.r.db, &f.id).unwrap()["status"],
        "selection_removal_failed"
    );
    api.0.borrow_mut().ignore_association_delete = false;
    cloud_commands::action_impl(
        &mut f.r.db,
        Some(&f.id),
        "retry_selection_removals",
        &json!({}),
    )
    .unwrap();
    f.step(&api);
    assert!(api.0.borrow().associations.is_empty());
    assert_eq!(api.0.borrow().files.len(), 1);
    let p = cloud::project(&f.r.db, &f.id).unwrap();
    assert_eq!(p["enabled"], 0);
    assert_eq!(p["paused"], 1);
    assert!(!cloud_commands::needed(&f.r.db, &p).unwrap());
}
#[test]
fn saved_folder_exclusion_removes_from_index_while_paused_and_reinclude_reuses_owned_store() {
    use crate::path_selection::{self, Rule};
    let mut f = Fixture::new();
    let api = Mock::default();
    f.enable();
    f.sync(&api);
    fs::create_dir_all(f.root.join("Drafts/Nested")).unwrap();
    fs::write(f.root.join("Drafts/Nested/notes.md"), "nested notes").unwrap();
    f.r.reconcile(&f.id, "test", &HashSet::new(), true).unwrap();
    let nested =
        f.r.snapshot()
            .unwrap()
            .documents
            .iter()
            .find(|d| d.name == "notes.md")
            .unwrap()
            .document_id
            .clone();
    let mut draft = f.draft();
    draft.documents.push(nested.clone());
    f.apply(draft);
    f.sync(&api);
    let store = cloud::project(&f.r.db, &f.id).unwrap()["store_id"].clone();
    let uploaded = api.0.borrow().files.len();
    assert_eq!(api.0.borrow().associations.len(), 2);
    let guard = cloud_commands::ProjectApi::new(
        &api,
        &f.r.store.join("test.db"),
        &f.id,
        "bound",
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
    )
    .unwrap();
    f.r.db
        .execute("UPDATE cloud_projects SET paused=1,auto_sync=0", [])
        .unwrap();
    let revision = path_selection::revision(&f.r.db, &f.id).unwrap();
    f.r.save_path_selection(
        &f.id,
        revision,
        vec![Rule {
            path: "Drafts".into(),
            kind: "folder".into(),
        }],
    )
    .unwrap();
    assert_eq!(cloud::selection_removals(&f.r.db, &f.id).unwrap().len(), 1);
    // An old upload guard cannot continue with the pre-save selection.
    assert!(guard.upload("not-sent.md", b"not sent".to_vec()).is_err());
    let removals = cloud::selection_removals(&f.r.db, &f.id).unwrap();
    let current_guard = cloud_commands::ProjectApi::new(
        &api,
        &f.r.store.join("test.db"),
        &f.id,
        "bound",
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
    )
    .unwrap();
    let endpoint = format!(
        "/vector_stores/{}/files/{}",
        store.as_str().unwrap(),
        s(&removals[0], "file_id")
    );
    assert!(current_guard.request("GET", &endpoint, None).is_ok());
    api.0.borrow_mut().ignore_association_delete = true;
    f.step(&api);
    assert_eq!(
        cloud::project(&f.r.db, &f.id).unwrap()["status"],
        "selection_removal_failed"
    );
    assert_eq!(api.0.borrow().associations.len(), 2);
    api.0.borrow_mut().ignore_association_delete = false;
    cloud_commands::action_impl(
        &mut f.r.db,
        Some(&f.id),
        "retry_selection_removals",
        &json!({}),
    )
    .unwrap();
    f.step(&api);
    assert_eq!(api.0.borrow().associations.len(), 1);
    assert_eq!(api.0.borrow().files.len(), uploaded);
    assert_eq!(f.r.versions(&nested).unwrap().len(), 1);
    assert_eq!(cloud::project(&f.r.db, &f.id).unwrap()["paused"], 1);
    // The saved cloud selection is retained; re-inclusion uses its previous store/policy.
    let revision = path_selection::revision(&f.r.db, &f.id).unwrap();
    f.r.save_path_selection(&f.id, revision, vec![]).unwrap();
    f.r.db
        .execute("UPDATE cloud_projects SET paused=0", [])
        .unwrap();
    f.sync(&api);
    assert_eq!(api.0.borrow().associations.len(), 2);
    assert_eq!(api.0.borrow().files.len(), uploaded);
    assert_eq!(cloud::project(&f.r.db, &f.id).unwrap()["store_id"], store);
}
#[test]
fn path_exclusions_filter_historical_versions_and_block_questions_until_cloud_removal_verifies() {
    use crate::path_selection::{self, Rule};
    let mut f = Fixture::new();
    let api = Mock::default();
    f.enable();
    f.sync(&api);
    f.edit();
    let mut draft = f.draft();
    draft.mode = "history".into();
    f.apply(draft);
    f.sync(&api);
    assert_eq!(api.0.borrow().associations.len(), 2);
    let revision = path_selection::revision(&f.r.db, &f.id).unwrap();
    f.r.save_path_selection(
        &f.id,
        revision,
        vec![Rule {
            path: String::new(),
            kind: "folder".into(),
        }],
    )
    .unwrap();
    let p = cloud::project(&f.r.db, &f.id).unwrap();
    assert!(
        cloud::desired(&f.r.db, &f.id, &cloud::draft(&f.r.db, &p).unwrap())
            .unwrap()
            .is_empty()
    );
    assert!(crate::chat::authorize(&f.r.db, &f.id, "bound")
        .unwrap_err()
        .contains("exclusions"));
    assert_eq!(cloud::selection_removals(&f.r.db, &f.id).unwrap().len(), 2);
    f.sync(&api);
    assert!(api.0.borrow().associations.is_empty());
    assert_eq!(f.r.versions(&f.doc).unwrap().len(), 2);
    assert_eq!(api.0.borrow().files.len(), 2);
}
#[test]
fn project_removal_handles_missing_resources_without_downloading_assistants_files() {
    for absent in ["none", "store", "all"] {
        let mut f = Fixture::new();
        let api = Mock::default();
        f.enable();
        f.sync(&api);
        let original = fs::read(f.root.join("Master_Context.md")).unwrap();
        let chat = crate::chat::create(&f.r, &f.id).unwrap();
        {
            let mut remote = api.0.borrow_mut();
            remote.deny_file_content = true;
            if absent != "none" {
                remote.store = None;
                remote.associations.clear();
            }
            if absent == "all" {
                remote.files.clear();
            }
            remote.calls.clear();
        }
        cloud_cleanup::begin(&f.r.db, &f.id, &cleanup_confirm(&f)).unwrap();
        cloud_cleanup::run(&mut f.r.db, &f.id, &api, "bound").unwrap();
        f.r.remove_with_cloud_choice(&f.id, false).unwrap();
        assert!(api.0.borrow().store.is_none());
        assert!(api.0.borrow().files.is_empty());
        assert!(f.r.snapshot().unwrap().projects.is_empty());
        assert!(Registry::open(&f.r.store.join("test.db"))
            .unwrap()
            .snapshot()
            .unwrap()
            .projects
            .is_empty());
        assert_eq!(
            fs::read(f.root.join("Master_Context.md")).unwrap(),
            original
        );
        assert_eq!(
            cloud::rows(
                &f.r.db,
                "SELECT id FROM document_versions WHERE document_id=?1",
                &[&f.doc]
            )
            .unwrap()
            .len(),
            1
        );
        assert_eq!(
            cloud::rows(
                &f.r.db,
                "SELECT id FROM project_chats WHERE id=?1",
                &[&s(&chat, "id")]
            )
            .unwrap()
            .len(),
            1
        );
        if absent == "all" {
            assert!(!api.0.borrow().calls.iter().any(|c| c.starts_with("DELETE")));
        }
    }
}
#[test]
fn cleanup_handles_store_disappearing_between_inventory_and_reference_check() {
    let mut f = Fixture::new();
    let api = Mock::default();
    f.enable();
    f.sync(&api);
    api.0.borrow_mut().vanish_store_on_listing = true;
    cloud_cleanup::begin(&f.r.db, &f.id, &cleanup_confirm(&f)).unwrap();
    cloud_cleanup::run(&mut f.r.db, &f.id, &api, "bound").unwrap();
    assert!(api.0.borrow().store.is_none());
    assert!(api.0.borrow().files.is_empty());
    assert!(f.jobs().is_empty());
}
#[test]
fn cleanup_refuses_mismatched_file_identity_without_any_delete() {
    for (field, value) in [
        ("id", json!("file-foreign")),
        ("filename", json!("unrelated.md")),
        ("bytes", json!(9999)),
        ("purpose", json!("fine-tune")),
    ] {
        let mut f = Fixture::new();
        let api = Mock::default();
        f.enable();
        f.sync(&api);
        api.0.borrow_mut().files[0].0[field] = value;
        cloud_cleanup::begin(&f.r.db, &f.id, &cleanup_confirm(&f)).unwrap();
        assert!(cloud_cleanup::run(&mut f.r.db, &f.id, &api, "bound")
            .unwrap_err()
            .contains("ownership"));
        assert!(!api
            .0
            .borrow()
            .calls
            .iter()
            .any(|call| call.starts_with("DELETE")));
        assert!(!f.r.snapshot().unwrap().projects.is_empty());
    }
}
#[test]
fn explicit_cloud_cleanup_deletes_bucket_and_uploads_then_reenable_uploads_fresh() {
    let mut f = Fixture::new();
    let api = Mock::default();
    f.enable();
    f.sync(&api);
    let chat = crate::chat::create(&f.r, &f.id).unwrap();
    let old_file = f.jobs()[0]["file_id"].clone();
    cloud_cleanup::begin(&f.r.db, &f.id, &cleanup_confirm(&f)).unwrap();
    let result = cloud_cleanup::run(&mut f.r.db, &f.id, &api, "bound").unwrap();
    assert_eq!(
        result,
        json!({"storesDeleted":1,"filesDeleted":1,"verified":true})
    );
    assert!(api.0.borrow().store.is_none());
    assert!(api.0.borrow().files.is_empty());
    assert!(f.jobs().is_empty());
    assert!(!cloud::rows(
        &f.r.db,
        "SELECT id FROM document_versions WHERE document_id=?1",
        &[&f.doc]
    )
    .unwrap()
    .is_empty());
    assert!(crate::chat::get(&f.r, &f.id, s(&chat, "id")).is_ok());
    assert!(f.root.join("Master_Context.md").exists());
    f.enable();
    f.sync(&api);
    assert!(api.0.borrow().store.is_some());
    assert_eq!(api.0.borrow().files.len(), 1);
    assert_eq!(f.jobs()[0]["state"], "indexed");
    assert_eq!(
        api.0
            .borrow()
            .calls
            .iter()
            .filter(|c| *c == "UPLOAD")
            .count(),
        2
    );
    assert!(!old_file.is_null());
}
#[test]
fn acknowledged_but_unfinished_cloud_deletion_retains_references_and_can_retry() {
    let mut f = Fixture::new();
    let api = Mock::default();
    f.enable();
    f.sync(&api);
    api.0.borrow_mut().ignore_store_delete = true;
    cloud_cleanup::begin(&f.r.db, &f.id, &cleanup_confirm(&f)).unwrap();
    assert!(cloud_cleanup::run(&mut f.r.db, &f.id, &api, "bound")
        .unwrap_err()
        .contains("still reports"));
    let p = cloud::project(&f.r.db, &f.id).unwrap();
    assert_eq!(p["status"], "cleanup_failed");
    assert_eq!(p["paused"], 1);
    assert!(!p["store_id"].is_null());
    assert!(!f.jobs()[0]["file_id"].is_null());
    assert_eq!(api.0.borrow().files.len(), 1);
    assert!(cloud_commands::action_impl(&mut f.r.db, Some(&f.id), "resume", &json!({})).is_err());
    api.0.borrow_mut().ignore_store_delete = false;
    cloud_cleanup::begin(&f.r.db, &f.id, &cleanup_confirm(&f)).unwrap();
    cloud_cleanup::run(&mut f.r.db, &f.id, &api, "bound").unwrap();
    assert!(api.0.borrow().files.is_empty());
}
#[test]
fn cleanup_checks_shared_references_metadata_scope_and_confirmation_before_deleting() {
    let mut f = Fixture::new();
    let api = Mock::default();
    f.enable();
    f.sync(&api);
    assert!(cloud_cleanup::begin(&f.r.db, &f.id, &json!({"confirm":false})).is_err());
    assert_eq!(cloud::project(&f.r.db, &f.id).unwrap()["paused"], 0);
    cloud_cleanup::begin(&f.r.db, &f.id, &cleanup_confirm(&f)).unwrap();
    assert!(cloud_cleanup::run(&mut f.r.db, &f.id, &api, "another-key").is_err());
    api.0.borrow_mut().shared_file = true;
    cloud_cleanup::begin(&f.r.db, &f.id, &cleanup_confirm(&f)).unwrap();
    assert!(cloud_cleanup::run(&mut f.r.db, &f.id, &api, "bound")
        .unwrap_err()
        .contains("Another vector store"));
    assert!(!api.0.borrow().calls.iter().any(|c| c.starts_with("DELETE")));
    api.0.borrow_mut().shared_file = false;
    api.0.borrow_mut().files[0].0["bytes"] = json!(9999);
    cloud_cleanup::begin(&f.r.db, &f.id, &cleanup_confirm(&f)).unwrap();
    assert!(cloud_cleanup::run(&mut f.r.db, &f.id, &api, "bound")
        .unwrap_err()
        .contains("ownership"));
    assert!(!api.0.borrow().calls.iter().any(|c| c.starts_with("DELETE")));
}
#[test]
fn cleanup_recovers_lost_upload_identity_and_verifies_missing_bucket_before_reset() {
    let mut f = Fixture::new();
    let api = Mock::default();
    f.enable();
    f.sync(&api);
    f.r.db
        .execute(
            "UPDATE cloud_jobs SET file_id=NULL,state='uncertain' WHERE project_id=?1",
            [&f.id],
        )
        .unwrap();
    // The app lost the upload acknowledgement, but the original exact job filename is recoverable.
    cloud_cleanup::begin(&f.r.db, &f.id, &cleanup_confirm(&f)).unwrap();
    cloud_cleanup::run(&mut f.r.db, &f.id, &api, "bound").unwrap();
    assert!(api.0.borrow().files.is_empty());
    assert!(api.0.borrow().store.is_none());
}

#[test]
fn interrupted_file_deletion_is_reconciled_on_explicit_retry_without_losing_ids() {
    let mut f = Fixture::new();
    let api = Mock::default();
    f.enable();
    f.sync(&api);
    api.0.borrow_mut().lose_file_delete_reply = true;
    cloud_cleanup::begin(&f.r.db, &f.id, &cleanup_confirm(&f)).unwrap();
    assert!(cloud_cleanup::run(&mut f.r.db, &f.id, &api, "bound").is_err());
    assert!(api.0.borrow().store.is_none());
    assert!(api.0.borrow().files.is_empty());
    assert!(!f.jobs()[0]["file_id"].is_null());
    cloud_cleanup::begin(&f.r.db, &f.id, &cleanup_confirm(&f)).unwrap();
    cloud_cleanup::run(&mut f.r.db, &f.id, &api, "bound").unwrap();
    assert!(f.jobs().is_empty());
    assert_eq!(
        api.0
            .borrow()
            .calls
            .iter()
            .filter(|s| s.starts_with("DELETE /files/"))
            .count(),
        1
    );
}
#[test]
fn removal_requires_explicit_retention_or_verified_cloud_cleanup_and_keeps_local_history() {
    let mut f = Fixture::new();
    let api = Mock::default();
    f.enable();
    f.sync(&api);
    let source_path = f.root.join("Master_Context.md");
    let source_bytes = fs::read(&source_path).unwrap();
    let snapshot_path = f.r.versions(&f.doc).unwrap()[0].snapshot_path.clone();
    let snapshot_bytes = fs::read(&snapshot_path).unwrap();
    let archive_path = f.root.join("Old Context Files").join("retained.md");
    fs::create_dir_all(archive_path.parent().unwrap()).unwrap();
    fs::write(&archive_path, b"retained local archive").unwrap();
    let chat = crate::chat::create(&f.r, &f.id).unwrap();
    let chat_id = s(&chat, "id").to_string();
    f.r.db.execute(
        "INSERT INTO project_chat_turns(id,chat_id,question,answer,state,created_at) VALUES('retained-turn',?1,'Saved question','Saved answer','completed',?2)",
        params![chat_id, crate::models::now()],
    ).unwrap();
    let doc_id = f.doc.clone();
    let saved_versions = cloud::rows(
        &f.r.db,
        "SELECT * FROM document_versions WHERE document_id=?1",
        &[&doc_id],
    )
    .unwrap();
    let saved_chat = cloud::rows(
        &f.r.db,
        "SELECT * FROM project_chats WHERE id=?1",
        &[&chat_id],
    )
    .unwrap();
    let saved_turns = cloud::rows(
        &f.r.db,
        "SELECT * FROM project_chat_turns WHERE chat_id=?1",
        &[&chat_id],
    )
    .unwrap();
    let assert_local_preserved = |r: &Registry| {
        assert!(r.snapshot().unwrap().projects.is_empty());
        assert!(r.snapshot().unwrap().documents.is_empty());
        assert_eq!(fs::read(&source_path).unwrap(), source_bytes);
        assert_eq!(fs::read(&snapshot_path).unwrap(), snapshot_bytes);
        assert_eq!(fs::read(&archive_path).unwrap(), b"retained local archive");
        assert_eq!(
            cloud::rows(
                &r.db,
                "SELECT * FROM document_versions WHERE document_id=?1",
                &[&doc_id]
            )
            .unwrap(),
            saved_versions
        );
        assert_eq!(
            cloud::rows(
                &r.db,
                "SELECT * FROM project_chats WHERE id=?1",
                &[&chat_id]
            )
            .unwrap(),
            saved_chat
        );
        assert_eq!(
            cloud::rows(
                &r.db,
                "SELECT * FROM project_chat_turns WHERE chat_id=?1",
                &[&chat_id]
            )
            .unwrap(),
            saved_turns
        );
    };
    assert!(f.r.remove_with_cloud_choice(&f.id, false).is_err());
    assert!(
        cloud_commands::action_impl(&mut f.r.db, Some(&f.id), "disconnect", &json!({})).is_err()
    );
    f.r.remove_with_cloud_choice(&f.id, true).unwrap();
    assert_local_preserved(&f.r);
    assert!(f.r.snapshot().unwrap().projects.is_empty());
    assert!(api.0.borrow().store.is_some());
    assert_eq!(api.0.borrow().files.len(), 1);
    let restored = f.r.add("Restored", f.root.to_str().unwrap()).unwrap();
    assert_eq!(restored, f.id);
    assert!(!f.r.snapshot().unwrap().projects[0].tracking_enabled);
    cloud_cleanup::begin(&f.r.db, &f.id, &cleanup_confirm(&f)).unwrap();
    cloud_cleanup::run(&mut f.r.db, &f.id, &api, "bound").unwrap();
    f.r.remove_with_cloud_choice(&f.id, false).unwrap();
    assert_local_preserved(&f.r);
    assert_local_preserved(&Registry::open(&f.r.store.join("test.db")).unwrap());
    assert!(api.0.borrow().store.is_none());
    assert!(f.root.join("Master_Context.md").exists());
    assert_eq!(
        cloud::rows(
            &f.r.db,
            "SELECT id FROM document_versions WHERE document_id=?1",
            &[&f.doc]
        )
        .unwrap()
        .len(),
        1
    );
}
#[test]
fn cleanup_completion_rolls_back_if_saved_resource_references_do_not_clear() {
    let mut f = Fixture::new();
    let api = Mock::default();
    f.enable();
    f.sync(&api);
    f.r.db.execute_batch("CREATE TRIGGER ignore_cleanup AFTER UPDATE OF store_id ON cloud_projects WHEN NEW.store_id IS NULL BEGIN UPDATE cloud_projects SET store_id=OLD.store_id WHERE project_id=NEW.project_id; END;").unwrap();
    cloud_cleanup::begin(&f.r.db, &f.id, &cleanup_confirm(&f)).unwrap();
    assert!(cloud_cleanup::run(&mut f.r.db, &f.id, &api, "bound").is_err());
    assert!(!f.jobs().is_empty());
    assert!(!cloud::project(&f.r.db, &f.id).unwrap()["store_id"].is_null());
    f.r.db
        .execute_batch("DROP TRIGGER ignore_cleanup;")
        .unwrap();
    cloud_cleanup::begin(&f.r.db, &f.id, &cleanup_confirm(&f)).unwrap();
    cloud_cleanup::run(&mut f.r.db, &f.id, &api, "bound").unwrap();
}

#[test]
fn disconnect_key_requires_active_project_cleanup_before_any_vault_access() {
    let mut f = Fixture::new();
    f.enable();
    assert!(
        cloud_commands::action_impl(&mut f.r.db, None, "disconnect_key", &json!({}))
            .unwrap_err()
            .contains("Delete cloud data")
    );
    assert_eq!(
        cloud::rows(&f.r.db, "SELECT status FROM cloud_connection", &[]).unwrap()[0]["status"],
        "connected"
    );
}
