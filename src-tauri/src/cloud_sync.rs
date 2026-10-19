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
                if e.retry && !e.ambiguous {
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
    if s(j, "state") == "pending" {
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
    let remote = match association(api, store, file)? {
        Some(v) => v,
        None => {
            // Check File existence first: a missing File must never be reported as indexed.
            api.request("GET", &format!("/files/{file}"), None)?;
            api.request(
                "POST",
                &format!("/vector_stores/{store}/files"),
                Some(json!({"file_id":file,"attributes":attrs})),
            )?
        }
    };
    owns_association(&remote, &attrs, store)?;
    let status = s(&remote, "status");
    match status {
        "completed"=>{
            execute(db,"UPDATE cloud_jobs SET state='indexed',phase='poll',association_status='completed',usage_bytes=?2,remote_current=?3,error=NULL,attempts=0,next_at=0 WHERE id=?1",&[&s(j,"id"),&remote["usage_bytes"].as_i64(),&attrs["is_current"].as_bool()])?;
            log(db,s(p,"project_id"),"Snapshot indexing completed; remote association verified.")
        },
        "in_progress"=>execute(db,"UPDATE cloud_jobs SET state='indexing',phase='poll',association_status='in_progress',next_at=?2 WHERE id=?1",&[&s(j,"id"),&(chrono::Utc::now().timestamp()+15)]),
        "failed"|"cancelled"=>Err(ApiError::local("OpenAI could not index this snapshot. Check file support and limits. Remove the failed association in the dashboard, then Retry/Reconcile to reuse the uploaded File.")),
        _=>Err(ApiError::local("OpenAI returned an unknown indexing state."))
    }
}
fn detach(db: &Connection, p: &Value, j: &Value, api: &dyn CloudApi) -> ApiResult<()> {
    let file = s(j, "file_id");
    let store = s(p, "store_id");
    if !file.is_empty() {
        if let Some(v) = association(api, store, file)? {
            owns_association(&v, &identity(db, p, j)?, store)?;
            api.request(
                "DELETE",
                &format!("/vector_stores/{store}/files/{file}"),
                None,
            )?;
        }
    }
    execute(db,"UPDATE cloud_jobs SET state='removed',association_status=NULL,remote_current=NULL,error=NULL,next_at=0 WHERE id=?1",&[&s(j,"id")])?;
    log(
        db,
        s(p, "project_id"),
        "Removed vector-store association. Uploaded File retained; local snapshot unchanged.",
    )
}
fn reconcile(db: &Connection, p: &Value, api: &dyn CloudApi) -> ApiResult<()> {
    let store = cloud::owned_store(api, db, p)?;
    let associations = cloud_api::list(api, &format!("/vector_stores/{}/files", s(p, "store_id")))?;
    let jobs = cloud::jobs(db, s(p, "project_id")).map_err(local)?;
    for remote in &associations {
        let job=jobs.iter().find(|j|s(j,"file_id")==s(remote,"id")).ok_or_else(||ApiError::local("Index contains an unrecognized association. Reconcile it in OpenAI before continuing."))?;
        owns_association(remote, &identity(db, p, job)?, s(p, "store_id"))?;
    }
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
pub fn step(
    db: &mut Connection,
    root: &Path,
    id: &str,
    api: &dyn CloudApi,
    key_scope: &str,
) -> Result<()> {
    let p = cloud::project(db, id)?;
    if n(&p, "enabled") != 1
        || s(&p, "scope") != key_scope
        || (n(&p, "paused") == 1 && n(&p, "delete_requested") != 1)
    {
        return Ok(());
    }
    if n(&p, "next_at") > chrono::Utc::now().timestamp() {
        return Ok(());
    }
    let result = (|| -> ApiResult<()> {
        if s(&p, "store_id").is_empty() {
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
        if s(&store, "status") == "expired" {
            return Err(ApiError::local("This project index expired. Delete/recreate explicitly; no replacement index was created."));
        }
        execute(
            db,
            "UPDATE cloud_projects SET usage_bytes=?2 WHERE project_id=?1",
            &[&id, &store["usage_bytes"].as_i64()],
        )?;
        if n(&p, "delete_requested") == 1 {
            return delete_index(db, &p, api);
        }
        if n(&p, "reconcile_requested") == 1 {
            reconcile(db, &p, api)?;
        }
        if n(&p, "started") != 1 {
            return Ok(());
        }
        if let Some(job) = cloud::jobs(db, id).map_err(local)?.iter().find(|j| {
            s(j, "phase") == "remove" && s(j, "state") != "removed" && s(j, "state") != "purged"
        }) {
            return detach(db, &p, job, api);
        }
        if n(&p, "auto_sync") == 1 || n(&p, "sync_requested") == 1 {
            cloud::queue(db, id).map_err(local)?;
        }
        let jobs = cloud::jobs(db, id).map_err(local)?;
        if let Some(j) = jobs.iter().find(|j| {
            n(j, "desired") == 1
                && n(j, "next_at") <= chrono::Utc::now().timestamp()
                && !["indexed", "removed", "purged", "failed", "uncertain"].contains(&s(j, "state"))
        }) {
            let result = if s(j, "file_id").is_empty() {
                upload(db, root, &p, j, api)
            } else {
                attach(db, &p, j, api)
            };
            if let Err(e) = result {
                job_failure(db, j, &e)?;
                return Err(e);
            }
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
        }
        Ok(())
    })();
    if let Err(e) = result {
        let tries = n(&p, "attempts") + 1;
        let delay = e.delay.max(2_i64.pow(tries.min(10) as u32)).min(3600);
        let next = if e.retry && tries <= 6 {
            chrono::Utc::now().timestamp() + delay
        } else {
            i64::MAX
        };
        db.execute("UPDATE cloud_projects SET status=?2,error=?3,next_at=?4,attempts=?5 WHERE project_id=?1",params![id,if e.ambiguous {"uncertain"} else if e.retry {"retry_wait"} else {"needs_attention"},e.message,next,tries]).map_err(error)?;
        cloud::activity(db, id, "error", &e.message)?;
    }
    Ok(())
}
fn job_failure(db: &Connection, j: &Value, e: &ApiError) -> ApiResult<()> {
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
    reconcile(db, p, api)?;
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
    let p = cloud::project(db, id)?;
    if n(&p, "enabled") != 1 || n(&p, "paused") == 1 || s(&p, "scope") != bound {
        return Err("Enable/resume this project and verify the saved key first.".into());
    }
    if query.trim().is_empty() || query.len() > 2000 {
        return Err("Enter a search query of 1–2000 bytes.".into());
    }
    cloud::owned_store(api, db, &p).map_err(|e| e.message)?;
    let known = cloud::jobs(db, id)?;
    if cloud::desired(db, id, &cloud::draft(db, &p)?)?
        .iter()
        .any(|v| {
            !known
                .iter()
                .any(|j| s(j, "version_id") == v.id && s(j, "state") == "indexed")
        })
    {
        return Err("Selected snapshots are not fully indexed. Sync/reconcile before searching this project.".into());
    }
    let mut filters = vec![
        json!({"type":"eq","key":"contextdock","value":cloud::install(db)?}),
        json!({"type":"eq","key":"project","value":id}),
    ];
    if s(&p, "mode") == "current" {
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
        let source = json!({"fileId":job["file_id"],"familyName":job["familyName"],"number":job["number"],"versionId":job["version_id"],"score":hit["score"]});
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
