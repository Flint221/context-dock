//! Explicit IPC tasks only: no scheduler, automatic retry, or capture hooks.
use crate::{
    ai,
    ai_inference::{self, InferenceApi},
    cloud::{self, s},
    cloud_api::{ApiError, OpenAi},
    cloud_credentials,
    registry::{error, Registry, Result},
};
use rusqlite::params;
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
pub struct Runtime {
    pub(crate) path: PathBuf,
    pub(crate) gate: Arc<Mutex<()>>,
}
impl Runtime {
    pub fn new(path: PathBuf) -> Result<Self> {
        let path = std::fs::canonicalize(path).map_err(|_| "Cannot resolve analysis storage.")?;
        let db = cloud::connect(&path)?;
        crate::chat::recover(&db)?;
        db.execute("UPDATE ai_reports SET state='uncertain',error='Application closed during a potentially billable request. No automatic retry; inspect usage before generating again.' WHERE state IN ('running','cancel_requested')",[]).map_err(error)?;
        Ok(Self {
            path,
            gate: Arc::new(Mutex::new(())),
        })
    }
}
struct Guard<'a> {
    api: &'a OpenAi,
    credential: &'a str,
    db: rusqlite::Connection,
    project: String,
    id: String,
    scope: String,
}
impl InferenceApi for Guard<'_> {
    fn respond(&self, body: Value) -> std::result::Result<Value, ApiError> {
        if ai_inference::contains_credential(&body, self.credential) {
            return Err(ApiError::local("Selected evidence contains the saved API credential. Redact it in a newly captured version before analysis. No request was sent."));
        }
        let allowed:i64=self.db.query_row("SELECT COUNT(*) FROM ai_reports a JOIN projects p ON p.id=a.project_id JOIN cloud_projects c ON c.project_id=p.id JOIN cloud_connection k ON k.singleton=1 WHERE a.id=?1 AND a.project_id=?2 AND a.state='running' AND p.removed_at IS NULL AND k.status='connected' AND k.scope=?3 AND c.scope=?3 AND c.enabled=1 AND c.paused=0",params![self.id,self.project,self.scope],|v|v.get(0)).map_err(|_|ApiError::local("Cannot verify analysis authorization."))?;
        if allowed != 1 {
            return Err(ApiError::local(
                "Analysis cancelled or connection changed before sending. No request was sent.",
            ));
        }
        self.api.respond(body)
    }
}
#[tauri::command]
pub async fn ai_action(
    state: tauri::State<'_, Runtime>,
    project: String,
    action: String,
    payload: Value,
) -> Result<Value> {
    let path = state.path.clone();
    let gate = state.gate.clone();
    tauri::async_runtime::spawn_blocking(move||{
        let mut r=Registry{db:cloud::connect(&path)?,store:path.parent().ok_or("Invalid analysis storage.")?.to_path_buf()};
        ai::project(&r,&project)?;
        if action=="generate" {
            if payload["consent"]!=true {return Err("Confirm the paid analysis request first.".into());}
            let _permit=gate.try_lock().map_err(|_|"Another analysis is running. No request was sent; retry after it finishes.")?;
            let id=s(&payload,"id");
            // Cache reuse needs neither credentials nor an API call.
            let row=ai::report(&r,&project,id)?;
            if payload["fresh"]!=true {
                if let Some(cached)=cloud::rows(&r.db,"SELECT id FROM ai_reports WHERE project_id=?1 AND cache_key=?2 AND state='completed' ORDER BY completed_at DESC LIMIT 1",&[&project,&s(&row,"cache_key")])?.pop() {return ai::report(&r,&project,s(&cached,"id"));}
            }
            let connected:i64=r.db.query_row("SELECT COUNT(*) FROM cloud_connection WHERE status='connected'",[],|v|v.get(0)).map_err(error)?;
            let cloud=cloud::project(&r.db,&project)?;
            if crate::cloud::n(&cloud,"enabled")!=1 || crate::cloud::n(&cloud,"paused")!=0 {return Err("Turn on Cloud sync before generating AI. Saved reports remain available offline.".into());}
            if connected!=1 {return Err("A fresh AI analysis requires a tested OpenAI connection. Saved reports and baselines remain available offline.".into());}
            let key=cloud_credentials::read()?.ok_or("Save and test your OpenAI key in Settings first.")?;
            let api=OpenAi::for_analysis(&key)?;
            let guard=Guard{api:&api,credential:&key,db:cloud::connect(&path)?,project:project.clone(),id:id.into(),scope:cloud::scope(&key)};
            ai_inference::generate(&r,&project,id,true,payload["fresh"]==true,&guard)
        } else {local(&mut r,&project,&action,&payload)}
    }).await.map_err(|_|"Analysis task could not finish. Inspect saved request status before another paid request.".to_string())?
}
pub fn local(r: &mut Registry, p: &str, action: &str, payload: &Value) -> Result<Value> {
    ai::project(r, p)?;
    match action {
        "view" => ai::view(r, p),
        "source" => {
            let (source, text, warnings) = ai::source(r, p, s(payload, "version"))?;
            Ok(json!({"source":source,"text":text,"warnings":warnings}))
        }
        "approve" => Ok(json!(ai::approve(r, p, payload)?)),
        "prepare" => ai::prepare(
            r,
            p,
            serde_json::from_value(payload["request"].clone())
                .map_err(|_| "Invalid analysis selection.")?,
        ),
        "report" => ai::report(r, p, s(payload, "id")),
        "cancel" => {
            let id = s(payload, "id");
            ai::report(r, p, id)?;
            r.db.execute("UPDATE ai_reports SET state=CASE WHEN state='preview' THEN 'cancelled' ELSE 'cancel_requested' END WHERE id=?1 AND state IN ('preview','running')",[id]).map_err(error)?;
            Ok(Value::Null)
        }
        "decision" => {
            let id = s(payload, "id");
            let report = ai::report(r, p, id)?;
            let count = report["result"]["findings"]
                .as_array()
                .ok_or("Only completed report findings can be reviewed.")?
                .len();
            let finding = payload["finding"].as_u64().ok_or("Choose a finding.")?;
            let decision = s(payload, "decision");
            if report["state"] != "completed"
                || finding >= count as u64
                || ![
                    "reviewed",
                    "intentional",
                    "follow_up",
                    "dismissed",
                    "unreviewed",
                ]
                .contains(&decision)
            {
                return Err("Invalid finding review decision.".into());
            }
            r.db.execute("INSERT INTO ai_decisions VALUES(?1,?2,?3,?4) ON CONFLICT(report_id,finding) DO UPDATE SET decision=excluded.decision,updated_at=excluded.updated_at",params![id,finding as i64,decision,crate::models::now()]).map_err(error)?;
            Ok(Value::Null)
        }
        _ => Err("Unknown local analysis action.".into()),
    }
}
