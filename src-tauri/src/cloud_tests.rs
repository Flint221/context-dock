//! Entirely mocked provider. These tests never load credentials or use a network client.
use crate::{
    cloud::{self, n, s},
    cloud_api::{self, ApiError, CloudApi},
    cloud_commands,
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
            return Ok(json!({"data":r.store.iter().collect::<Vec<_>>(),"has_more":false}));
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
                r.store = None;
                return Ok(json!({"deleted":true}));
            }
            return r.store.clone().ok_or_else(missing);
        }
        if path.starts_with("/vector_stores/vs_test/files?") {
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
                    r.associations.remove(index);
                    Ok(json!({"deleted":true}))
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
    cloud_commands::action_impl(&mut f.r.db, Some(&f.id), "disconnect", &json!({})).unwrap();
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
    cloud_commands::action_impl(&mut f.r.db, Some(&f.id), "disconnect", &json!({})).unwrap();
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
