use crate::{
    cloud::{self, n, s},
    cloud_api::{self, CloudApi, OpenAi},
    cloud_cleanup, cloud_credentials,
    cloud_models::CloudDraft,
    cloud_sync,
    registry::{error, Result},
};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
};
use tauri::Emitter;
/// Recheck persisted authorization between individual outbound requests.
pub struct ProjectApi<'a> {
    api: &'a dyn CloudApi,
    db: Connection,
    project: String,
    scope: String,
    stop: Arc<AtomicBool>,
    cleanup: bool,
    folder_revision: i64,
    path_revision: i64,
}
impl<'a> ProjectApi<'a> {
    pub fn new(
        api: &'a dyn CloudApi,
        path: &std::path::Path,
        project: &str,
        scope: &str,
        stop: Arc<AtomicBool>,
    ) -> Result<Self> {
        let db = cloud::connect(path)?;
        let folder_revision = db
            .query_row(
                "SELECT folder_revision FROM projects WHERE id=?1",
                [project],
                |r| r.get(0),
            )
            .map_err(error)?;
        let path_revision = crate::path_selection::revision(&db, project)?;
        Ok(Self {
            api,
            db,
            project: project.into(),
            scope: scope.into(),
            stop,
            cleanup: false,
            folder_revision,
            path_revision,
        })
    }
    fn check(&self, method: &str, path: &str) -> std::result::Result<(), cloud_api::ApiError> {
        let removal = !self.cleanup
            && cloud::selection_removal_request(&self.db, &self.project, method, path)
                .map_err(|e| cloud_api::ApiError::local(&e))?;
        if !self.cleanup && !removal {
            if crate::path_selection::revision(&self.db, &self.project)
                .map_err(|e| cloud_api::ApiError::local(&e))?
                != self.path_revision
            {
                return Err(cloud_api::ApiError::local(
                    "Folder selection changed. No further upload or attachment was sent.",
                ));
            }
            crate::folder_recovery::require_ready(&self.db, &self.project)
                .map_err(|e| cloud_api::ApiError::local(&e))?;
            let revision: i64 = self
                .db
                .query_row(
                    "SELECT folder_revision FROM projects WHERE id=?1",
                    [&self.project],
                    |r| r.get(0),
                )
                .map_err(|_| {
                    cloud_api::ApiError::local("Cannot check the project folder revision.")
                })?;
            if revision != self.folder_revision {
                return Err(cloud_api::ApiError::local(
                    "The project folder changed. Refresh documents before syncing again.",
                ));
            }
        }
        let active:i64=self.db.query_row("SELECT COUNT(*) FROM cloud_projects c JOIN projects p ON p.id=c.project_id JOIN cloud_connection k ON k.singleton=1 WHERE c.project_id=?1 AND (c.enabled=1 OR (?4=1 AND c.paused=1)) AND p.removed_at IS NULL AND ((?3=0 AND c.status NOT IN ('cleanup_pending','cleanup_failed') AND (c.paused=0 OR c.delete_requested=1 OR ?4=1)) OR (?3=1 AND c.paused=1 AND c.status='cleanup_pending')) AND c.scope=?2 AND k.scope=?2 AND k.status='connected'",params![self.project,self.scope,self.cleanup as i64,removal as i64],|r|r.get(0)).map_err(|_|cloud_api::ApiError::local("Cloud authorization state could not be checked."))?;
        if active != 1 || self.stop.load(Ordering::Relaxed) {
            return Err(cloud_api::ApiError::local(
                "Cloud sync paused, disconnected or stopped before the next request.",
            ));
        }
        if !self.cleanup && !removal {
            let p = cloud::project(&self.db, &self.project)
                .map_err(|e| cloud_api::ApiError::local(&e))?;
            if n(&p, "delete_requested") != 1 {
                cloud::desired(
                    &self.db,
                    &self.project,
                    &cloud::draft(&self.db, &p).map_err(|e| cloud_api::ApiError::local(&e))?,
                )
                .map_err(|e| cloud_api::ApiError::local(&e))?;
            }
        }
        Ok(())
    }
}
impl CloudApi for ProjectApi<'_> {
    fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> std::result::Result<Value, cloud_api::ApiError> {
        self.check(method, path)?;
        if self.cleanup && !["GET", "DELETE"].contains(&method) {
            return Err(cloud_api::ApiError::local(
                "Cleanup cannot upload or create resources.",
            ));
        }
        self.api.request(method, path, body)
    }
    fn upload(
        &self,
        name: &str,
        bytes: Vec<u8>,
    ) -> std::result::Result<Value, cloud_api::ApiError> {
        self.check("POST", "/files")?;
        self.api.upload(name, bytes)
    }
    fn content(&self, id: &str) -> std::result::Result<Vec<u8>, cloud_api::ApiError> {
        self.check("GET", &format!("/files/{id}/content"))?;
        self.api.content(id)
    }
}
pub struct CloudRuntime {
    path: PathBuf,
    pub(crate) gate: Arc<Mutex<()>>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Drop for CloudRuntime {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // Do not hold application shutdown on an in-flight HTTP request. Every mutation
        // has already recorded its recovery intent; process exit can interrupt it safely.
        self.worker.take();
    }
}
fn current_api(db: &Connection) -> Result<(OpenAi, String)> {
    let key = cloud_credentials::read()?.ok_or("Save and test an API key in Settings first.")?;
    let scope = cloud::scope(&key);
    let connected:i64=db.query_row("SELECT COUNT(*) FROM cloud_connection WHERE singleton=1 AND scope=?1 AND status='connected'",[&scope],|r|r.get(0)).map_err(error)?;
    if connected != 1 {
        return Err("Test the saved key before using cloud indexing.".into());
    }
    Ok((OpenAi::new(&key)?, scope))
}
pub fn needed(db: &Connection, p: &Value) -> Result<bool> {
    if n(p, "next_at") > chrono::Utc::now().timestamp() {
        return Ok(false);
    }
    if n(p, "enabled") == 1 && n(p, "delete_requested") == 1 {
        return Ok(true);
    }
    let removals = cloud::selection_removals(db, s(p, "project_id"))?;
    if !removals.is_empty() && !["cleanup_pending", "cleanup_failed"].contains(&s(p, "status")) {
        return Ok(n(p, "enabled") == 1 || removals.iter().any(|j| s(j, "phase") == "path_remove"));
    }
    if n(p, "enabled") != 1 {
        return Ok(false);
    }
    if crate::folder_recovery::require_ready(db, s(p, "project_id")).is_err() {
        return Ok(false);
    }
    if n(p, "paused") == 1 {
        return Ok(false);
    }
    if s(p, "store_id").is_empty() || n(p, "reconcile_requested") == 1 {
        return Ok(true);
    }
    let jobs = cloud::jobs(db, s(p, "project_id"))?;
    if n(p, "sync_requested") == 1
        && !jobs.iter().any(|j| {
            s(j, "state") == "indexing" && n(j, "next_at") > chrono::Utc::now().timestamp()
        })
    {
        return Ok(true);
    }
    if jobs.iter().any(|j| {
        n(j, "next_at") <= chrono::Utc::now().timestamp()
            && !matches!(
                s(j, "state"),
                "indexed" | "failed" | "uncertain" | "removed" | "purged"
            )
    }) {
        return Ok(true);
    }
    if n(p, "auto_sync") == 1 && n(p, "started") == 1 {
        let wanted = match cloud::desired(db, s(p, "project_id"), &cloud::draft(db, p)?) {
            Ok(wanted) => wanted,
            Err(_) => return Ok(true),
        };
        if wanted.iter().any(|v| {
            !jobs.iter().any(|j| {
                s(j, "version_id") == v.id
                    && s(j, "state") == "indexed"
                    && j["remote_current"].as_i64().map(|x| x == 1) == Some(v.current)
            })
        }) || jobs.iter().any(|j| {
            s(j, "state") == "indexed" && !wanted.iter().any(|v| v.id == s(j, "version_id"))
        }) {
            return Ok(true);
        }
        // Periodic verification only for projects explicitly opted into automatic cloud traffic.
        if p["last_sync"]
            .as_str()
            .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
            .is_none_or(|t| chrono::Utc::now().timestamp() - t.timestamp() > 900)
        {
            db.execute(
                "UPDATE cloud_projects SET reconcile_requested=1 WHERE project_id=?1",
                [s(p, "project_id")],
            )
            .map_err(error)?;
            return Ok(true);
        }
    }
    Ok(false)
}
pub(crate) fn runtime_database_path(path: PathBuf) -> std::io::Result<PathBuf> {
    // Match Registry::store before deriving the guarded snapshot root. On Windows,
    // canonicalization adds the verbatim prefix even when the folder never moved.
    std::fs::canonicalize(path)
}
impl CloudRuntime {
    pub fn start(path: PathBuf, app: tauri::AppHandle) -> std::io::Result<Self> {
        let path = runtime_database_path(path)?;
        if let Ok(db) = cloud::connect(&path) {
            let _=db.execute("UPDATE cloud_projects SET reconcile_requested=1 WHERE enabled=1 AND paused=0 AND started=1",[]);
        }
        let gate = Arc::new(Mutex::new(()));
        let stop = Arc::new(AtomicBool::new(false));
        let (worker_path, worker_gate, worker_stop) = (path.clone(), gate.clone(), stop.clone());
        let worker=thread::Builder::new().name("contextdock-cloud".into()).spawn(move|| {
            let Ok(mut db)=cloud::connect(&worker_path) else{return;};
            let root=worker_path.parent().unwrap_or(std::path::Path::new(".")).to_path_buf();
            let mut cursor=0usize;
            while !worker_stop.load(Ordering::Relaxed) {
                if let Ok(_guard)=worker_gate.try_lock() {
                    if let Ok(projects)=cloud::rows(&db,"SELECT c.* FROM cloud_projects c JOIN projects p ON p.id=c.project_id WHERE p.removed_at IS NULL AND (c.enabled=1 OR EXISTS(SELECT 1 FROM cloud_jobs j WHERE j.project_id=c.project_id AND j.phase='path_remove' AND j.state NOT IN ('removed','purged')))",&[]) {
                        for offset in 0..projects.len() {
                            let index=(cursor+offset)%projects.len();let p=&projects[index];
                            if needed(&db,p).unwrap_or(false) {
                                if let Ok((api,scope))=current_api(&db) {
                                    if s(p,"scope")==scope {
                                        if let Ok(guarded)=ProjectApi::new(&api,&worker_path,s(p,"project_id"),&scope,worker_stop.clone()) {
                                            let _=cloud_sync::step(&mut db,&root,s(p,"project_id"),&guarded,&scope);
                                        }
                                        let _=app.emit("cloud-changed",s(p,"project_id"));
                                    }
                                }
                                cursor=index+1; break;
                            }
                        }
                    }
                }
                for _ in 0..20 {if worker_stop.load(Ordering::Relaxed){break;} thread::sleep(std::time::Duration::from_millis(100));}
            }
        })?;
        Ok(Self {
            path,
            gate,
            stop,
            worker: Some(worker),
        })
    }
}
#[tauri::command]
pub async fn cloud_read(
    state: tauri::State<'_, CloudRuntime>,
    project: Option<String>,
) -> Result<Value> {
    let path = state.path.clone();
    tauri::async_runtime::spawn_blocking(move|| {
        let db=cloud::connect(&path)?;
        if let Some(id)=project {return cloud::view(&db,&id);}
        let mut status=cloud::rows(&db,"SELECT status,tested_at,error FROM cloud_connection WHERE singleton=1",&[])?.pop().ok_or("Connection state unavailable.")?;
        status["saved"]=json!(cloud_credentials::read()?.is_some());
        status["usage"]=json!(cloud::rows(&db,"SELECT c.project_id,p.name,c.store_id,c.status,c.usage_bytes,(SELECT COALESCE(SUM(v.size),0) FROM cloud_jobs j JOIN document_versions v ON v.id=j.version_id WHERE j.project_id=c.project_id AND j.state='indexed') rawBytes FROM cloud_projects c JOIN projects p ON p.id=c.project_id WHERE c.store_id IS NOT NULL",&[])?);
        Ok(status)
    }).await.map_err(|_|"Cloud status task failed.".to_string())?
}
#[tauri::command]
pub fn cloud_link(url: String) -> Result<()> {
    if ![
        "https://platform.openai.com/usage",
        "https://platform.openai.com/settings/organization/billing/overview",
        "https://developers.openai.com/api/docs/pricing",
        "https://developers.openai.com/api/docs/guides/your-data",
        "https://platform.openai.com/storage/vector_stores",
        "https://platform.openai.com/api-keys",
    ]
    .contains(&url.as_str())
    {
        return Err("Unsupported cloud resource link.".into());
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};
        let operation = "open\0".encode_utf16().collect::<Vec<_>>();
        let target = url.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                operation.as_ptr(),
                target.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        if result as isize <= 32 {
            return Err("Could not open the default browser.".into());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err("Opening the platform dashboard requires Windows.".into())
    }
}
#[tauri::command]
pub async fn cloud_key(state: tauri::State<'_, CloudRuntime>, key: String) -> Result<()> {
    let key = cloud_credentials::validate(key)?;
    let path = state.path.clone();
    let gate = state.gate.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = gate.lock().map_err(|_| "Cloud worker unavailable.")?;
        let db = cloud::connect(&path)?;
        cloud_credentials::save(&key)?;
        db.execute("UPDATE cloud_projects SET status='credential_changed',error='Test the replacement key, then Verify key for each existing project index.' WHERE enabled=1",[]).map_err(error)?;
        db.execute(
            "UPDATE cloud_connection SET scope=?1,status='untested',tested_at=NULL,error=NULL",
            [cloud::scope(&key)],
        )
        .map_err(error)?;
        Ok(())
    })
    .await
    .map_err(|_| "Credential task failed.".to_string())?
}
#[tauri::command]
pub async fn cloud_action(
    state: tauri::State<'_, CloudRuntime>,
    project: Option<String>,
    action: String,
    payload: Value,
) -> Result<Value> {
    let path = state.path.clone();
    let gate = state.gate.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if ["delete_cloud", "delete_index"].contains(&action.as_str()) {
            let id = project.as_deref().ok_or("Select a project.")?;
            let mut db = cloud::connect(&path)?;
            if action == "delete_index" { validate_index_deletion(&cloud::project(&db,id)?, &payload)?; }
            cloud_cleanup::begin(&db, id, &payload)?;
            let generation = n(&cloud::project(&db,id)?, "revision");
            // Persist stop first, then drain any in-flight upload before inventory/deletion.
            let _guard = gate.lock().map_err(|_| "Cloud worker unavailable.")?;
            let result = (|| {
                let p = cloud::project(&db,id)?;
                if n(&p,"revision") != generation || s(&p,"status") != "cleanup_pending" {
                    return Err("Cloud state changed while waiting for uploads to stop. Review and retry deletion.".into());
                }
                if !cloud_cleanup::has_remote(&db,&p)? {
                    return cloud_cleanup::finish(&mut db,id,json!({"storesDeleted":0,"filesDeleted":0,"verified":true}));
                }
                let (api,scope) = current_api(&db)?;
                let mut guarded = ProjectApi::new(&api,&path,id,&scope,Arc::new(AtomicBool::new(false)))?;
                guarded.cleanup = true;
                cloud_cleanup::run(&mut db,id,&guarded,&scope)
            })();
            if let Err(message) = &result {
                db.execute("UPDATE cloud_projects SET status='cleanup_failed',error=?2 WHERE project_id=?1 AND revision=?3",params![id,format!("Cloud deletion incomplete. References retained for retry. {message}"),generation]).map_err(error)?;
            }
            return result;
        }
        let _guard = if ["pause", "disconnect", "disconnect_key", "stop_automatic"]
            .contains(&action.as_str())
        {
            None
        } else {
            Some(gate.lock().map_err(|_| "Cloud worker unavailable.")?)
        };
        let mut db = cloud::connect(&path)?;
        action_impl(&mut db, project.as_deref(), &action, &payload)
    })
    .await
    .map_err(|_| "Cloud operation failed.".to_string())?
}
pub(crate) fn validate_index_deletion(project: &Value, payload: &Value) -> Result<()> {
    let name = s(project, "store_name");
    let id = s(project, "store_id");
    if id.is_empty() || s(payload, "storeId") != id {
        return Err("The cloud index changed. Close this dialog and review deletion again.".into());
    }
    if name.trim().is_empty() || s(payload, "confirmation") != name {
        return Err("Type the exact cloud index name shown in the confirmation dialog.".into());
    }
    Ok(())
}
pub fn action_impl(
    db: &mut Connection,
    project: Option<&str>,
    action: &str,
    payload: &Value,
) -> Result<Value> {
    if action == "disconnect_key" {
        let active:i64=db.query_row("SELECT COUNT(*) FROM cloud_projects c JOIN projects p ON p.id=c.project_id WHERE p.removed_at IS NULL AND (c.store_id IS NOT NULL OR c.store_op IS NOT NULL OR EXISTS(SELECT 1 FROM cloud_jobs j WHERE j.project_id=c.project_id))",[],|r|r.get(0)).map_err(error)?;
        if active != 0 {
            return Err("Delete cloud data for your active projects before disconnecting OpenAI. Turn off each project's Cloud sync and complete cleanup first, or remove the project with an explicit keep-cloud-data choice.".into());
        }
        db.execute(
            "UPDATE cloud_connection SET status='disconnected',error=NULL",
            [],
        )
        .map_err(error)?;
        db.execute("UPDATE cloud_projects SET status='key_disconnected',error='Reconnect and test an API key to resume cloud operations.' WHERE enabled=1",[]).map_err(error)?;
        cloud_credentials::delete()?;
        if cloud_credentials::read()?.is_some() {
            return Err("OpenAI disconnect verification failed: the saved key is still present. Try Disconnect again.".into());
        }
        let status: String = db
            .query_row(
                "SELECT status FROM cloud_connection WHERE singleton=1",
                [],
                |r| r.get(0),
            )
            .map_err(error)?;
        if status != "disconnected" {
            return Err(
                "OpenAI disconnect verification failed: refresh the connection status.".into(),
            );
        }
        return Ok(Value::Null);
    }
    if action == "test" {
        let key = cloud_credentials::read()?.ok_or("No API key is saved.")?;
        let api = OpenAi::new(&key)?;
        let result = api.request("GET", "/vector_stores?limit=1", None);
        db.execute(
            "UPDATE cloud_connection SET status=?1,error=?2,tested_at=?3,scope=?4",
            params![
                if result.is_ok() { "connected" } else { "error" },
                result.as_ref().err().map(|e| e.message.clone()),
                models_now(),
                cloud::scope(&key)
            ],
        )
        .map_err(error)?;
        result.map_err(|e| e.message)?;
        return Ok(Value::Null);
    }
    let id = project.ok_or("Select a project.")?;
    let p = cloud::project(db, id)?;
    if ![
        "pause",
        "disconnect",
        "stop_automatic",
        "preview",
        "apply",
        "retry_selection_removals",
    ]
    .contains(&action)
    {
        crate::folder_recovery::require_ready(db, id)?;
    }
    if ["cleanup_pending", "cleanup_failed"].contains(&s(&p, "status")) {
        return Err("Finish or retry cloud deletion before changing sync settings. Remote references have been retained.".into());
    }
    match action {
        "preview" => {
            let draft: CloudDraft = serde_json::from_value(payload["draft"].clone())
                .map_err(|_| "Invalid cloud configuration.")?;
            if draft.enabled && !cloud::selection_reduction(db, &p, &draft)? {
                let connected: bool = db
                    .query_row(
                        "SELECT status='connected' FROM cloud_connection WHERE singleton=1",
                        [],
                        |r| r.get(0),
                    )
                    .map_err(error)?;
                if !connected {
                    return Err("Test the saved OpenAI key successfully in Settings or with Test saved OpenAI key before reviewing cloud changes. No configuration was saved.".into());
                }
            }
            Ok(serde_json::to_value(cloud::preview(db, id, &draft)?)
                .map_err(|_| "Invalid preview")?)
        }
        "apply" => {
            let draft: CloudDraft = serde_json::from_value(payload["draft"].clone())
                .map_err(|_| "Invalid cloud configuration.")?;
            if !draft.enabled && cloud_cleanup::has_remote(db, &p)? {
                return Err("Use Turn off Cloud sync to delete and verify remote data before disabling this project.".into());
            }
            if payload["consent"] != true {
                return Err("Explicit cloud upload and retention consent is required.".into());
            }
            cloud::apply(db, id, &draft, s(payload, "token"))?;
            Ok(Value::Null)
        }
        "retry_selection_removals" => {
            let removals = cloud::selection_removals(db, id)?;
            if n(&p, "enabled") != 1 && !removals.iter().any(|j| s(j, "phase") == "path_remove") {
                return Err("Cloud sync is off. Use confirmed project cloud cleanup to remove retained remote data.".into());
            }
            if cloud::selection_removals(db, id)?.is_empty() {
                return Err("No document opt-out removals need retry.".into());
            }
            db.execute("UPDATE cloud_projects SET next_at=0,attempts=0,error=NULL,status='selection_removing' WHERE project_id=?1",[id]).map_err(error)?;
            Ok(Value::Null)
        }
        "stop_automatic" => {
            db.execute("UPDATE cloud_projects SET auto_sync=0,paused=1,sync_requested=0,delete_requested=0,status='paused',revision=revision+1 WHERE project_id=?1", [id]).map_err(error)?;
            cloud::verify_flags(
                db,
                id,
                &[
                    ("auto_sync", 0),
                    ("paused", 1),
                    ("sync_requested", 0),
                    ("delete_requested", 0),
                ],
            )?;
            Ok(Value::Null)
        }
        "pause" | "disconnect" => {
            if action == "disconnect"
                && payload["retainCloud"] != true
                && cloud_cleanup::has_remote(db, &p)?
            {
                return Err("Turn off Cloud sync using confirmed cloud deletion. Retaining cloud data requires an explicit retention choice.".into());
            }
            db.execute("UPDATE cloud_projects SET paused=1,delete_requested=CASE WHEN ?2='disconnect' THEN 0 ELSE delete_requested END,enabled=CASE WHEN ?2='disconnect' THEN 0 ELSE enabled END,status=?2,revision=revision+1 WHERE project_id=?1",params![id,action]).map_err(error)?;
            cloud::verify_flags(
                db,
                id,
                if action == "disconnect" {
                    &[("enabled", 0), ("paused", 1), ("delete_requested", 0)]
                } else {
                    &[("paused", 1)]
                },
            )?;
            Ok(Value::Null)
        }
        "start" | "sync" | "resume" | "reconcile" => {
            if n(&p, "delete_requested") == 1 && action != "reconcile" {
                return Err("Project index deletion is pending. Wait for completion or Disconnect local cloud sync to stop future requests.".into());
            }
            if n(&p, "enabled") != 1 {
                return Err("Enable cloud indexing with a reviewed selection first.".into());
            }
            let (api, bound) = current_api(db)?;
            if s(&p, "scope") != bound {
                if s(&p, "store_id").is_empty() {
                    return Err("Reconnect the original credential before resolving an interrupted index creation.".into());
                }
                cloud::owned_store(&api, db, &p).map_err(|e| e.message)?;
                db.execute(
                    "UPDATE cloud_projects SET scope=?2 WHERE project_id=?1",
                    params![id, bound],
                )
                .map_err(error)?;
            }
            db.execute("UPDATE cloud_projects SET paused=CASE WHEN delete_requested=1 THEN 1 ELSE 0 END,started=CASE WHEN ?2='start' THEN 1 ELSE started END,sync_requested=CASE WHEN ?2 IN ('start','sync','resume') THEN 1 ELSE sync_requested END,reconcile_requested=CASE WHEN store_id IS NOT NULL THEN 1 ELSE 0 END,next_at=0,attempts=0,error=NULL,status='queued' WHERE project_id=?1",params![id,action]).map_err(error)?;
            // Preserve the irreversible phase. An uncertain upload is reconciled, never reposted.
            db.execute("UPDATE cloud_jobs SET state=CASE WHEN file_id IS NOT NULL THEN 'uploaded' WHEN phase='recover' OR state='uncertain' OR state='uploading' THEN 'uploading' ELSE 'pending' END,next_at=0,attempts=0,poll_count=0,error=NULL WHERE project_id=?1 AND state IN ('failed','uncertain','uploading')",[id]).map_err(error)?;
            cloud::activity(
                db,
                id,
                "request",
                "User requested sync/resume/reconciliation.",
            )?;
            if action != "reconcile" {
                cloud::verify_flags(
                    db,
                    id,
                    if action == "start" {
                        &[("enabled", 1), ("paused", 0), ("started", 1)]
                    } else {
                        &[("enabled", 1), ("paused", 0)]
                    },
                )?;
            }
            Ok(Value::Null)
        }
        "search" => {
            let (api, bound) = current_api(db)?;
            cloud_sync::search(db, id, s(payload, "query"), &api, &bound)
        }
        "remove_now" => {
            let (api, bound) = current_api(db)?;
            if s(&p, "scope") != bound {
                return Err(
                    "Verify this index with the saved key before deleting an association.".into(),
                );
            }
            action_impl(db, Some(id), "remove", payload)?;
            let jobs = cloud::jobs(db, id)?;
            let job = jobs
                .iter()
                .find(|j| s(j, "version_id") == s(payload, "version"))
                .ok_or("Owned version not found.")?;
            let result = cloud_sync::remove_owned(db, &p, job, &api);
            if let Err(message) = &result {
                db.execute(
                    "UPDATE cloud_jobs SET error=?2 WHERE id=?1",
                    params![s(job, "id"), message],
                )
                .map_err(error)?;
            }
            result?;
            Ok(Value::Null)
        }
        "remove" => {
            if payload["confirm"] != true {
                return Err("Confirm removal of the remote association.".into());
            }
            let version = s(payload, "version");
            let job = cloud::jobs(db, id)?
                .into_iter()
                .find(|j| s(j, "version_id") == version)
                .ok_or("Version is not owned by this project's cloud index.")?;
            if s(&job, "file_id").is_empty() {
                return Err("No known remote association for this version.".into());
            }
            let tx = db.transaction().map_err(error)?;
            tx.execute(
                "INSERT OR IGNORE INTO cloud_exclusions VALUES(?1,?2)",
                params![id, version],
            )
            .map_err(error)?;
            tx.execute(
                "UPDATE cloud_jobs SET desired=0,phase='remove',state='removing',next_at=0 WHERE id=?1",
                [s(&job, "id")],
            )
            .map_err(error)?;
            tx.execute(
                "UPDATE cloud_projects SET sync_requested=1,next_at=0 WHERE project_id=?1",
                [id],
            )
            .map_err(error)?;
            tx.commit().map_err(error)?;
            Ok(Value::Null)
        }
        "delete_index" | "delete_cloud" => {
            Err("Cloud deletion must run through the guarded cleanup command.".into())
        }
        "purge_file" => purge(db, &p, payload),
        "retry_absent" => {
            let (api, bound) = current_api(db)?;
            if n(&p, "enabled") != 1 || s(&p, "scope") != bound {
                return Err("Enable this project and verify the original credential first.".into());
            }
            retry_absent(db, &p, payload, &api)?;
            Ok(Value::Null)
        }
        "new_index" => {
            if s(&p, "store_state") != "deleted" || payload["confirm"] != true {
                return Err(
                    "Delete or reconcile the missing index, then confirm creating a replacement."
                        .into(),
                );
            }
            db.execute("UPDATE cloud_projects SET store_id=NULL,store_op=NULL,store_state='none',started=0,enabled=0,paused=1,next_at=0,status='local_only',revision=revision+1 WHERE project_id=?1",[id]).map_err(error)?;
            Ok(Value::Null)
        }
        _ => Err("Unknown cloud operation.".into()),
    }
}
fn models_now() -> String {
    crate::models::now()
}
pub fn retry_absent(db: &Connection, p: &Value, payload: &Value, api: &dyn CloudApi) -> Result<()> {
    if s(payload, "confirmation") != "RETRY ABSENT OPERATION"
        || payload["noOtherReferences"] != true
    {
        return Err("Confirm the previous request has ended and no remote copy exists in the correct OpenAI account.".into());
    }
    let pid = s(p, "project_id");
    if s(payload, "version").is_empty() {
        if !s(p, "store_id").is_empty() || s(p, "store_state") != "creating" {
            return Err("Only an uncertain index creation can be reset.".into());
        }
        let stores = cloud_api::list(api, "/vector_stores").map_err(|e| e.message)?;
        let install = cloud::install(db)?;
        if stores.iter().any(|v| {
            v["metadata"]["contextdock"] == install
                && v["metadata"]["project"] == pid
                && v["metadata"]["operation"] == p["store_op"]
        }) {
            return Err("A matching index exists. Use Reconcile to recover it instead.".into());
        }
        db.execute(
            "UPDATE cloud_projects SET store_state='pending' WHERE project_id=?1",
            [pid],
        )
        .map_err(error)?;
    } else {
        cloud::owned_store(api, db, p).map_err(|e| e.message)?;
        let jobs = cloud::jobs(db, pid)?;
        let job = jobs
            .iter()
            .find(|j| {
                s(j, "version_id") == s(payload, "version")
                    && matches!(
                        s(j, "state"),
                        "failed" | "uncertain" | "uploading" | "uploaded"
                    )
            })
            .ok_or("Only a failed or uncertain upload can be reset.")?;
        let file = s(job, "file_id");
        if !file.is_empty() {
            match api.request("GET", &format!("/files/{file}"), None) {
                Err(e) if e.missing => {}
                Err(e) => return Err(e.message),
                Ok(_) => {
                    return Err(
                        "The uploaded File still exists. Retry/Reconcile without a new upload."
                            .into(),
                    )
                }
            }
            match api.request(
                "GET",
                &format!("/vector_stores/{}/files/{file}", s(p, "store_id")),
                None,
            ) {
                Err(e) if e.missing => {}
                Err(e) => return Err(e.message),
                Ok(_) => {
                    return Err(
                        "An association still exists. Reconcile it before proceeding.".into(),
                    )
                }
            }
        } else if s(job, "phase") != "recover" {
            return Err("This upload has no ambiguous outcome. Use Retry/Reconcile.".into());
        }
        let files = cloud_api::list(api, "/files?purpose=assistants").map_err(|e| e.message)?;
        if files
            .iter()
            .any(|v| s(v, "filename") == s(job, "remote_name"))
        {
            return Err("A matching upload exists. Use Reconcile to recover it instead.".into());
        }
        db.execute("UPDATE cloud_jobs SET file_id=NULL,state='pending',phase='upload',attempts=0,next_at=0,error=NULL WHERE id=?1",[s(job,"id")]).map_err(error)?;
    }
    db.execute("UPDATE cloud_projects SET paused=0,sync_requested=1,next_at=0,attempts=0,error=NULL,status='queued' WHERE project_id=?1",[pid]).map_err(error)?;
    cloud::activity(db,pid,"recovery","User verified absence after the previous request ended, and explicitly authorized retrying a missing/uncertain operation.")
}
fn purge(db: &Connection, p: &Value, payload: &Value) -> Result<Value> {
    let (api, bound) = current_api(db)?;
    if s(p, "scope") != bound {
        return Err("Verify this project with the saved key first.".into());
    }
    purge_owned(db, p, payload, &api)
}
pub fn purge_owned(
    db: &Connection,
    p: &Value,
    payload: &Value,
    api: &dyn CloudApi,
) -> Result<Value> {
    let jobs = cloud::jobs(db, s(p, "project_id"))?;
    let job = jobs
        .iter()
        .find(|j| s(j, "version_id") == s(payload, "version") && s(j, "state") == "removed")
        .ok_or("Detach this owned version before deleting its uploaded File.")?;
    let file = s(job, "file_id");
    cloud_api::valid_id(file, "file").map_err(|e| e.message)?;
    if s(payload, "confirmation") != format!("DELETE {file}")
        || payload["noOtherReferences"] != true
    {
        return Err(
            "Confirm the File ID and that you have verified no other resource uses this File."
                .into(),
        );
    }
    for store in cloud_api::list(api, "/vector_stores").map_err(|e| e.message)? {
        cloud_api::valid_id(s(&store, "id"), "vs_").map_err(|e| e.message)?;
        let associations =
            cloud_api::list(api, &format!("/vector_stores/{}/files", s(&store, "id")))
                .map_err(|e| e.message)?;
        if associations.iter().any(|a| s(a, "id") == file) {
            return Err(
                "Another vector store still references this File. Nothing was deleted.".into(),
            );
        }
    }
    match api.request("GET", &format!("/files/{file}"), None) {
        Ok(remote) => {
            if s(&remote, "filename") != s(job, "remote_name")
                || n(&remote, "bytes") != n(job, "rawBytes")
            {
                return Err("File ownership cannot be established.".into());
            }
            let bytes = api.content(file).map_err(|e| e.message)?;
            let version = cloud::version(db, s(job, "version_id"))?;
            use sha2::{Digest, Sha256};
            if format!("{:x}", Sha256::digest(bytes)) != version.hash {
                return Err("Remote File integrity mismatch; deletion refused.".into());
            }
            api.request("DELETE", &format!("/files/{file}"), None)
                .map_err(|e| e.message)?;
        }
        Err(e) if e.missing => {}
        Err(e) => return Err(e.message),
    }
    db.execute(
        "UPDATE cloud_jobs SET state='purged',file_id=NULL WHERE id=?1",
        [s(job, "id")],
    )
    .map_err(error)?;
    cloud::activity(db,s(p,"project_id"),"deletion","Owned uploaded File explicitly deleted after checking accessible stores and user-confirmed external references.")?;
    Ok(Value::Null)
}
