use crate::{
    cloud::{self, n, s},
    cloud_api::{self, ApiError, CloudApi},
    models,
    registry::{error, Result},
};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
type ApiResult<T> = std::result::Result<T, ApiError>;
fn local(e: String) -> ApiError {
    ApiError::local(&e)
}
fn execute(db: &Connection, sql: &str, args: &[&dyn rusqlite::ToSql]) -> ApiResult<()> {
    db.execute(sql, args)
        .map(|_| ())
        .map_err(error)
        .map_err(local)
}
fn log(db: &Connection, id: &str, message: &str) -> ApiResult<()> {
    cloud::activity(db, id, "sync", message).map_err(local)
}
fn metadata(db: &Connection, p: &Value) -> ApiResult<Value> {
    Ok(
        json!({"contextdock":cloud::install(db).map_err(local)?,"project":p["project_id"],"operation":p["store_op"]}),
    )
}
fn identity(db: &Connection, p: &Value, j: &Value) -> ApiResult<Value> {
    let v = cloud::version(db, s(j, "version_id")).map_err(local)?;
    let mut attrs = cloud_api::attributes(
        &cloud::install(db).map_err(local)?,
        s(p, "project_id"),
        &v.document,
        &v.id,
        &v.hash,
        v.current,
    );
    attrs["version_number"] = json!(v.number);
    Ok(attrs)
}
fn owns_association(remote: &Value, attrs: &Value, store: &str) -> ApiResult<()> {
    if remote["vector_store_id"] != store
        || ["contextdock", "project", "document", "version", "sha256"]
            .iter()
            .any(|k| remote["attributes"][k] != attrs[k])
    {
        return Err(ApiError::local(
            "Index contains a foreign or mismatched file. No destructive changes were made.",
        ));
    }
    Ok(())
}
// Creation/list responses can omit optional attributes while indexing settles.
// Missing evidence is pending, never permission to search, replace or delete.
fn association_verified(remote: &Value, attrs: &Value, store: &str, file: &str) -> ApiResult<bool> {
    for (field, expected) in [("id", file), ("vector_store_id", store)] {
        if remote[field] != expected {
            return Err(ApiError::local(&format!(
                "Index file identity mismatch ({field}). No destructive changes were made."
            )));
        }
    }
    let mut complete = true;
    if !remote["attributes"].is_null() && !remote["attributes"].is_object() {
        return Err(ApiError::local(
            "Index returned invalid verification attributes. No destructive changes were made.",
        ));
    }
    for field in ["contextdock", "project", "document", "version", "sha256"] {
        let value = &remote["attributes"][field];
        if value.is_null() {
            complete = false;
        } else if value != &attrs[field] {
            return Err(ApiError::local(&format!(
                "Index contains a foreign or mismatched file ({field}). No destructive changes were made."
            )));
        }
    }
    Ok(complete)
}
fn wait_for_association(db: &Connection, j: &Value, verification: bool) -> ApiResult<()> {
    if n(j, "poll_count") >= 120 {
        let reason = if verification {
            "verification attributes are still incomplete"
        } else {
            "processing is still in progress"
        };
        return Err(ApiError::local(&format!("Index {reason} after 120 polls (about 30 minutes). Retry/Reconcile to resume checking the existing File; no new upload is needed.")));
    }
    execute(db,"UPDATE cloud_jobs SET state='indexing',phase='poll',association_status=?3,poll_count=poll_count+1,error=NULL,next_at=?2 WHERE id=?1",&[&s(j,"id"),&(chrono::Utc::now().timestamp()+15),&if verification { "verifying" } else { "in_progress" }])
}
fn create_store(db: &Connection, p: &Value, api: &dyn CloudApi) -> ApiResult<()> {
    let pid = s(p, "project_id");
    let expected = metadata(db, p)?;
    if s(p, "store_state") == "pending" {
        execute(db,"UPDATE cloud_projects SET store_state='creating',status='creating' WHERE project_id=?1",&[&pid])?;
        let result = api.request(
            "POST",
            "/vector_stores",
            Some(json!({"name":p["store_name"],"metadata":expected})),
        );
        match result {
            Ok(store) => return adopt(db, p, &store),
            Err(e) => {
                if !e.ambiguous {
                    execute(
                        db,
                        "UPDATE cloud_projects SET store_state='pending' WHERE project_id=?1",
                        &[&pid],
                    )?;
                }
                return Err(e);
            }
        }
    }
    let stores = cloud_api::list(api, "/vector_stores")?;
    let matches = stores
        .iter()
        .filter(|v| v["metadata"] == expected)
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        adopt(db, p, matches[0])
    } else {
        Err(ApiError::local("Interrupted index creation could not be uniquely reconciled. Check the OpenAI dashboard; ContextDock will not blindly create a duplicate. Reconcile again after remote state settles."))
    }
}
fn adopt(db: &Connection, p: &Value, store: &Value) -> ApiResult<()> {
    let id = store["id"]
        .as_str()
        .ok_or_else(|| ApiError::local("Missing remote index identifier."))?;
    cloud_api::valid_id(id, "vs_")?;
    if store["metadata"] != metadata(db, p)? {
        return Err(ApiError::local(
            "Created index identity could not be verified. Reconcile before proceeding.",
        ));
    }
    execute(db,"UPDATE cloud_projects SET store_id=?2,store_state='linked',status='ready',created_at=COALESCE(created_at,?3),error=NULL,attempts=0,next_at=0 WHERE project_id=?1",&[&s(p,"project_id"),&id,&models::now()])?;
    log(
        db,
        s(p, "project_id"),
        "Project index created or recovered with verified ownership.",
    )
}
fn upload(db: &Connection, root: &Path, p: &Value, j: &Value, api: &dyn CloudApi) -> ApiResult<()> {
    let jid = s(j, "id");
    let v = cloud::version(db, s(j, "version_id")).map_err(local)?;
    let bytes = cloud::verified(root, &v, n(p, "soft_limit")).map_err(local)?;
    if s(j, "state") == "pending" && s(j, "phase") != "recover" {
        execute(
            db,
            "UPDATE cloud_jobs SET state='uploading',phase='recover' WHERE id=?1",
            &[&jid],
        )?;
        match api.upload(s(j, "remote_name"), bytes) {
            Ok(file) => return adopt_file(db, j, &file),
            Err(e) => {
                if !e.ambiguous {
                    execute(
                        db,
                        "UPDATE cloud_jobs SET state='pending',phase='upload' WHERE id=?1",
                        &[&jid],
                    )?;
                }
                return Err(e);
            }
        }
    }
    let files = cloud_api::list(api, "/files?purpose=assistants")?;
    let found = files
        .iter()
        .filter(|f| {
            s(f, "filename") == s(j, "remote_name")
                && n(f, "bytes") == v.size
                && s(f, "purpose") == "assistants"
        })
        .collect::<Vec<_>>();
    if found.len() != 1 {
        return Err(ApiError::local("Upload outcome is uncertain. No duplicate upload was attempted. Reconcile after checking Files in the OpenAI dashboard."));
    }
    let id = s(found[0], "id");
    let recovered = api.content(id)?;
    if format!("{:x}", Sha256::digest(&recovered)) != v.hash {
        return Err(ApiError::local(
            "Recovered upload has different bytes. Refusing attachment.",
        ));
    }
    adopt_file(db, j, found[0])
}
fn adopt_file(db: &Connection, j: &Value, file: &Value) -> ApiResult<()> {
    let id = s(file, "id");
    cloud_api::valid_id(id, "file")?;
    if s(file, "filename") != s(j, "remote_name") || n(file, "bytes") != n(j, "rawBytes") {
        return Err(ApiError::local(
            "Upload response identity does not match this snapshot. Reconcile before retrying.",
        ));
    }
    execute(db,"UPDATE cloud_jobs SET file_id=?2,state='uploaded',phase='attach',attempts=0,next_at=0,error=NULL WHERE id=?1",&[&s(j,"id"),&id])
}
fn association(api: &dyn CloudApi, store: &str, file: &str) -> ApiResult<Option<Value>> {
    cloud_api::valid_id(file, "file")?;
    match api.request("GET", &format!("/vector_stores/{store}/files/{file}"), None) {
        Ok(v) => Ok(Some(v)),
        Err(e) if e.missing => Ok(None),
        Err(e) => Err(e),
    }
}
fn attach(db: &Connection, p: &Value, j: &Value, api: &dyn CloudApi) -> ApiResult<()> {
    let store = s(p, "store_id");
    let file = s(j, "file_id");
    let attrs = identity(db, p, j)?;
    let mut existing = association(api, store, file)?;
    if s(j, "phase") == "retry_attach" {
        if let Some(remote) = &existing {
            if matches!(s(remote, "status"), "failed" | "cancelled") {
                owns_association(remote, &attrs, store)?;
                api.request(
                    "DELETE",
                    &format!("/vector_stores/{store}/files/{file}"),
                    None,
                )?;
                existing = None;
            }
        }
    }
    let mut remote = match existing {
        Some(v) => v,
        None => {
            if crate::desktop::uploads_paused(db) {
                return execute(
                    db,
                    "UPDATE cloud_jobs SET next_at=?2 WHERE id=?1",
                    &[&s(j, "id"), &(chrono::Utc::now().timestamp() + 15)],
                );
            }
            // Check File existence first: a missing File must never be reported as indexed.
            api.request("GET", &format!("/files/{file}"), None)?;
            api.request(
                "POST",
                &format!("/vector_stores/{store}/files"),
                Some(json!({"file_id":file,"attributes":attrs})),
            )?
        }
    };
    if !association_verified(&remote, &attrs, store, file)? {
        // Read the existing association instead of trusting an incomplete POST
        // receipt or replaying an upload/attachment. Conflicting values fail above.
        if let Some(full) = association(api, store, file)? {
            remote = full;
        }
        if !association_verified(&remote, &attrs, store, file)? {
            return wait_for_association(db, j, true);
        }
    }
    let status = s(&remote, "status");
    match status {
        "completed" => {
            execute(db,"UPDATE cloud_jobs SET state='indexed',phase='poll',association_status='completed',usage_bytes=?2,remote_current=?3,error=NULL,attempts=0,next_at=0 WHERE id=?1",&[&s(j,"id"),&remote["usage_bytes"].as_i64(),&attrs["is_current"].as_bool()])?;
            log(
                db,
                s(p, "project_id"),
                "Snapshot indexing completed; remote association verified.",
            )
        }
        "in_progress" => wait_for_association(db, j, false),
        "failed" | "cancelled" => {
            execute(
                db,
                "UPDATE cloud_jobs SET association_status=?2,phase='retry_attach' WHERE id=?1",
                &[&s(j, "id"), &status],
            )?;
            Err(ApiError::local("OpenAI could not index this snapshot. Check file support and limits. Retry/Reconcile will replace only the failed association and reuse the uploaded File."))
        }
        _ => Err(ApiError::local(
            "OpenAI returned an unknown indexing state.",
        )),
    }
}
fn detach(db: &Connection, p: &Value, j: &Value, api: &dyn CloudApi) -> ApiResult<()> {
    let file = s(j, "file_id");
    let store = s(p, "store_id");
    if !file.is_empty() {
        if let Some(v) = association(api, store, file)? {
            owns_association(&v, &identity(db, p, j)?, store)?;
            match api.request(
                "DELETE",
                &format!("/vector_stores/{store}/files/{file}"),
                None,
            ) {
                Ok(receipt) if receipt["deleted"] == true => {}
                Err(e) if e.missing => {}
                Err(e) => return Err(e),
                Ok(_) => {
                    return Err(ApiError::local(
                        "OpenAI did not confirm index-entry deletion. Removal remains pending.",
                    ))
                }
            }
            if association(api, store, file)?.is_some() {
                let mut error=ApiError::local(&format!("Removal verification is pending for {}. OpenAI still reports its index entry after confirming deletion; The entry will be checked again during retry.",s(j,"sourceName")));
                error.retry = true;
                error.delay = 5;
                return Err(error);
            }
        }
    }
    execute(db,"UPDATE cloud_jobs SET state='removed',association_status=NULL,remote_current=NULL,error=NULL,next_at=0 WHERE id=?1",&[&s(j,"id")])?;
    log(
        db,
        s(p, "project_id"),
        "Removed vector-store association. Uploaded File retained; local snapshot unchanged.",
    )
}
pub fn remove_owned(
    db: &Connection,
    p: &Value,
    j: &Value,
    api: &dyn CloudApi,
) -> std::result::Result<(), String> {
    cloud::owned_store(api, db, p).map_err(|e| e.message)?;
    detach(db, p, j, api).map_err(|e| e.message)
}
fn reconcile(db: &Connection, p: &Value, api: &dyn CloudApi, allow_pending: bool) -> ApiResult<()> {
    let store = cloud::owned_store(api, db, p)?;
    let mut associations =
        cloud_api::list(api, &format!("/vector_stores/{}/files", s(p, "store_id")))?;
    let jobs = cloud::jobs(db, s(p, "project_id")).map_err(local)?;
    for remote in &mut associations {
        let job=jobs.iter().find(|j|s(j,"file_id")==s(remote,"id")).ok_or_else(||ApiError::local("Index contains an unrecognized association. Reconcile it in OpenAI before continuing."))?;
        let attrs = identity(db, p, job)?;
        let store = s(p, "store_id");
        let file = s(job, "file_id");
        if !association_verified(remote, &attrs, store, file)? {
            if let Some(full) = association(api, store, file)? {
                *remote = full;
            }
            if !association_verified(remote, &attrs, store, file)? {
                if allow_pending
                    && n(job, "desired") == 1
                    && !matches!(
                        s(job, "state"),
                        "failed" | "uncertain" | "removed" | "purged"
                    )
                {
                    wait_for_association(db, job, true)?;
                } else {
                    owns_association(remote, &attrs, store)?;
                }
            }
        }
    }
    let jobs = cloud::jobs(db, s(p, "project_id")).map_err(local)?;
    for job in jobs.iter().filter(|j| s(j, "state") == "indexed") {
        let remote = associations
            .iter()
            .find(|a| s(a, "id") == s(job, "file_id"));
        if remote.is_none_or(|a| s(a, "status") != "completed") {
            execute(db,"UPDATE cloud_jobs SET state='uploaded',phase='attach',error=NULL,next_at=0 WHERE id=?1",&[&s(job,"id")])?;
        }
    }
    execute(db,"UPDATE cloud_projects SET usage_bytes=?2,reconcile_requested=0,last_sync=?3,status='reconciled',error=NULL WHERE project_id=?1",&[&s(p,"project_id"),&store["usage_bytes"].as_i64(),&models::now()])
}
fn source_error(message: &str) -> bool {
    message.contains(cloud::SOURCE_BUSY) || message.contains(cloud::SOURCE_MISSING)
}
/// Probe only local readiness. Never send cloud traffic while an editor holds a
/// selected source, and never treat genuine remote identity failures as recoverable.
pub(crate) fn source_recovery_ready(db: &Connection, p: &Value) -> Result<bool> {
    if n(p, "enabled") != 1
        || n(p, "paused") == 1
        || n(p, "started") != 1
        || n(p, "auto_sync") != 1
        || n(p, "delete_requested") == 1
        || matches!(s(p, "status"), "cleanup_pending" | "cleanup_failed")
    {
        return Ok(false);
    }
    if !s(p, "error").is_empty() && !source_error(s(p, "error")) {
        return Ok(false);
    }
    let interrupted = (s(p, "status") == "needs_attention" && source_error(s(p, "error")))
        || cloud::jobs(db, s(p, "project_id"))?
            .iter()
            .any(|j| s(j, "state") == "failed" && source_error(s(j, "error")));
    Ok(interrupted && cloud::desired(db, s(p, "project_id"), &cloud::draft(db, p)?).is_ok())
}
pub fn legacy_removal_wait(p: &Value) -> bool {
    n(p,"enabled")==1 && n(p,"paused")==0 && n(p,"next_at")==i64::MAX
        && n(p,"attempts")<=6 && s(p,"status")=="needs_attention"
        && s(p,"error")=="The index entry is still present in OpenAI. Removal remains pending; retry after checking the remote state."
}
pub fn step(
    db: &mut Connection,
    root: &Path,
    id: &str,
    api: &dyn CloudApi,
    key_scope: &str,
) -> Result<()> {
    let mut p = cloud::project(db, id)?;
    if source_recovery_ready(db, &p)? {
        let tx = db.unchecked_transaction().map_err(error)?;
        for j in cloud::jobs(&tx, id)?.iter().filter(|j| {
            s(j, "state") == "failed"
                && source_error(s(j, "error"))
                && !matches!(s(j, "phase"), "remove" | "selection_remove" | "path_remove")
        }) {
            tx.execute("UPDATE cloud_jobs SET state=CASE WHEN file_id IS NOT NULL THEN 'uploaded' WHEN phase='recover' THEN 'uploading' ELSE 'pending' END,error=NULL,next_at=0,attempts=0 WHERE id=?1",[s(j,"id")]).map_err(error)?;
        }
        tx.execute("UPDATE cloud_projects SET next_at=0,attempts=0,error=NULL,status='queued' WHERE project_id=?1",[id]).map_err(error)?;
        tx.commit().map_err(error)?;
        p = cloud::project(db, id)?;
    }
    if legacy_removal_wait(&p) {
        db.execute(
            "UPDATE cloud_projects SET next_at=0,status='retry_wait' WHERE project_id=?1",
            [id],
        )
        .map_err(error)?;
        p = cloud::project(db, id)?;
    }
    // Saved document opt-outs can only GET/DELETE their owned associations, including
    // while ordinary uploads are paused or the source folder is unavailable.
    if (n(&p, "enabled") == 1
        || cloud::selection_removals(db, id)?
            .iter()
            .any(|j| s(j, "phase") == "path_remove"))
        && s(&p, "scope") == key_scope
        && !["cleanup_pending", "cleanup_failed"].contains(&s(&p, "status"))
        && n(&p, "next_at") <= chrono::Utc::now().timestamp()
    {
        if let Some(job) = cloud::selection_removals(db, id)?.first() {
            let result=match cloud::owned_store(api,db,&p) {
                Ok(_)=>detach(db,&p,job,api),
                Err(e) if e.missing=>execute(db,"UPDATE cloud_jobs SET state='removed',association_status=NULL,remote_current=NULL,error=NULL,next_at=0 WHERE id=?1",&[&s(job,"id")]),
                Err(e)=>Err(e),
            };
            // Whole-project Off persists its stop intent before waiting for the
            // HTTP gate. A late opt-out result must not overwrite that decision.
            let Ok(current) = cloud::project(db, id) else {
                return Ok(());
            };
            if n(&current, "enabled") != n(&p, "enabled")
                || n(&current, "revision") != n(&p, "revision")
                || ["cleanup_pending", "cleanup_failed"].contains(&s(&current, "status"))
            {
                return Ok(());
            }
            match result {
                Ok(()) => {
                    db.execute("UPDATE cloud_projects SET status=CASE WHEN paused=1 THEN 'paused' ELSE 'queued' END,error=NULL,next_at=0,attempts=0 WHERE project_id=?1",[id]).map_err(error)?;
                }
                Err(e) => {
                    db.execute(
                        "UPDATE cloud_jobs SET error=?2 WHERE id=?1",
                        params![s(job, "id"), e.message],
                    )
                    .map_err(error)?;
                    let tries = n(&p, "attempts") + 1;
                    let retry = e.retry && tries <= 6;
                    let next = if retry {
                        chrono::Utc::now().timestamp()
                            + e.delay.max(2_i64.pow(tries.min(10) as u32)).min(3600)
                    } else {
                        i64::MAX
                    };
                    db.execute("UPDATE cloud_projects SET status=?2,error=?3,next_at=?4,attempts=?5 WHERE project_id=?1",params![id,if retry {"retry_wait"} else {"selection_removal_failed"},format!("Document removal incomplete. Remote references retained for retry. {}",e.message),next,tries]).map_err(error)?;
                }
            }
            return Ok(());
        }
    }
    if n(&p, "delete_requested") != 1 {
        crate::folder_recovery::require_ready(db, id)?;
    }
    if n(&p, "enabled") != 1
        || s(&p, "scope") != key_scope
        || (n(&p, "paused") == 1 && n(&p, "delete_requested") != 1)
    {
        return Ok(());
    }
    if n(&p, "next_at") > chrono::Utc::now().timestamp() {
        return Ok(());
    }
    if n(&p, "delete_requested") != 1 {
        let tracking: bool = db
            .query_row(
                "SELECT tracking_enabled FROM projects WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .map_err(error)?;
        if !tracking {
            return Ok(());
        }
    }
    let result = (|| -> ApiResult<()> {
        if n(&p, "delete_requested") != 1 {
            cloud::desired(db, id, &cloud::draft(db, &p).map_err(local)?).map_err(local)?;
        }
        if s(&p, "store_id").is_empty() {
            if crate::desktop::uploads_paused(db) {
                return Ok(());
            }
            return create_store(db, &p, api);
        }
        let store = match cloud::owned_store(api, db, &p) {
            Ok(store) => store,
            Err(e) if e.missing => {
                execute(db,"UPDATE cloud_jobs SET state='uploaded',association_status=NULL WHERE project_id=?1 AND state='indexed'",&[&id])?;
                if n(&p, "delete_requested") == 1 {
                    execute(db,"UPDATE cloud_jobs SET state='removed' WHERE project_id=?1 AND state!='purged'",&[&id])?;
                    return execute(db,"UPDATE cloud_projects SET enabled=0,paused=1,started=0,delete_requested=0,store_state='deleted',status='deleted',error=NULL WHERE project_id=?1",&[&id]);
                }
                return Err(e);
            }
            Err(e) => return Err(e),
        };
        if n(&p, "delete_requested") == 1 {
            return delete_index(db, &p, api);
        }
        if s(&store, "status") == "expired" {
            return Err(ApiError::local("This project index expired. Delete/recreate explicitly; no replacement index was created."));
        }
        execute(
            db,
            "UPDATE cloud_projects SET usage_bytes=?2 WHERE project_id=?1",
            &[&id, &store["usage_bytes"].as_i64()],
        )?;
        if n(&p, "reconcile_requested") == 1 {
            reconcile(db, &p, api, true)?;
        }
        if n(&p, "started") != 1 {
            execute(db,"UPDATE cloud_projects SET sync_requested=0,status='ready',error=NULL WHERE project_id=?1",&[&id])?;
            return Ok(());
        }
        if n(&p, "auto_sync") == 1 || n(&p, "sync_requested") == 1 {
            cloud::queue(db, id).map_err(local)?;
        }
        if let Some(job) = cloud::jobs(db, id).map_err(local)?.iter().find(|j| {
            s(j, "phase") == "remove" && s(j, "state") != "removed" && s(j, "state") != "purged"
        }) {
            return detach(db, &p, job, api);
        }
        let jobs = cloud::jobs(db, id).map_err(local)?;
        let tracking: bool = db
            .query_row(
                "SELECT tracking_enabled FROM projects WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .map_err(error)
            .map_err(local)?;
        if let Some(j) = jobs.iter().find(|j| {
            tracking
                && (!crate::desktop::uploads_paused(db) || s(j, "state") == "indexing")
                && n(j, "desired") == 1
                && n(j, "next_at") <= chrono::Utc::now().timestamp()
                && !["indexed", "removed", "purged", "failed", "uncertain"].contains(&s(j, "state"))
        }) {
            let result = if s(j, "file_id").is_empty() {
                upload(db, root, &p, j, api)
            } else {
                attach(db, &p, j, api)
            };
            if let Err(mut e) = result {
                if e.message
                    == "Cloud sync paused, disconnected or stopped before the next request."
                {
                    return Err(e);
                }
                e.message = format!(
                    "{} ({}): {} Other files can continue syncing.",
                    s(j, "sourcePath"),
                    crate::matching::version_label(s(j, "sourcePath"))
                        .unwrap_or_else(|| format!("saved revision {}", n(j, "number"))),
                    e.message
                );
                job_failure(db, j, &e)?;
                log(db, id, &e.message)?;
                // Backoff/failure belongs to this file, not the entire project.
                return Ok(());
            }
            return Ok(());
        }
        if crate::desktop::uploads_paused(db) {
            return Ok(());
        }
        // Preserve all known-good old associations until every desired replacement is indexed.
        let ready = jobs
            .iter()
            .filter(|j| n(j, "desired") == 1)
            .all(|j| s(j, "state") == "indexed");
        if ready {
            if let Some(j) = jobs
                .iter()
                .find(|j| n(j, "desired") == 0 && !matches!(s(j, "state"), "removed" | "purged"))
            {
                return detach(db, &p, j, api);
            }
            for j in jobs.iter().filter(|j| s(j, "state") == "indexed") {
                let attrs = identity(db, &p, j)?;
                if j["remote_current"].as_i64().map(|v| v == 1) != attrs["is_current"].as_bool() {
                    let remote=association(api,s(&p,"store_id"),s(j,"file_id"))?.ok_or_else(||ApiError::local("Indexed association disappeared. Reconcile before updating metadata."))?;
                    owns_association(&remote, &attrs, s(&p, "store_id"))?;
                    api.request(
                        "POST",
                        &format!(
                            "/vector_stores/{}/files/{}",
                            s(&p, "store_id"),
                            s(j, "file_id")
                        ),
                        Some(json!({"attributes":attrs})),
                    )?;
                    return execute(
                        db,
                        "UPDATE cloud_jobs SET remote_current=?2 WHERE id=?1",
                        &[&s(j, "id"), &attrs["is_current"].as_bool()],
                    );
                }
            }
            execute(db,"UPDATE cloud_projects SET sync_requested=0,status='up_to_date',last_sync=?2,error=NULL,attempts=0 WHERE project_id=?1",&[&id,&models::now()])?;
        } else if jobs
            .iter()
            .filter(|j| n(j, "desired") == 1)
            .all(|j| matches!(s(j, "state"), "indexed" | "failed" | "uncertain"))
        {
            execute(db,"UPDATE cloud_projects SET sync_requested=0,status='file_errors',last_sync=?2,error=NULL WHERE project_id=?1",&[&id,&models::now()])?;
        }
        Ok(())
    })();
    if let Err(e) = result {
        // A concurrently persisted pause/disconnect is a control decision, not
        // a provider failure. Preserve it instead of overwriting its status.
        let Ok(current) = cloud::project(db, id) else {
            return Ok(());
        };
        let connected: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM cloud_connection WHERE scope=?1 AND status='connected'",
                [key_scope],
                |r| r.get(0),
            )
            .map_err(error)?;
        if n(&current, "enabled") != 1
            || (n(&current, "paused") == 1 && n(&current, "delete_requested") != 1)
            || connected != 1
        {
            return Ok(());
        }
        let tries = n(&p, "attempts") + 1;
        let delay = e.delay.max(2_i64.pow(tries.min(10) as u32)).min(3600);
        let next = if e.retry && tries <= 6 {
            chrono::Utc::now().timestamp() + delay
        } else {
            i64::MAX
        };
        db.execute("UPDATE cloud_projects SET status=?2,error=?3,next_at=?4,attempts=?5 WHERE project_id=?1",params![id,if e.ambiguous {"uncertain"} else if e.retry && tries<=6 {"retry_wait"} else {"needs_attention"},e.message,next,tries]).map_err(error)?;
        cloud::activity(db, id, "error", &e.message)?;
    } else {
        db.execute(
            "UPDATE cloud_projects SET attempts=0,next_at=0 WHERE project_id=?1",
            [id],
        )
        .map_err(error)?;
    }
    Ok(())
}
fn job_failure(db: &Connection, j: &Value, e: &ApiError) -> ApiResult<()> {
    if e.message == "Cloud sync paused, disconnected or stopped before the next request." {
        return Ok(());
    }
    let tries = n(j, "attempts") + 1;
    let state = if e.ambiguous {
        "uncertain"
    } else if e.retry && tries <= 6 {
        s(j, "state")
    } else {
        "failed"
    };
    execute(
        db,
        "UPDATE cloud_jobs SET state=?2,error=?3,attempts=?4,next_at=?5 WHERE id=?1",
        &[
            &s(j, "id"),
            &state,
            &e.message,
            &tries,
            &(chrono::Utc::now().timestamp() + e.delay.max(2_i64.pow(tries.min(10) as u32))),
        ],
    )
}
fn delete_index(db: &Connection, p: &Value, api: &dyn CloudApi) -> ApiResult<()> {
    // List and verify every association before deleting anything.
    reconcile(db, p, api, false)?;
    let jobs = cloud::jobs(db, s(p, "project_id")).map_err(local)?;
    if let Some(j) = jobs
        .iter()
        .find(|j| !matches!(s(j, "state"), "removed" | "purged"))
    {
        return detach(db, p, j, api);
    }
    match api.request(
        "DELETE",
        &format!("/vector_stores/{}", s(p, "store_id")),
        None,
    ) {
        Ok(_) => {}
        Err(e) if e.missing => {}
        Err(e) => return Err(e),
    };
    execute(db,"UPDATE cloud_projects SET enabled=0,paused=1,started=0,auto_sync=0,delete_requested=0,store_state='deleted',status='deleted',error=NULL WHERE project_id=?1",&[&s(p,"project_id")])?;
    log(db,s(p,"project_id"),"Project vector store deleted. Underlying uploaded Files retained for explicit separate cleanup.")
}
pub fn search(
    db: &Connection,
    id: &str,
    query: &str,
    api: &dyn CloudApi,
    bound: &str,
) -> Result<Value> {
    search_impl(db, id, query, api, bound, false)
}
pub fn search_current(
    db: &Connection,
    id: &str,
    query: &str,
    api: &dyn CloudApi,
    bound: &str,
) -> Result<Value> {
    search_impl(db, id, query, api, bound, true)
}
fn search_impl(
    db: &Connection,
    id: &str,
    query: &str,
    api: &dyn CloudApi,
    bound: &str,
    current_only: bool,
) -> Result<Value> {
    crate::folder_recovery::require_ready(db, id)?;
    if !cloud::selection_removals(db, id)?.is_empty() {
        return Err("Document opt-out removals are not yet verified. Finish or retry them before searching.".into());
    }
    let p = cloud::project(db, id)?;
    if n(&p, "enabled") != 1 || n(&p, "paused") == 1 || s(&p, "scope") != bound {
        return Err("Enable/resume this project and verify the saved key first.".into());
    }
    if query.trim().is_empty() || query.len() > 2000 {
        return Err("Enter a search query of 1–2000 bytes.".into());
    }
    cloud::owned_store(api, db, &p).map_err(|e| e.message)?;
    let known = cloud::jobs(db, id)?;
    let wanted = cloud::desired(db, id, &cloud::draft(db, &p)?)?;
    if wanted.iter().any(|v| {
        !known
            .iter()
            .any(|j| s(j, "version_id") == v.id && s(j, "state") == "indexed")
    }) || known
        .iter()
        .any(|j| s(j, "state") == "indexed" && !wanted.iter().any(|v| v.id == s(j, "version_id")))
    {
        return Err("Cloud selection changes are not fully synchronized. Sync/reconcile before searching this project.".into());
    }
    let mut filters = vec![
        json!({"type":"eq","key":"contextdock","value":cloud::install(db)?}),
        json!({"type":"eq","key":"project","value":id}),
    ];
    if current_only || s(&p, "mode") == "current" {
        filters.push(json!({"type":"eq","key":"is_current","value":true}));
    }
    let response=api.request("POST",&format!("/vector_stores/{}/search",s(&p,"store_id")),Some(json!({"query":query,"max_num_results":5,"rewrite_query":false,"filters":{"type":"and","filters":filters}}))).map_err(|e|e.message)?;
    let jobs = cloud::jobs(db, id)?;
    let mut hits = Vec::new();
    let mut provenance = Vec::new();
    for hit in response["data"]
        .as_array()
        .ok_or("Invalid search response.")?
        .iter()
        .take(5)
    {
        let job = jobs
            .iter()
            .find(|j| s(j, "file_id") == s(hit, "file_id") && s(j, "state") == "indexed")
            .ok_or("Search returned an unknown file. Reconcile before searching again.")?;
        let attrs = identity(db, &p, job).map_err(|e| e.message)?;
        if !wanted
            .iter()
            .any(|v| v.id == s(job, "version_id") && (!current_only || v.current))
            || ((current_only || s(&p, "mode") == "current")
                && hit["attributes"]["is_current"] != attrs["is_current"])
        {
            return Err("Local selection/current version changed during search. Synchronize and search again.".into());
        }
        if ["contextdock", "project", "document", "version", "sha256"]
            .iter()
            .any(|k| hit["attributes"][k] != attrs[k])
        {
            return Err("Search provenance does not match this project.".into());
        }
        let excerpt = hit["content"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|c| c["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default()
            .chars()
            .take(8000)
            .collect::<String>();
        let source = json!({"fileId":job["file_id"],"familyName":job["familyName"],"sourceName":job["sourceName"],"number":job["number"],"versionId":job["version_id"],"score":hit["score"]});
        provenance.push(source.clone());
        let mut result = source;
        result["excerpt"] = json!(excerpt);
        hits.push(result);
    }
    db.execute(
        "INSERT INTO cloud_retrievals VALUES(?1,?2,?3,?4)",
        params![
            uuid::Uuid::new_v4().to_string(),
            id,
            models::now(),
            json!(provenance).to_string()
        ],
    )
    .map_err(error)?;
    Ok(json!(hits))
}
