//! Disposable local files and scripted search/model output; no keys or network.
use crate::{
    chat,
    cloud::{self, s},
    cloud_api::{self, ApiError, CloudApi},
    cloud_models::CloudDraft,
    registry::Registry,
};
use serde_json::{json, Value};
use std::{cell::RefCell, fs};
struct Fixture {
    _temp: tempfile::TempDir,
    r: Registry,
    p: String,
    chat: String,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let app = temp.path().join("app");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&app).unwrap();
        fs::write(
            source.join("Master.md"),
            "# Project\nLOCAL-ONLY-COMPLETE-SNAPSHOT\nReact and SQLite.\n",
        )
        .unwrap();
        let mut r = Registry::open(&app.join("test.db")).unwrap();
        let p = r.add("Test project", source.to_str().unwrap()).unwrap();
        let ids = r.source_ids(&p).unwrap();
        r.configure_tracking(&p, true, false, ids).unwrap();
        r.db.execute(
            "UPDATE cloud_connection SET status='connected',scope='bound'",
            [],
        )
        .unwrap();
        let doc = r.snapshot().unwrap().documents[0].document_id.clone();
        let draft = CloudDraft {
            enabled: true,
            automatic: true,
            mode: "current".into(),
            documents: vec![doc],
            soft_limit: 52428800,
        };
        let preview = cloud::preview(&r.db, &p, &draft).unwrap();
        cloud::apply(&mut r.db, &p, &draft, &preview.token).unwrap();
        cloud::queue(&mut r.db, &p).unwrap();
        r.db.execute("UPDATE cloud_projects SET store_id='vs_chat',store_state='linked',started=1,paused=0 WHERE project_id=?1",[&p]).unwrap();
        r.db.execute("UPDATE cloud_jobs SET state='indexed',file_id='file-chat',remote_current=1 WHERE project_id=?1",[&p]).unwrap();
        let chat = s(&chat::create(&r, &p).unwrap(), "id").to_string();
        Self {
            _temp: temp,
            r,
            p,
            chat,
        }
    }
    fn api(&self) -> Mock {
        let p = cloud::project(&self.r.db, &self.p).unwrap();
        let job = cloud::jobs(&self.r.db, &self.p).unwrap().remove(0);
        let v = cloud::version(&self.r.db, s(&job, "version_id")).unwrap();
        Mock {
            store: json!({"metadata":{"contextdock":cloud::install(&self.r.db).unwrap(),"project":self.p,"operation":p["store_op"]}}),
            hit: json!({"file_id":job["file_id"],"attributes":cloud_api::attributes(&cloud::install(&self.r.db).unwrap(),&self.p,&v.document,&v.id,&v.hash,true),"score":0.9,"content":[{"type":"text","text":"React frontend and SQLite database."}]}),
            calls: RefCell::new(vec![]),
            stop_after_search: None,
            empty: false,
            bad_answer: false,
            timeout: false,
        }
    }
    fn ask(&self, api: &Mock, q: &str) -> Value {
        chat::ask(&self.r, &self.p, &self.chat, q, api, "bound").unwrap()
    }
}
struct Mock {
    store: Value,
    hit: Value,
    calls: RefCell<Vec<(String, Option<Value>)>>,
    stop_after_search: Option<std::path::PathBuf>,
    empty: bool,
    bad_answer: bool,
    timeout: bool,
}
impl CloudApi for Mock {
    fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> std::result::Result<Value, ApiError> {
        self.calls
            .borrow_mut()
            .push((format!("{method} {path}"), body));
        if path == "/vector_stores/vs_chat" {
            return Ok(self.store.clone());
        }
        if path == "/vector_stores/vs_chat/search" {
            if let Some(path) = &self.stop_after_search {
                let db = cloud::connect(path).unwrap();
                db.execute("UPDATE cloud_projects SET enabled=0,paused=1", [])
                    .unwrap();
            }
            return Ok(json!({"data":if self.empty {vec![]} else {vec![self.hit.clone()]}}));
        }
        assert_eq!(path, "/responses");
        if self.timeout {
            return Err(ApiError {
                message: "synthetic timeout".into(),
                ambiguous: true,
                retry: true,
                missing: false,
                delay: 0,
            });
        }
        Ok(
            json!({"status":"completed","usage":{"input_tokens":123,"output_tokens":30},"output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":json!({"answer":"It uses React and SQLite [1].","sources":if self.bad_answer {vec![99]} else {vec![1]}}).to_string()}]}]}),
        )
    }
    fn upload(&self, _: &str, _: Vec<u8>) -> std::result::Result<Value, ApiError> {
        panic!("Chat must not upload")
    }
    fn content(&self, _: &str) -> std::result::Result<Vec<u8>, ApiError> {
        panic!("Chat must not read remote full files")
    }
}
#[test]
fn chat_uses_current_vector_results_and_preserves_multiple_local_conversations() {
    let f = Fixture::new();
    let api = f.api();
    let saved = f.ask(&api, "What technology does this use?");
    assert_eq!(saved["turns"][0]["state"], "completed");
    assert_eq!(saved["turns"][0]["sources"][0]["familyName"], "Master");
    assert_eq!(saved["turns"][0]["sources"][0]["number"], 1);
    assert_eq!(saved["turns"][0]["sources"][0]["sourceName"], "Master.md");
    let calls = api.calls.borrow();
    assert_eq!(calls.len(), 3);
    let body = calls.last().unwrap().1.as_ref().unwrap();
    assert_eq!(body["store"], false);
    assert_eq!(body["background"], false);
    assert!(body.get("tools").is_none());
    assert!(body.get("previous_response_id").is_none());
    assert!(!body.to_string().contains("LOCAL-ONLY-COMPLETE-SNAPSHOT"));
    assert!(body.to_string().contains("React frontend and SQLite"));
    drop(calls);
    f.ask(&api, "And the database?");
    let calls = api.calls.borrow();
    let body = calls.last().unwrap().1.as_ref().unwrap();
    assert!(body.to_string().contains("What technology does this use?"));
    drop(calls);
    let second = chat::create(&f.r, &f.p).unwrap();
    assert_eq!(chat::list(&f.r, &f.p).unwrap().as_array().unwrap().len(), 2);
    chat::delete(&f.r, &f.p, &f.chat).unwrap();
    assert!(chat::get(&f.r, &f.p, &f.chat).is_err());
    assert!(chat::get(&f.r, &f.p, s(&second, "id")).is_ok());
    assert_eq!(
        f.r.db
            .query_row("SELECT COUNT(*) FROM project_chat_turns", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(cloud::jobs(&f.r.db, &f.p).unwrap().len(), 1);
    assert_eq!(f.r.snapshot().unwrap().documents.len(), 1);
}
#[test]
fn cloud_off_paused_unsynced_wrong_key_and_removed_projects_cannot_send() {
    for update in [
        "UPDATE cloud_projects SET enabled=0",
        "UPDATE cloud_projects SET paused=1",
        "UPDATE cloud_jobs SET state='pending'",
        "UPDATE cloud_connection SET scope='another'",
        "UPDATE projects SET removed_at='now'",
    ] {
        let f = Fixture::new();
        let api = f.api();
        f.r.db.execute(update, []).unwrap();
        assert!(chat::ask(&f.r, &f.p, &f.chat, "Question?", &api, "bound").is_err());
        assert!(api.calls.borrow().is_empty());
    }
}
#[test]
fn disabling_cloud_between_search_and_answer_blocks_generation() {
    let f = Fixture::new();
    let mut api = f.api();
    api.stop_after_search = Some(f.r.store.join("test.db"));
    let saved = f.ask(&api, "What technology?");
    assert_eq!(saved["turns"][0]["state"], "failed");
    assert!(!api
        .calls
        .borrow()
        .iter()
        .any(|c| c.0.ends_with("/responses")));
    assert!(s(&saved["turns"][0], "error").contains("Cloud sync is off"));
}
#[test]
fn foreign_search_and_answer_sources_are_rejected_without_paid_retry() {
    for foreign in [true, false] {
        let f = Fixture::new();
        let mut api = f.api();
        if foreign {
            api.hit["attributes"]["project"] = json!("other");
        } else {
            api.bad_answer = true;
        }
        let saved = f.ask(&api, "What technology?");
        assert_eq!(saved["turns"][0]["state"], "failed");
        assert!(saved["turns"][0]["answer"].is_null());
        assert_eq!(
            api.calls
                .borrow()
                .iter()
                .filter(|c| c.0.ends_with("/responses"))
                .count(),
            if foreign { 0 } else { 1 }
        );
    }
}
#[test]
fn no_results_save_an_explanation_without_a_model_request() {
    let f = Fixture::new();
    let mut api = f.api();
    api.empty = true;
    let saved = f.ask(&api, "Unknown subject?");
    assert_eq!(saved["turns"][0]["state"], "completed");
    assert!(s(&saved["turns"][0], "answer").contains("couldn't find"));
    assert!(!api
        .calls
        .borrow()
        .iter()
        .any(|c| c.0.ends_with("/responses")));
}
#[test]
fn timeout_and_restart_keep_history_without_automatic_replay() {
    let f = Fixture::new();
    let mut api = f.api();
    api.timeout = true;
    let saved = f.ask(&api, "What technology?");
    assert_eq!(saved["turns"][0]["state"], "uncertain");
    assert_eq!(
        api.calls
            .borrow()
            .iter()
            .filter(|c| c.0.ends_with("/responses"))
            .count(),
        1
    );
    f.r.db
        .execute("UPDATE project_chat_turns SET state='running'", [])
        .unwrap();
    chat::recover(&f.r.db).unwrap();
    assert_eq!(
        chat::get(&f.r, &f.p, &f.chat).unwrap()["turns"][0]["state"],
        "uncertain"
    );
    let reopened = Registry::open(&f.r.store.join("test.db")).unwrap();
    assert_eq!(
        chat::list(&reopened, &f.p)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn chat_access_is_project_scoped_and_saved_key_like_text_is_rejected() {
    let mut f = Fixture::new();
    let api = f.api();
    let other = f._temp.path().join("other");
    fs::create_dir(&other).unwrap();
    let p = f.r.add("Other", other.to_str().unwrap()).unwrap();
    assert!(chat::get(&f.r, &p, &f.chat).is_err());
    assert!(chat::delete(&f.r, &p, &f.chat).is_err());
    assert!(chat::ask(
        &f.r,
        &f.p,
        &f.chat,
        &format!("key {}{}", "sk-", "synthetic".repeat(5)),
        &api,
        "bound"
    )
    .is_err());
    assert!(api.calls.borrow().is_empty());
}
#[test]
fn chat_migration_preserves_cloud_mapping_snapshots_and_approved_baselines() {
    let mut f = Fixture::new();
    let baseline=crate::ai::approve(&mut f.r,&f.p,&json!({"approve":true,"title":"Original","content":{"purpose":"Offline MVP","goals":"","requirements":"","nonGoals":"","constraints":"","deliverables":"","principles":""}})).unwrap();
    f.r.db
        .execute_batch(
            "DROP TABLE project_chat_turns; DROP TABLE project_chats; ALTER TABLE projects DROP COLUMN folder_state; ALTER TABLE projects DROP COLUMN folder_identity; ALTER TABLE projects DROP COLUMN folder_revision; PRAGMA user_version=4;",
        )
        .unwrap();
    let r = Registry::open(&f.r.store.join("test.db")).unwrap();
    assert_eq!(
        s(&cloud::project(&r.db, &f.p).unwrap(), "store_id"),
        "vs_chat"
    );
    assert_eq!(
        s(&crate::ai::baseline(&r, &f.p, &baseline).unwrap(), "title"),
        "Original"
    );
    assert_eq!(chat::list(&r, &f.p).unwrap(), json!([]));
    assert_eq!(r.snapshot().unwrap().documents.len(), 1);
}
#[test]
fn history_index_still_filters_chat_to_current_versions() {
    let f = Fixture::new();
    f.r.db
        .execute("UPDATE cloud_projects SET mode='history'", [])
        .unwrap();
    let api = f.api();
    f.ask(&api, "What technology?");
    let calls = api.calls.borrow();
    let body = calls
        .iter()
        .find(|c| c.0.ends_with("/search"))
        .unwrap()
        .1
        .as_ref()
        .unwrap();
    assert!(body["filters"]["filters"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["key"] == "is_current" && f["value"] == true));
}
