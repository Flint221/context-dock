//! Explicit, resumable project cloud erasure. Never discard IDs before remote verification.
use crate::{
    cloud::{self, n, s},
    cloud_api::{self, CloudApi},
    registry::{error, Result},
};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

pub fn has_remote(db: &Connection, p: &Value) -> Result<bool> {
    Ok(!s(p, "store_id").is_empty()
        || !s(p, "store_op").is_empty()
        || !cloud::jobs(db, s(p, "project_id"))?.is_empty())
}
pub fn begin(db: &Connection, id: &str, payload: &Value) -> Result<()> {
    let p = cloud::project(db, id)?;
    if payload["confirm"] != true
        || payload["storeId"] != p["store_id"]
        || s(payload, "confirmation") != s(&p, "store_name")
    {
        return Err(
            "The project cloud data changed. Review the deletion confirmation again.".into(),
        );
    }
    db.execute("UPDATE cloud_projects SET enabled=1,paused=1,auto_sync=0,sync_requested=0,reconcile_requested=0,delete_requested=0,status='cleanup_pending',error=NULL,revision=revision+1 WHERE project_id=?1",[id]).map_err(error)?;
    cloud::verify_flags(
        db,
        id,
        &[
            ("paused", 1),
            ("delete_requested", 0),
            ("sync_requested", 0),
        ],
    )
}
fn absent(api: &dyn CloudApi, path: &str) -> Result<()> {
    match api.request("GET",path,None) {
        Err(e) if e.missing => Ok(()),
        Err(e) => Err(e.message),
        Ok(_) => Err("OpenAI still reports the resource exists. Cloud cleanup is incomplete; retry deletion.".into()),
    }
}
fn erase(api: &dyn CloudApi, path: &str) -> Result<()> {
    match api.request("DELETE", path, None) {
        Ok(receipt) if receipt["deleted"] == true => {}
        Err(e) if e.missing => {}
        Err(e) => return Err(e.message),
        Ok(_) => {
            return Err("OpenAI did not confirm deletion. Cloud cleanup is incomplete.".into())
        }
    }
    absent(api, path)
}
pub fn finish(db: &mut Connection, id: &str, receipt: Value) -> Result<Value> {
    let tx = db.transaction().map_err(error)?;
    tx.execute("DELETE FROM cloud_jobs WHERE project_id=?1", [id])
        .map_err(error)?;
    tx.execute("DELETE FROM cloud_exclusions WHERE project_id=?1", [id])
        .map_err(error)?;
    tx.execute("UPDATE cloud_projects SET enabled=0,paused=1,started=0,auto_sync=0,store_id=NULL,store_op=NULL,store_state='none',status='cloud_deleted',error=NULL,delete_requested=0,sync_requested=0,reconcile_requested=0,usage_bytes=0,created_at=NULL,last_sync=NULL,next_at=0,attempts=0,revision=revision+1 WHERE project_id=?1",[id]).map_err(error)?;
    cloud::verify_flags(&tx, id, &[("enabled", 0), ("paused", 1), ("started", 0)])?;
    let saved = cloud::project(&tx, id)?;
    if !s(&saved, "store_id").is_empty()
        || !s(&saved, "store_op").is_empty()
        || s(&saved, "store_state") != "none"
        || s(&saved, "status") != "cloud_deleted"
        || !cloud::jobs(&tx, id)?.is_empty()
    {
        return Err(
            "Cloud cleanup completion could not be saved. References retained for retry.".into(),
        );
    }
    cloud::activity(&tx,id,"deletion",&format!("Verified project cloud cleanup: {receipt}. Local documents, snapshots and chats retained."))?;
    tx.commit().map_err(error)?;
    Ok(receipt)
}
pub fn run(db: &mut Connection, id: &str, api: &dyn CloudApi, scope: &str) -> Result<Value> {
    let result = erase_owned(db, id, api, scope);
    if let Err(message) = &result {
        db.execute("UPDATE cloud_projects SET status='cleanup_failed',error=?2,paused=1 WHERE project_id=?1",params![id,format!("Cloud deletion incomplete. References retained for retry. {message}")]).map_err(error)?;
    }
    result
}
fn erase_owned(db: &mut Connection, id: &str, api: &dyn CloudApi, scope: &str) -> Result<Value> {
    let p = cloud::project(db, id)?;
    if s(&p, "scope") != scope || n(&p, "paused") != 1 || s(&p, "status") != "cleanup_pending" {
        return Err("Cloud cleanup authorization changed. Review deletion again.".into());
    }
    let install = cloud::install(db)?;
    let jobs = cloud::jobs(db, id)?;
    let stores = cloud_api::list(api, "/vector_stores").map_err(|e| e.message)?;
    let mut owned: HashSet<String> = stores
        .iter()
        .filter(|v| v["metadata"]["contextdock"] == install && v["metadata"]["project"] == id)
        .map(|v| s(v, "id").to_string())
        .collect();
    if !s(&p, "store_id").is_empty() {
        match cloud::owned_store(api, db, &p) {
            Ok(_) => {
                owned.insert(s(&p, "store_id").into());
            }
            Err(e) if e.missing => {}
            Err(e) => return Err(e.message),
        }
    }
    // Recover uploads whose HTTP response was lost by their exact app-generated job filename.
    let remote_files = cloud_api::list(api, "/files").map_err(|e| e.message)?;
    let mut files = Vec::new();
    for remote in remote_files {
        if let Some(job) = jobs.iter().find(|j| {
            s(j, "remote_name") == s(&remote, "filename")
                || (!s(j, "file_id").is_empty() && s(j, "file_id") == s(&remote, "id"))
        }) {
            let file = s(&remote, "id");
            cloud_api::valid_id(file, "file").map_err(|e| e.message)?;
            if s(job, "remote_name") != s(&remote, "filename")
                || n(job, "rawBytes") != n(&remote, "bytes")
            {
                return Err("Uploaded File ownership mismatch. No cloud data was deleted.".into());
            }
            let bytes = api.content(file).map_err(|e| e.message)?;
            let version = cloud::version(db, s(job, "version_id"))?;
            if format!("{:x}", Sha256::digest(bytes)) != version.hash {
                return Err("Uploaded File integrity mismatch. No cloud data was deleted.".into());
            }
            files.push(file.to_string());
        }
    }
    for job in &jobs {
        let file = s(job, "file_id");
        if !file.is_empty() && !files.iter().any(|f| f == file) {
            cloud_api::valid_id(file, "file").map_err(|e| e.message)?;
            absent(api, &format!("/files/{file}"))?;
        }
    }
    // Refuse to delete uploads used by any other accessible bucket.
    for store in &stores {
        let store_id = s(store, "id");
        cloud_api::valid_id(store_id, "vs_").map_err(|e| e.message)?;
        let associations = cloud_api::list(api, &format!("/vector_stores/{store_id}/files"))
            .map_err(|e| e.message)?;
        if owned.contains(store_id) {
            for a in &associations {
                if a["attributes"]["contextdock"] != install
                    || a["attributes"]["project"] != id
                    || !jobs.iter().any(|j| {
                        j["version_id"] == a["attributes"]["version"]
                            && j["document_id"] == a["attributes"]["document"]
                    })
                {
                    return Err("Project bucket contains an unrecognized association. No cloud data was deleted.".into());
                }
            }
        } else if associations
            .iter()
            .any(|a| files.iter().any(|f| s(a, "id") == f))
        {
            return Err("Another vector store uses this project's uploaded File. Cloud data was retained; resolve the shared reference before retrying deletion.".into());
        }
    }
    for store in &owned {
        cloud_api::valid_id(store, "vs_").map_err(|e| e.message)?;
        erase(api, &format!("/vector_stores/{store}"))?;
    }
    for file in &files {
        erase(api, &format!("/files/{file}"))?;
    }
    // Re-list to avoid declaring success when an interrupted earlier operation materialized late.
    if cloud_api::list(api, "/vector_stores")
        .map_err(|e| e.message)?
        .iter()
        .any(|v| v["metadata"]["contextdock"] == install && v["metadata"]["project"] == id)
        || cloud_api::list(api, "/files")
            .map_err(|e| e.message)?
            .iter()
            .any(|v| jobs.iter().any(|j| s(j, "remote_name") == s(v, "filename")))
    {
        return Err(
            "Cloud data is still visible. References retained; retry cloud deletion.".into(),
        );
    }
    finish(
        db,
        id,
        json!({"storesDeleted":owned.len(),"filesDeleted":files.len(),"verified":true}),
    )
}
