use crate::{
    drive_api::{ApiError, GoogleDrive},
    drive_auth::{self, NativeVault, Vault},
    drive_commands::DriveRuntime,
    drive_publish,
    registry::{error, Registry},
    Store,
};
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
};
use tauri::Emitter;

pub struct Publisher {
    pub path: PathBuf,
    pub gate: Arc<Mutex<()>>,
    pub stop: Arc<AtomicBool>,
    dirty: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Publisher {
    pub fn start(path: PathBuf, app: tauri::AppHandle) -> std::io::Result<Self> {
        let gate = Arc::new(Mutex::new(()));
        let stop = Arc::new(AtomicBool::new(false));
        let dirty = Arc::new(AtomicBool::new(true));
        let (worker_path, worker_gate, worker_stop, worker_dirty) =
            (path.clone(), gate.clone(), stop.clone(), dirty.clone());
        let worker = thread::Builder::new()
            .name("contextdock-drive".into())
            .spawn(move || {
                crate::drive_worker::run(worker_path, worker_gate, worker_stop, worker_dirty, app)
            })?;
        Ok(Self {
            path,
            gate,
            stop,
            dirty,
            worker: Some(worker),
        })
    }
    pub fn wake(&self) {
        self.dirty.store(true, Ordering::Release);
        if let Some(worker) = &self.worker {
            worker.thread().unpark();
        }
    }
    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::Release);
        self.wake();
    }
}
impl Drop for Publisher {
    fn drop(&mut self) {
        self.request_stop();
        self.worker.take();
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    enabled: bool,
    automatic: bool,
    recycle: bool,
    connected: bool,
    account_id: Option<String>,
    folder_id: Option<String>,
    root_ready: bool,
    error: Option<String>,
    documents: Vec<drive_publish::Publication>,
}
type SavedSettings = (bool, bool, bool, String, Option<String>, Option<String>);
#[tauri::command]
pub async fn drive_project_read(
    state: tauri::State<'_, Store>,
    project: String,
) -> Result<View, String> {
    crate::work(state.inner().clone(), move |registry| {
        registry.path(&project)?;
        let settings: Option<SavedSettings> = registry.db.query_row("SELECT enabled,automatic,recycle,account_id,folder_id,error FROM drive_projects WHERE project_id=?1",[&project],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional().map_err(error)?;
        let (enabled,automatic,recycle,account,folder_id,error)=settings.unwrap_or((false,false,false,String::new(),None,None));
        let connected=NativeVault.load()?.and_then(|s|s.session.as_ref().map(|s|s.account.id.clone())).is_some_and(|id|account.is_empty()||id==account);
        let root_ready=registry.db.query_row("SELECT root_ready FROM drive_accounts WHERE account_id=?1",[&account],|r|r.get(0)).optional().map_err(crate::registry::error)?.unwrap_or(false);
        let mut documents=drive_publish::publications(&registry.db,&project).map_err(|e|e.message)?;
        let blocked=crate::path_selection::blocked_documents(&registry.db,&project)?;
        let hidden=crate::cloud::rows(&registry.db,"SELECT id FROM documents WHERE project_id=?1 AND (merged_into IS NOT NULL OR tracking_state IN ('ignored','grouped_history'))",&[&project])?.into_iter().map(|d|crate::cloud::s(&d,"id").to_string()).collect::<std::collections::HashSet<_>>();
        documents.retain(|d|!blocked.contains(&d.document_id)&&!hidden.contains(&d.document_id));
        for document in &mut documents {
            if document.state=="synced" {
                match drive_publish::current(&registry.db,&project,&document.document_id) {
                    Ok(current) if document.verified_hash.as_deref()==Some(current.hash.as_str()) && document.verified_name.as_deref()==Some(current.name.as_str()) => {},
                    Ok(_)=>document.state="queued".into(),
                    Err(_)=>document.state="waiting_file".into(),
                }
            }
        }
        Ok(View {enabled,automatic,recycle,connected,account_id:(!account.is_empty()).then_some(account),folder_id,root_ready,error,documents})
    }).await
}
pub fn open(path: &std::path::Path) -> Result<Registry, String> {
    Ok(Registry {
        db: crate::cloud::connect(path)?,
        store: path
            .parent()
            .ok_or("Drive registry path is unavailable.")?
            .to_path_buf(),
    })
}
pub fn api(
    path: PathBuf,
    project: String,
    account: String,
    document: Option<(String, String)>,
    access: zeroize::Zeroizing<String>,
    stop: Arc<AtomicBool>,
) -> Result<GoogleDrive, ApiError> {
    let db = crate::cloud::connect(&path)?;
    GoogleDrive::new(access, move |write| {
        if stop.load(Ordering::Acquire) {
            return Err(ApiError {
                retry: true,
                waiting: true,
                ..ApiError::local("ContextDock is stopping. Publication remains queued.")
            });
        }
        let connected = NativeVault
            .load()?
            .and_then(|s| s.session.as_ref().map(|s| s.account.id.clone()));
        if connected.as_deref() != Some(account.as_str()) {
            return Err(ApiError {
                auth: true,
                ..ApiError::local("The Google Drive account disconnected or changed.")
            });
        }
        drive_publish::require_enabled(&db, &project, &account).map_err(|e| ApiError {
            retry: true,
            waiting: true,
            ..e
        })?;
        if write {
            if crate::desktop::uploads_paused(&db) {
                return Err(ApiError {
                    retry: true,
                    waiting: true,
                    delay: 30,
                    ..ApiError::local(
                        "Uploads are paused to save resources. Publication remains queued.",
                    )
                });
            }
            if let Some((document, generation)) = &document {
                if drive_publish::generation(&drive_publish::current(&db, &project, document)?)
                    != *generation
                {
                    return Err(ApiError {retry:true,waiting:true,..ApiError::local("The local document changed before upload. The latest generation remains queued.")});
                }
            }
        }
        Ok(())
    })
}
pub fn record_error(
    registry: &Registry,
    project: &str,
    document: Option<&str>,
    failure: &ApiError,
) -> Result<(), String> {
    crate::drive_scheduler::retry_error(
        registry,
        project,
        document,
        failure,
        chrono::Utc::now().timestamp(),
    )
}
#[tauri::command]
pub async fn drive_project_action(
    state: tauri::State<'_, Publisher>,
    auth: tauri::State<'_, DriveRuntime>,
    app: tauri::AppHandle,
    project: String,
    action: String,
    enabled: Option<bool>,
    automatic: Option<bool>,
) -> Result<(), String> {
    let path = state.path.clone();
    let gate = state.gate.clone();
    let auth_gate = auth.gate.clone();
    let id = project.clone();
    let stop = state.stop.clone();
    let result=tauri::async_runtime::spawn_blocking(move || {
        let registry=open(&path)?;
        if action=="configure" {
            let enabled=enabled.ok_or("Specify whether publication should be enabled.")?;
            let current=NativeVault.load()?.and_then(|s|s.session.as_ref().map(|s|s.account.id.clone()));
            let account=if enabled {current.ok_or("Connect Google Drive in Settings first.")?} else {
                registry.db.query_row("SELECT account_id FROM drive_projects WHERE project_id=?1",[&id],|r|r.get::<_,String>(0)).optional().map_err(error)?.or(current).ok_or("This project has no Drive publication settings.")?
            };
            drive_publish::configure(&registry.db,&id,&account,enabled,automatic.unwrap_or(false)).map_err(|e|e.message)?;
            if enabled {crate::drive_scheduler::reset(&registry,&id)?;}
            return Ok(());
        }
        if action!="publish" && action!="create_root" {return Err("Unknown Drive publication action.".into());}
        let _gate=gate.try_lock().map_err(|_|"Another Drive publication is running. Its latest state will appear here.")?;
        let (account,access)={
            let _auth=auth_gate.lock().map_err(|_|"Google connection unavailable.")?;
            let saved=NativeVault.load()?.ok_or("Connect Google Drive in Settings first.")?;
            let account=saved.session.as_ref().ok_or("Connect Google Drive in Settings first.")?.account.id.clone();
            let access=drive_auth::access(&NativeVault,&drive_auth::GoogleHttp::new()?,chrono::Utc::now().timestamp())?;
            (account,access)
        };
        drive_publish::require_enabled(&registry.db,&id,&account).map_err(|e|e.message)?;
        crate::drive_scheduler::reset(&registry,&id)?;
        let documents=drive_publish::eligible_documents(&registry.db,&id).map_err(|e|e.message)?;
        if documents.is_empty() && action=="publish" {return Err("No stable selected documents are ready. Check file tracking and folder selection.".into());}
        if action=="create_root" {
            registry.db.execute("UPDATE drive_accounts SET create_root=1 WHERE account_id=?1",[&account]).map_err(error)?;
            let allowed:bool=registry.db.query_row("SELECT create_root FROM drive_accounts WHERE account_id=?1",[&account],|r|r.get(0)).map_err(error)?;
            if !allowed {return Err("The Drive folder creation choice did not save. No folders were created.".into());}
        }
        let folders=api(path.clone(),id.clone(),account.clone(),None,zeroize::Zeroizing::new(access.to_string()),stop.clone()).map_err(|e|e.message)?;
        let destination=match drive_publish::ensure_destination(&registry.db,&id,&account,&folders) {
            Ok(parent)=>parent,
            Err(e)=>{record_error(&registry,&id,None,&e)?;return Err(e.message);}
        };
        if action=="create_root" {return Ok(());}
        let mut problems=vec![];
        for document in documents {
            let current=match drive_publish::current(&registry.db,&id,&document) {Ok(c)=>c,Err(e)=>{problems.push(e.message);continue;}};
            let client=api(path.clone(),id.clone(),account.clone(),Some((document.clone(),drive_publish::generation(&current))),zeroize::Zeroizing::new(access.to_string()),stop.clone()).map_err(|e|e.message)?;
            if let Err(e)=drive_publish::publish(&registry,&id,&account,&document,&destination,&client) {
                record_error(&registry,&id,Some(&document),&e)?;problems.push(e.message);
            }
        }
        if !problems.is_empty() {return Err(problems.join(" "));}
        let publications=drive_publish::publications(&registry.db,&id).map_err(|e|e.message)?;
        let pending=drive_publish::eligible_documents(&registry.db,&id).map_err(|e|e.message)?.iter().any(|document|publications.iter().all(|row|&row.document_id!=document||row.state!="synced"));
        registry.db.execute("UPDATE drive_projects SET requested=?2,attempts=0,next_at=0,error=NULL WHERE project_id=?1",params![id,pending]).map_err(error)?;
        Ok(())
    }).await.map_err(|_|"Drive publication task could not finish.".to_string())?;
    state.wake();
    let _ = app.emit("drive-changed", &project);
    result
}
