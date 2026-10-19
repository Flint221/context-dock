//! Cloud state has its own short-lived SQLite connection. No HTTP holds the local registry lock.
use crate::{
    cloud_api::{self, ApiError, CloudApi},
    cloud_models::{CloudDraft, CloudPlan, CloudVersion},
    extraction, models,
    registry::{error, Result},
    storage,
};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{io::Read, path::Path};
pub fn connect(path: &Path) -> Result<Connection> {
    let db = Connection::open(path).map_err(error)?;
    db.busy_timeout(std::time::Duration::from_secs(10))
        .map_err(error)?;
    db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;")
        .map_err(error)?;
    Ok(db)
}
pub fn rows(db: &Connection, sql: &str, args: &[&dyn rusqlite::ToSql]) -> Result<Vec<Value>> {
    let mut q = db.prepare(sql).map_err(error)?;
    let names = q
        .column_names()
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let result = q
        .query_map(args, |r| {
            let mut map = serde_json::Map::new();
            for (i, name) in names.iter().enumerate() {
                let value = match r.get_ref(i)? {
                    rusqlite::types::ValueRef::Null => Value::Null,
                    rusqlite::types::ValueRef::Integer(n) => json!(n),
                    rusqlite::types::ValueRef::Real(n) => json!(n),
                    rusqlite::types::ValueRef::Text(t) => json!(String::from_utf8_lossy(t)),
                    _ => Value::Null,
                };
                map.insert(name.clone(), value);
            }
            Ok(Value::Object(map))
        })
        .map_err(error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(error);
    result
}
pub fn project(db: &Connection, id: &str) -> Result<Value> {
    rows(db,"SELECT c.* FROM cloud_projects c JOIN projects p ON p.id=c.project_id WHERE p.id=?1 AND p.removed_at IS NULL",&[&id])?.pop().ok_or("Project not registered.".into())
}
pub fn s<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key].as_str().unwrap_or("")
}
pub fn n(v: &Value, key: &str) -> i64 {
    v[key].as_i64().unwrap_or(0)
}
pub fn install(db: &Connection) -> Result<String> {
    db.query_row("SELECT id FROM cloud_installation", [], |r| r.get(0))
        .map_err(error)
}
pub fn scope(key: &str) -> String {
    format!("{:x}", Sha256::digest(key.as_bytes()))
}
pub fn activity(db: &Connection, id: &str, kind: &str, message: &str) -> Result<()> {
    db.execute(
        "INSERT INTO cloud_activity(project_id,kind,message,created_at) VALUES(?1,?2,?3,?4)",
        params![id, kind, message, models::now()],
    )
    .map_err(error)?;
    Ok(())
}
const ELIGIBLE:&str="d.tracking_state='tracked' AND d.merged_into IS NULL AND d.current_version_id IS NOT NULL AND d.kind IN ('markdown','word') AND NOT EXISTS(SELECT 1 FROM pending_matches m JOIN source_files sf ON sf.id=m.source_id WHERE sf.document_id=d.id OR EXISTS(SELECT 1 FROM json_each(m.candidate_ids) j WHERE j.value=d.id))";
pub fn families(db: &Connection, id: &str) -> Result<Vec<Value>> {
    rows(db,&format!("SELECT d.id,d.family_name name,({ELIGIBLE}) eligible, EXISTS(SELECT 1 FROM cloud_documents cd WHERE cd.project_id=d.project_id AND cd.document_id=d.id) selected,(SELECT number FROM document_versions WHERE id=d.current_version_id) currentNumber,(SELECT MAX(v.number) FROM cloud_jobs j JOIN document_versions v ON v.id=j.version_id WHERE j.document_id=d.id AND j.state='indexed' AND j.remote_current=1) cloudNumber,(SELECT COUNT(*) FROM document_versions WHERE document_id=d.id) versionCount,(SELECT COALESCE(SUM(size),0) FROM document_versions WHERE document_id=d.id) rawBytes FROM documents d WHERE d.project_id=?1 AND d.merged_into IS NULL ORDER BY d.family_name"),&[&id])
}
pub fn desired(db: &Connection, id: &str, draft: &CloudDraft) -> Result<Vec<CloudVersion>> {
    if !["current", "history"].contains(&draft.mode.as_str())
        || !(1..=512 * 1024 * 1024).contains(&draft.soft_limit)
    {
        return Err("Choose a valid mode and a soft limit between 1 byte and 512 MiB.".into());
    }
    let families = families(db, id)?;
    if draft.documents.iter().any(|d| {
        !families
            .iter()
            .any(|f| s(f, "id") == d && n(f, "eligible") == 1)
    }) {
        return Err(
            "Selection includes an ineligible or unrelated document. Resolve local tracking first."
                .into(),
        );
    }
    let mut q=db.prepare(&format!("SELECT v.id,v.document_id,v.number,d.kind,v.sha256,v.snapshot_relative,v.size,v.id=d.current_version_id FROM document_versions v JOIN documents d ON d.id=v.document_id WHERE d.project_id=?1 AND {ELIGIBLE} AND NOT EXISTS(SELECT 1 FROM cloud_exclusions e WHERE e.project_id=d.project_id AND e.version_id=v.id) ORDER BY d.id,v.number")).map_err(error)?;
    let versions = q
        .query_map([id], |r| {
            Ok(CloudVersion {
                id: r.get(0)?,
                document: r.get(1)?,
                number: r.get(2)?,
                kind: r.get(3)?,
                hash: r.get(4)?,
                snapshot: r.get(5)?,
                size: r.get(6)?,
                current: r.get(7)?,
            })
        })
        .map_err(error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(error)?;
    Ok(versions
        .into_iter()
        .filter(|v| draft.documents.contains(&v.document) && (draft.mode == "history" || v.current))
        .collect())
}
pub fn draft(db: &Connection, p: &Value) -> Result<CloudDraft> {
    Ok(CloudDraft {
        enabled: n(p, "enabled") == 1,
        automatic: n(p, "auto_sync") == 1,
        mode: s(p, "mode").into(),
        soft_limit: n(p, "soft_limit"),
        documents: families(db, s(p, "project_id"))?
            .iter()
            .filter(|f| n(f, "selected") == 1)
            .map(|f| s(f, "id").into())
            .collect(),
    })
}
pub fn preview(db: &Connection, id: &str, draft: &CloudDraft) -> Result<CloudPlan> {
    let p = project(db, id)?;
    let versions = desired(db, id, draft)?;
    let jobs = jobs(db, id)?;
    let wanted = versions.iter().map(|v| v.id.as_str()).collect::<Vec<_>>();
    let additions = versions
        .iter()
        .filter(|v| {
            !jobs
                .iter()
                .any(|j| s(j, "version_id") == v.id && s(j, "state") == "indexed")
        })
        .count();
    let removals = jobs
        .iter()
        .filter(|j| {
            !wanted.contains(&s(j, "version_id")) && !["removed", "purged"].contains(&s(j, "state"))
        })
        .count();
    let token = scope(&format!(
        "{id}:{}:{}:{wanted:?}:{jobs:?}",
        n(&p, "revision"),
        serde_json::to_string(draft).map_err(|_| "Invalid selection")?
    ));
    Ok(CloudPlan {
        token,
        additions,
        removals,
        files: versions.len(),
        bytes: versions.iter().map(|v| v.size).sum(),
        store_name: s(&p, "store_name").into(),
    })
}
pub fn apply(db: &mut Connection, id: &str, draft: &CloudDraft, token: &str) -> Result<()> {
    let tx = db
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(error)?;
    if preview(&tx, id, draft)?.token != token {
        return Err("Local versions or cloud state changed. Review a fresh preview.".into());
    }
    let bound:String=tx.query_row("SELECT COALESCE(scope,'') FROM cloud_connection WHERE singleton=1 AND status='connected'",[],|r|r.get(0)).map_err(|_|"Test the saved OpenAI key successfully before enabling cloud indexing.".to_string())?;
    let p = project(&tx, id)?;
    if n(&p, "delete_requested") == 1 {
        return Err(
            "Finish or disconnect the pending index deletion before changing cloud configuration."
                .into(),
        );
    }
    if !s(&p, "scope").is_empty() && s(&p, "scope") != bound {
        return Err(
            "Verify the existing index with the replacement key before changing its configuration."
                .into(),
        );
    }
    tx.execute("DELETE FROM cloud_documents WHERE project_id=?1", [id])
        .map_err(error)?;
    for d in &draft.documents {
        tx.execute("INSERT INTO cloud_documents VALUES(?1,?2)", params![id, d])
            .map_err(error)?;
    }
    tx.execute("UPDATE cloud_projects SET enabled=?2,auto_sync=?3,mode=?4,soft_limit=?5,scope=?6,store_op=COALESCE(store_op,?7),store_state=CASE WHEN store_state='none' THEN 'pending' ELSE store_state END,status=CASE WHEN ?2 THEN 'ready_to_create' ELSE 'disconnected' END,paused=CASE WHEN ?2 THEN paused ELSE 1 END,sync_requested=CASE WHEN started=1 THEN 1 ELSE 0 END,revision=revision+1,error=NULL,next_at=0,attempts=0 WHERE project_id=?1",params![id,draft.enabled,draft.automatic,draft.mode,draft.soft_limit,bound,uuid::Uuid::new_v4().to_string()]).map_err(error)?;
    activity(
        &tx,
        id,
        "consent",
        "Cloud selection and retention policy explicitly confirmed.",
    )?;
    tx.commit().map_err(error)
}
pub fn jobs(db: &Connection, id: &str) -> Result<Vec<Value>> {
    rows(db,"SELECT j.*,v.number,v.size rawBytes,d.family_name familyName FROM cloud_jobs j JOIN document_versions v ON v.id=j.version_id JOIN documents d ON d.id=j.document_id WHERE j.project_id=?1 ORDER BY d.family_name,v.number",&[&id])
}
pub fn view(db: &Connection, id: &str) -> Result<Value> {
    let mut p = project(db, id)?;
    if n(&p, "enabled") == 1 && s(&p, "store_state") != "deleted" {
        let connection = rows(
            db,
            "SELECT status,scope FROM cloud_connection WHERE singleton=1",
            &[],
        )?
        .pop()
        .ok_or("Connection state unavailable.")?;
        if s(&connection, "status") != "connected" {
            p["status"] = json!("key_disconnected");
            p["error"] = json!("Save/test the OpenAI key in Settings to resume cloud operations.");
        } else if p["scope"] != connection["scope"] {
            p["status"] = json!("credential_changed");
            p["error"] =
                json!("Verify this index with the tested replacement key before resuming.");
        } else if n(&p, "paused") == 1 && n(&p, "delete_requested") != 1 {
            p["status"] = json!("paused");
        }
    }
    let js = jobs(db, id)?;
    p["families"] = json!(families(db, id)?);
    p["activity"]=json!(rows(db,"SELECT kind,message,created_at FROM cloud_activity WHERE project_id=?1 ORDER BY id DESC LIMIT 30",&[&id])?);
    p["indexed"] = json!(js.iter().filter(|j| s(j, "state") == "indexed").count());
    p["pending"] = json!(js
        .iter()
        .filter(
            |j| !["indexed", "removed", "purged", "failed", "uncertain"].contains(&s(j, "state"))
        )
        .count());
    p["failed"] = json!(js
        .iter()
        .filter(|j| ["failed", "uncertain"].contains(&s(j, "state")))
        .count());
    p["rawBytes"] = json!(js
        .iter()
        .filter(|j| s(j, "state") == "indexed")
        .map(|j| n(j, "rawBytes"))
        .sum::<i64>());
    p["retainedFiles"] = json!(js
        .iter()
        .filter(|j| !s(j, "file_id").is_empty() && s(j, "state") != "purged")
        .count());
    p["jobs"] = json!(js);
    p.as_object_mut()
        .expect("project row object")
        .remove("scope");
    Ok(p)
}
pub fn queue(db: &mut Connection, id: &str) -> Result<()> {
    let tx = db
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(error)?;
    let p = project(&tx, id)?;
    let draft = draft(&tx, &p)?;
    let wanted = desired(&tx, id, &draft)?;
    let install = install(&tx)?;
    tx.execute("UPDATE cloud_jobs SET desired=0 WHERE project_id=?1", [id])
        .map_err(error)?;
    for v in wanted {
        let job = uuid::Uuid::new_v4().to_string();
        let ext = if v.kind == "word" { "docx" } else { "md" };
        tx.execute("INSERT INTO cloud_jobs(id,project_id,document_id,version_id,remote_name,updated_at) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(project_id,version_id) DO UPDATE SET desired=1,state=CASE WHEN state='removed' THEN CASE WHEN file_id IS NULL THEN 'pending' ELSE 'uploaded' END ELSE state END",params![job,id,v.document,v.id,format!("cd_{install}_{id}_{}_{job}.{ext}",v.id),models::now()]).map_err(error)?;
    }
    tx.commit().map_err(error)
}
pub fn verified(root: &Path, v: &CloudVersion, limit: i64) -> Result<Vec<u8>> {
    if v.size <= 0 || v.size > limit || v.size as usize > extraction::MAX_SNAPSHOT {
        return Err("Snapshot exceeds the configured soft limit or the 16 MiB application safety limit (empty files are unsupported).".into());
    }
    let path = storage::checked(root, &v.snapshot, false)?;
    let file = storage::locked_read(&path.path, false)?;
    let mut bytes = Vec::new();
    file.take(extraction::MAX_SNAPSHOT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read immutable snapshot.".to_string())?;
    if bytes.len() as i64 != v.size || format!("{:x}", Sha256::digest(&bytes)) != v.hash {
        return Err("Snapshot integrity verification failed. No bytes uploaded.".into());
    }
    extraction::extract(&bytes, &v.kind)?;
    // Bound all DOCX XML, including parts omitted by the local text extractor.
    if v.kind == "word" {
        let archive = zip::ZipArchive::new(std::io::Cursor::new(&bytes))
            .map_err(|_| "Invalid DOCX snapshot")?;
        let xml = archive.decompressed_size().unwrap_or(u128::MAX);
        if xml > 4 * 1024 * 1024 {
            return Err(
                "DOCX expanded content exceeds the conservative token-screening limit.".into(),
            );
        }
    }
    Ok(bytes)
}
pub fn version(db: &Connection, id: &str) -> Result<CloudVersion> {
    db.query_row("SELECT v.id,v.document_id,v.number,d.kind,v.sha256,v.snapshot_relative,v.size,v.id=d.current_version_id FROM document_versions v JOIN documents d ON d.id=v.document_id WHERE v.id=?1",[id],|r|Ok(CloudVersion{id:r.get(0)?,document:r.get(1)?,number:r.get(2)?,kind:r.get(3)?,hash:r.get(4)?,snapshot:r.get(5)?,size:r.get(6)?,current:r.get(7)?})).map_err(error)
}
pub fn owned_store(
    api: &dyn CloudApi,
    db: &Connection,
    p: &Value,
) -> std::result::Result<Value, ApiError> {
    let id = s(p, "store_id");
    cloud_api::valid_id(id, "vs_")?;
    let store = api.request("GET", &format!("/vector_stores/{id}"), None)?;
    if store["metadata"]["contextdock"]
        != install(db).map_err(|_| ApiError::local("Installation identity unavailable."))?
        || store["metadata"]["project"] != p["project_id"]
        || store["metadata"]["operation"] != p["store_op"]
    {
        return Err(ApiError::local(
            "Remote index ownership does not match this project. No changes made.",
        ));
    }
    Ok(store)
}
