use crate::{
    drive_api::ApiError,
    drive_auth::{self, NativeVault, Vault},
    drive_commands::DriveRuntime,
    drive_publication_commands::{self as commands, record_error},
    drive_publish, drive_scheduler,
    registry::Registry,
};
use rusqlite::OptionalExtension;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};

fn next_check(registry: &Registry, now: i64) -> Duration {
    let next:Option<i64>=registry.db.query_row("SELECT MIN(deadline) FROM (SELECT ds.settle_until deadline FROM drive_schedule ds JOIN drive_projects dp ON dp.project_id=ds.project_id WHERE dp.enabled=1 AND ds.blocked IS NULL AND (dp.automatic=1 OR dp.requested=1) UNION ALL SELECT dp.next_at*1000 FROM drive_projects dp LEFT JOIN drive_schedule ds ON ds.project_id=dp.project_id WHERE dp.enabled=1 AND ds.blocked IS NULL AND (dp.automatic=1 OR dp.requested=1) UNION ALL SELECT dd.next_at*1000 FROM drive_documents dd JOIN drive_projects dp ON dp.project_id=dd.project_id LEFT JOIN drive_schedule ds ON ds.project_id=dp.project_id WHERE dp.enabled=1 AND ds.blocked IS NULL AND dd.state!='needs_attention' AND (dp.automatic=1 OR dp.requested=1)) WHERE deadline>?1",[now],|r|r.get(0)).optional().ok().flatten().flatten();
    Duration::from_millis(
        next.map(|n| (n - now).clamp(100, 300_000) as u64)
            .unwrap_or(300_000),
    )
}
fn access(app: &tauri::AppHandle, account: &str) -> Result<zeroize::Zeroizing<String>, ApiError> {
    let runtime = app.state::<DriveRuntime>();
    let _gate = runtime
        .gate
        .lock()
        .map_err(|_| ApiError::local("Google connection unavailable."))?;
    let saved = NativeVault.load()?;
    if saved
        .as_ref()
        .and_then(|s| s.session.as_ref())
        .is_none_or(|s| s.account.id != account)
    {
        return Err(ApiError {
            auth: true,
            ..ApiError::local("Connect the project's Google Drive account to resume publication.")
        });
    }
    drive_auth::access(
        &NativeVault,
        &drive_auth::GoogleHttp::new()?,
        chrono::Utc::now().timestamp(),
    )
    .map_err(|message| {
        let retry =
            message.contains("could not be reached") || message.contains("temporarily unavailable");
        ApiError {
            retry,
            auth: !retry,
            ..ApiError::local(message)
        }
    })
}
enum Failure {
    Project(ApiError),
    Document(String, ApiError),
}
impl From<ApiError> for Failure {
    fn from(e: ApiError) -> Self {
        Self::Project(e)
    }
}
impl From<String> for Failure {
    fn from(e: String) -> Self {
        Self::Project(ApiError::from(e))
    }
}
fn one(
    registry: &Registry,
    job: &drive_scheduler::Due,
    app: &tauri::AppHandle,
    stop: Arc<AtomicBool>,
    path: &std::path::Path,
) -> Result<(), Failure> {
    let access = access(app, &job.account)?;
    let api = commands::api(
        path.to_path_buf(),
        job.project.clone(),
        job.account.clone(),
        None,
        zeroize::Zeroizing::new(access.to_string()),
        stop.clone(),
    )?;
    let parent = drive_publish::ensure_destination(&registry.db, &job.project, &job.account, &api)?;
    let Some(document) = job.documents.first() else {
        return Ok(());
    };
    let current = drive_publish::current(&registry.db, &job.project, document)
        .map_err(|e| Failure::Document(document.clone(), e))?;
    let api = commands::api(
        path.to_path_buf(),
        job.project.clone(),
        job.account.clone(),
        Some((document.clone(), drive_publish::generation(&current))),
        access,
        stop,
    )?;
    let result = drive_publish::publish(
        registry,
        &job.project,
        &job.account,
        document,
        &parent,
        &api,
    );
    result.map_err(|e| Failure::Document(document.clone(), e))?;
    crate::drive_recycle::after_publication(
        registry,
        &job.project,
        &job.account,
        document,
        &parent,
        &api,
    );
    let publications = drive_publish::publications(&registry.db, &job.project)?;
    let all_current = drive_publish::eligible_documents(&registry.db, &job.project)?
        .iter()
        .all(|id| {
            let Ok(c) = drive_publish::current(&registry.db, &job.project, id) else {
                return false;
            };
            publications.iter().any(|p| {
                p.document_id == *id
                    && p.state == "synced"
                    && p.verified_hash.as_deref() == Some(c.hash.as_str())
                    && p.verified_name.as_deref() == Some(c.name.as_str())
            })
        });
    if all_current {
        registry.db.execute("UPDATE drive_projects SET requested=0,attempts=0,next_at=0,error=NULL WHERE project_id=?1",[&job.project]).map_err(crate::registry::error)?;
    }
    Ok(())
}
pub fn run(
    path: PathBuf,
    gate: Arc<Mutex<()>>,
    stop: Arc<AtomicBool>,
    dirty: Arc<AtomicBool>,
    app: tauri::AppHandle,
) {
    let Ok(registry) = commands::open(&path) else {
        return;
    };
    let mut version = -1i64;
    let mut check_at = Instant::now();
    let mut cursor = 0usize;
    while !stop.load(Ordering::Acquire) {
        let observed = registry
            .db
            .query_row("PRAGMA data_version", [], |r| r.get::<_, i64>(0))
            .unwrap_or(-1);
        if dirty.swap(false, Ordering::AcqRel) || observed != version || Instant::now() >= check_at
        {
            version = observed;
            let now = chrono::Utc::now().timestamp_millis();
            if crate::desktop::uploads_paused(&registry.db) {
                check_at = Instant::now() + Duration::from_secs(30);
            } else if let Ok(jobs) = drive_scheduler::due(&registry, now) {
                if jobs.is_empty() {
                    check_at = Instant::now() + next_check(&registry, now);
                } else if let Ok(_guard) = gate.try_lock() {
                    let job = &jobs[cursor % jobs.len()];
                    cursor = cursor.wrapping_add(1);
                    if let Err(failure) = one(&registry, job, &app, stop.clone(), &path) {
                        match failure {
                            Failure::Project(error) => {
                                let _ = record_error(&registry, &job.project, None, &error);
                            }
                            Failure::Document(document, error) => {
                                let _ =
                                    record_error(&registry, &job.project, Some(&document), &error);
                            }
                        }
                    }
                    let _ = app.emit("drive-changed", &job.project);
                    check_at = Instant::now() + Duration::from_millis(250);
                } else {
                    check_at = Instant::now() + Duration::from_secs(1);
                }
            } else {
                check_at = Instant::now() + Duration::from_secs(5);
            }
        }
        if stop.load(Ordering::Acquire) {
            break;
        }
        thread::park_timeout(
            check_at
                .saturating_duration_since(Instant::now())
                .min(Duration::from_secs(5)),
        );
    }
}
