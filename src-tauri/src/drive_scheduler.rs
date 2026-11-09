//! Durable decisions only: idle checks never hash files or call Google.
use crate::{
    drive_api::{ApiError, Result},
    drive_publish,
    registry::{error, Registry},
};
use rusqlite::{params, OptionalExtension};

pub const SETTLE_MS: i64 = 3_000;
pub struct Due {
    pub project: String,
    pub account: String,
    pub documents: Vec<String>,
}
pub fn due(registry: &Registry, now_ms: i64) -> Result<Vec<Due>> {
    let db = &registry.db;
    let mut query=db.prepare("SELECT dp.project_id,dp.account_id,dp.automatic,dp.requested,dp.next_at,COALESCE(ds.blocked,''),da.root_id,da.create_root FROM drive_projects dp JOIN projects p ON p.id=dp.project_id JOIN drive_accounts da ON da.account_id=dp.account_id LEFT JOIN drive_schedule ds ON ds.project_id=dp.project_id WHERE dp.enabled=1 AND p.tracking_enabled=1 AND p.removed_at IS NULL AND (dp.automatic=1 OR dp.requested=1) ORDER BY dp.project_id").map_err(error)?;
    type Project = (
        String,
        String,
        bool,
        bool,
        i64,
        String,
        Option<String>,
        bool,
    );
    let projects = query
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
            ))
        })
        .map_err(error)?
        .collect::<std::result::Result<Vec<Project>, _>>()
        .map_err(error)?;
    let mut ready = vec![];
    for (project, account, automatic, requested, next_at, blocked, root, may_create) in projects {
        if !blocked.is_empty() || next_at > now_ms / 1000 || (root.is_none() && !may_create) {
            continue;
        }
        if drive_publish::require_enabled(db, &project, &account).is_err() {
            continue;
        }
        let documents = drive_publish::eligible_documents(db, &project)?;
        let publications = drive_publish::publications(db, &project)?;
        let mut signature = String::new();
        let mut needed = vec![];
        for document in documents {
            let current = drive_publish::current(db, &project, &document)?;
            signature.push_str(&drive_publish::generation(&current));
            let saved = publications.iter().find(|p| p.document_id == document);
            if (automatic || requested)
                && saved.is_none_or(|p| {
                    p.state != "synced"
                        || p.verified_hash.as_deref() != Some(current.hash.as_str())
                        || p.verified_name.as_deref() != Some(current.name.as_str())
                })
                && saved.is_none_or(|p| p.next_at <= now_ms / 1000 && p.state != "needs_attention")
            {
                needed.push(document);
            }
        }
        let signature = crate::cloud::scope(&signature);
        let previous: Option<(Option<String>, i64)> = db
            .query_row(
                "SELECT observed_signature,settle_until FROM drive_schedule WHERE project_id=?1",
                [&project],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(error)?;
        let unchanged = previous
            .as_ref()
            .is_some_and(|(old, _)| old.as_deref() == Some(signature.as_str()));
        if !unchanged {
            db.execute("INSERT INTO drive_schedule(project_id,observed_signature,settle_until) VALUES(?1,?2,?3) ON CONFLICT(project_id) DO UPDATE SET observed_signature=excluded.observed_signature,settle_until=excluded.settle_until",params![project,signature,now_ms+SETTLE_MS]).map_err(error)?;
            continue;
        }
        if previous.is_some_and(|(_, until)| until > now_ms) {
            continue;
        }
        if !needed.is_empty() {
            ready.push(Due {
                project,
                account,
                documents: needed,
            });
        }
    }
    Ok(ready)
}
pub fn retry_error(
    registry: &Registry,
    project: &str,
    document: Option<&str>,
    failure: &ApiError,
    now: i64,
) -> std::result::Result<(), String> {
    let attempts: i64 = if let Some(document) = document {
        registry
            .db
            .query_row(
                "SELECT attempts FROM drive_documents WHERE document_id=?1",
                [document],
                |r| r.get(0),
            )
            .optional()
            .map_err(error)?
            .unwrap_or(0)
    } else {
        registry
            .db
            .query_row(
                "SELECT attempts FROM drive_projects WHERE project_id=?1",
                [project],
                |r| r.get(0),
            )
            .map_err(error)?
    };
    let delay = (5u64.saturating_mul(1u64 << attempts.min(8) as u32))
        .max(failure.delay)
        .min(3600);
    let next = now + delay as i64;
    let block = if failure.auth {
        Some("auth")
    } else if !failure.retry && document.is_none() {
        Some("attention")
    } else {
        None
    };
    registry.db.execute("INSERT INTO drive_schedule(project_id,blocked) VALUES(?1,?2) ON CONFLICT(project_id) DO UPDATE SET blocked=excluded.blocked",params![project,block]).map_err(error)?;
    registry.db.execute("UPDATE drive_projects SET requested=1,attempts=attempts+1,next_at=?2,error=?3 WHERE project_id=?1",params![project,if document.is_none() || failure.auth {next}else{0},failure.message]).map_err(error)?;
    if let Some(document) = document {
        registry.db.execute("UPDATE drive_documents SET state=?2,attempts=attempts+1,next_at=?3,error=?4 WHERE document_id=?1",params![document,if failure.waiting {"waiting_file"}else if failure.retry && !failure.auth {"offline"}else{"needs_attention"},next,failure.message]).map_err(error)?;
    }
    Ok(())
}
pub fn reset(registry: &Registry, project: &str) -> std::result::Result<(), String> {
    registry
        .db
        .execute(
            "UPDATE drive_schedule SET blocked=NULL,settle_until=0 WHERE project_id=?1",
            [project],
        )
        .map_err(error)?;
    registry.db.execute("UPDATE drive_projects SET requested=1,next_at=0,attempts=0,error=NULL WHERE project_id=?1",[project]).map_err(error)?;
    registry.db.execute("UPDATE drive_documents SET next_at=0,attempts=0,error=NULL,state=CASE WHEN state='needs_attention' THEN 'queued' ELSE state END WHERE project_id=?1",[project]).map_err(error)?;
    Ok(())
}
pub fn resume_auth(registry: &Registry, account: &str) -> std::result::Result<(), String> {
    let tx = registry.db.unchecked_transaction().map_err(error)?;
    tx.execute("UPDATE drive_documents SET state=CASE WHEN state='needs_attention' THEN 'queued' ELSE state END,next_at=0,attempts=0,error=NULL WHERE project_id IN (SELECT dp.project_id FROM drive_projects dp JOIN drive_schedule ds ON ds.project_id=dp.project_id WHERE dp.account_id=?1 AND ds.blocked='auth')",[account]).map_err(error)?;
    tx.execute("UPDATE drive_projects SET next_at=0,attempts=0,error=NULL WHERE account_id=?1 AND project_id IN (SELECT project_id FROM drive_schedule WHERE blocked='auth')",[account]).map_err(error)?;
    tx.execute("UPDATE drive_schedule SET blocked=NULL,settle_until=0 WHERE blocked='auth' AND project_id IN (SELECT project_id FROM drive_projects WHERE account_id=?1)",[account]).map_err(error)?;
    tx.commit().map_err(error)?;
    Ok(())
}
