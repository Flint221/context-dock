//! Cloud state has its own short-lived SQLite connection. No HTTP holds the local registry lock.
use crate::{
    cloud_api::{self, ApiError, CloudApi},
    cloud_models::{CloudDraft, CloudPlan, CloudVersion},
    extraction, models,
    registry::{error, Result},
    storage,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{io::Read, path::Path};
pub const SOURCE_MISSING: &str = "Selected project files are missing or inaccessible. Restore them and refresh documents before syncing or asking questions.";
pub const SOURCE_BUSY: &str = "A selected document is temporarily locked or unreadable. Close the editor or wait for saving to finish; automatic sync will resume once it is readable.";
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
const ELIGIBLE:&str="d.tracking_state='tracked' AND d.merged_into IS NULL AND d.current_version_id IS NOT NULL AND d.kind IN ('markdown','word') AND NOT EXISTS(SELECT 1 FROM operations o WHERE o.document_id=d.id AND o.kind='initial_import' AND o.state!='done') AND NOT EXISTS(SELECT 1 FROM pending_matches m JOIN source_files sf ON sf.id=m.source_id WHERE sf.document_id=d.id OR EXISTS(SELECT 1 FROM json_each(m.candidate_ids) j WHERE j.value=d.id))";
pub fn families(db: &Connection, id: &str) -> Result<Vec<Value>> {
    let mut families=rows(db,&format!("SELECT d.id,d.family_name name,d.tracking_state trackingState,({ELIGIBLE}) eligible, EXISTS(SELECT 1 FROM cloud_documents cd WHERE cd.project_id=d.project_id AND cd.document_id=d.id) selected,(SELECT number FROM document_versions WHERE id=d.current_version_id) currentNumber,(SELECT MAX(v.number) FROM cloud_jobs j JOIN document_versions v ON v.id=j.version_id WHERE j.document_id=d.id AND j.state='indexed' AND j.remote_current=1) cloudNumber,(SELECT COUNT(*) FROM document_versions WHERE document_id=d.id) versionCount,(SELECT COALESCE(SUM(size),0) FROM document_versions WHERE document_id=d.id) rawBytes FROM documents d WHERE d.project_id=?1 AND d.merged_into IS NULL ORDER BY d.family_name"),&[&id])?;
    let blocked = crate::path_selection::blocked_documents(db, id)?;
    for family in &mut families {
        family["versionCount"] = json!(crate::versioning::heads(db, s(family, "id"))?.len());
        let excluded = blocked.contains(s(family, "id"));
        family["pathExcluded"] = json!(excluded as i64);
        // A previously captured selected family must not silently disappear
        // from the desired index if tracking state becomes inconsistent.
        // Keep it selected and let eligibility freeze sync for review.
        family["selected"] = json!(
            (!excluded
                && (s(family, "trackingState") == "tracked" || n(family, "versionCount") > 0))
                as i64
        );
        if excluded {
            family["eligible"] = json!(0);
        }
    }
    Ok(families)
}
pub fn desired(db: &Connection, id: &str, draft: &CloudDraft) -> Result<Vec<CloudVersion>> {
    desired_selection(db, id, draft, true)
}
fn desired_selection(
    db: &Connection,
    id: &str,
    draft: &CloudDraft,
    check_sources: bool,
) -> Result<Vec<CloudVersion>> {
    let blocked = crate::path_selection::blocked_versions(db, id)?;
    let path_rules = crate::path_selection::rules(db, id)?;
    if check_sources {
        crate::folder_recovery::require_ready(db, id)?;
    }
    let root: String = db
        .query_row("SELECT path FROM projects WHERE id=?1", [id], |r| r.get(0))
        .map_err(error)?;
    let selected: Vec<String> = families(db, id)?
        .iter()
        .filter(|f| n(f, "selected") == 1)
        .map(|f| s(f, "id").to_string())
        .collect();
    if check_sources
        && families(db, id)?
            .iter()
            .any(|f| n(f, "selected") == 1 && n(f, "eligible") != 1)
    {
        return Err("A selected tracked document needs local tracking review before cloud sync; existing index entries are retained.".into());
    }
    for document in selected.iter().filter(|_| check_sources) {
        let source:Option<(String,String,Option<i64>)>=db.query_row("SELECT s.relative_path,s.status,s.hashed_size FROM documents d JOIN source_files s ON s.id=d.active_source_id WHERE d.id=?1 AND d.project_id=?2",params![document,id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(error)?;
        if let Some((relative, status, hashed_size)) = source {
            if !crate::path_selection::permits(&path_rules, &relative) {
                continue;
            }
            let source = storage::checked(Path::new(&root), &relative, false);
            if status != "available" || source.is_err() {
                return Err(SOURCE_MISSING.into());
            }
            let source = source?;
            if !source.path.is_file() {
                return Err(SOURCE_MISSING.into());
            }
            if storage::locked_read(&source.path, false).is_err() {
                return Err(SOURCE_BUSY.into());
            }
            if hashed_size.is_none() {
                return Err("Recheck the reassigned folder's documents before syncing or asking questions. Use Refresh documents to capture any changed files.".into());
            }
        } else {
            return Err("A selected tracked document has no active source. Resolve its local tracking state before cloud sync; existing index entries are retained.".into());
        }
    }
    if !["current", "history"].contains(&draft.mode.as_str())
        || !(1..=512 * 1024 * 1024).contains(&draft.soft_limit)
    {
        return Err("Choose a valid mode and a soft limit between 1 byte and 512 MiB.".into());
    }
    let eligibility = if check_sources { ELIGIBLE } else { "1=1" };
    let mut q=db.prepare(&format!("SELECT v.id,v.document_id,v.number,d.kind,v.sha256,v.snapshot_relative,v.size,v.id=d.current_version_id FROM document_versions v JOIN documents d ON d.id=v.document_id WHERE d.project_id=?1 AND {eligibility} ORDER BY d.id,v.number")).map_err(error)?;
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
    let mut heads = std::collections::HashSet::new();
    for document in &selected {
        heads.extend(crate::versioning::heads(db, document)?);
    }
    Ok(versions
        .into_iter()
        .filter(|v| {
            heads.contains(&v.id)
                && !blocked.contains(&v.id)
                && selected.contains(&v.document)
                && (draft.mode == "history" || v.current)
        })
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
pub fn selection_reduction(_db: &Connection, _p: &Value, _requested: &CloudDraft) -> Result<bool> {
    // Document selection now comes exclusively from saved paths; a client-supplied
    // list can never authorize the legacy opt-out / reduced-validation path.
    Ok(false)
}
pub fn preview(db: &Connection, id: &str, draft: &CloudDraft) -> Result<CloudPlan> {
    let p = project(db, id)?;
    let reduction = selection_reduction(db, &p, draft)?;
    let versions = desired_selection(db, id, draft, !reduction)?;
    if draft.enabled && n(&p, "started") == 0 && versions.is_empty() && !reduction {
        return Err("Select at least one eligible document before enabling cloud indexing. No configuration was saved.".into());
    }
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
    let p = project(&tx, id)?;
    let reduction = selection_reduction(&tx, &p, draft)?;
    let bound: String = if reduction {
        s(&p, "scope").into()
    } else {
        tx.query_row("SELECT COALESCE(scope,'') FROM cloud_connection WHERE singleton=1 AND status='connected'",[],|r|r.get(0)).map_err(|_|"Test the saved OpenAI key successfully before enabling cloud indexing.".to_string())?
    };
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
    let selection = families(&tx, id)?
        .into_iter()
        .filter(|f| n(f, "selected") == 1)
        .map(|f| s(&f, "id").to_string())
        .collect::<Vec<_>>();
    // Existing cloud_documents rows are retained for compatibility, but never
    // authorize a document or schedule a removal under the unified selection.
    tx.execute("UPDATE cloud_projects SET enabled=?2,auto_sync=1,mode=?3,soft_limit=?4,scope=?5,store_op=COALESCE(store_op,?6),store_state=CASE WHEN store_state='none' THEN 'pending' ELSE store_state END,status=CASE WHEN ?2 THEN 'ready_to_create' ELSE 'disconnected' END,paused=CASE WHEN ?2 THEN paused ELSE 1 END,sync_requested=CASE WHEN started=1 THEN 1 ELSE 0 END,revision=revision+1,error=NULL,next_at=0,attempts=0 WHERE project_id=?1",params![id,draft.enabled,draft.mode,draft.soft_limit,bound,uuid::Uuid::new_v4().to_string()]).map_err(error)?;
    activity(
        &tx,
        id,
        "consent",
        "Cloud selection and retention policy explicitly confirmed.",
    )?;
    verify_flags(
        &tx,
        id,
        &[
            ("enabled", draft.enabled as i64),
            ("auto_sync", 1),
            ("soft_limit", draft.soft_limit),
        ],
    )?;
    let saved = project(&tx, id)?;
    let mut selected = families(&tx, id)?
        .iter()
        .filter(|f| n(f, "selected") == 1)
        .map(|f| s(f, "id").to_string())
        .collect::<Vec<_>>();
    let mut requested = selection;
    selected.sort();
    requested.sort();
    if s(&saved, "mode") != draft.mode || selected != requested {
        return Err(
            "Cloud configuration verification failed. The requested selection was not saved."
                .into(),
        );
    }
    tx.commit().map_err(error)
}
pub fn verify_flags(db: &Connection, id: &str, expected: &[(&str, i64)]) -> Result<()> {
    let saved = project(db, id)?;
    if expected
        .iter()
        .any(|(key, value)| saved[*key].as_i64() != Some(*value))
    {
        return Err("Cloud setting verification failed. The backend did not save the requested state; refresh status before trying again.".into());
    }
    Ok(())
}
pub fn jobs(db: &Connection, id: &str) -> Result<Vec<Value>> {
    rows(db,"SELECT j.*,v.number,v.source_name sourceName,v.source_relative sourcePath,v.size rawBytes,d.family_name familyName FROM cloud_jobs j JOIN document_versions v ON v.id=j.version_id JOIN documents d ON d.id=j.document_id WHERE j.project_id=?1 ORDER BY d.family_name,v.number",&[&id])
}
pub fn selection_removals(db: &Connection, id: &str) -> Result<Vec<Value>> {
    let blocked = crate::path_selection::blocked_versions(db, id)?;
    Ok(rows(db,"SELECT j.*,v.number,v.source_name sourceName,v.size rawBytes,d.family_name familyName FROM cloud_jobs j JOIN document_versions v ON v.id=j.version_id JOIN documents d ON d.id=j.document_id WHERE j.project_id=?1 AND j.phase IN ('selection_remove','path_remove') AND j.state NOT IN ('removed','purged') AND j.desired=0 ORDER BY j.id",&[&id])?.into_iter().filter(|j|blocked.contains(s(j,"version_id"))).collect())
}
/// Only the exact GET/DELETE routes needed to complete saved document opt-outs.
pub fn selection_removal_request(
    db: &Connection,
    id: &str,
    method: &str,
    path: &str,
) -> Result<bool> {
    if !["GET", "DELETE"].contains(&method) {
        return Ok(false);
    }
    let p = project(db, id)?;
    let jobs = selection_removals(db, id)?;
    if jobs.is_empty() || s(&p, "store_id").is_empty() {
        return Ok(false);
    }
    let store = format!("/vector_stores/{}", s(&p, "store_id"));
    Ok((method == "GET" && path == store)
        || jobs.iter().any(|j| {
            !s(j, "file_id").is_empty() && path == format!("{store}/files/{}", s(j, "file_id"))
        }))
}
pub fn view(db: &Connection, id: &str) -> Result<Value> {
    let mut p = project(db, id)?;
    let connection = rows(
        db,
        "SELECT status,scope FROM cloud_connection WHERE singleton=1",
        &[],
    )?
    .pop()
    .ok_or("Connection state unavailable.")?;
    p["connectionStatus"] = connection["status"].clone();
    if n(&p, "enabled") == 1
        && s(&p, "store_state") != "deleted"
        && ![
            "cleanup_pending",
            "cleanup_failed",
            "selection_removal_failed",
        ]
        .contains(&s(&p, "status"))
    {
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
    if p["error"].is_null() {
        if let Some(job) = js.iter().find(|j| !s(j, "error").is_empty()) {
            p["error"] = job["error"].clone();
        }
    }
    p["selectionSaved"] = json!(!s(&p, "scope").is_empty());
    p["selectionRemovalPending"] = json!(selection_removals(db, id)?.len());
    let local = desired(db, id, &draft(db, &p)?);
    p["localReady"] = json!(local.is_ok());
    p["localError"] = json!(local.err());
    p["families"] = json!(families(db, id)?);
    p["activity"]=json!(rows(db,"SELECT kind,message,created_at FROM cloud_activity WHERE project_id=?1 ORDER BY id DESC LIMIT 30",&[&id])?);
    p["indexed"] = json!(js
        .iter()
        .filter(|j| n(j, "desired") == 1 && s(j, "state") == "indexed")
        .count());
    p["pending"] = json!(js
        .iter()
        .filter(|j| n(j, "desired") == 1
            && !["indexed", "removed", "purged", "failed", "uncertain"].contains(&s(j, "state")))
        .count());
    p["failed"] = json!(js
        .iter()
        .filter(|j| n(j, "desired") == 1 && ["failed", "uncertain"].contains(&s(j, "state")))
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
        tx.execute("UPDATE cloud_jobs SET phase=CASE WHEN file_id IS NOT NULL THEN 'attach' ELSE phase END,state=CASE WHEN file_id IS NOT NULL THEN 'uploaded' ELSE state END,desired=1,next_at=0,error=NULL WHERE project_id=?1 AND version_id=?2 AND state!='purged' AND phase IN ('selection_remove','path_remove','remove')", params![id,v.id]).map_err(error)?;
        let job = uuid::Uuid::new_v4().to_string();
        let ext = if v.kind == "word" { "docx" } else { "md" };
        tx.execute("INSERT INTO cloud_jobs(id,project_id,document_id,version_id,remote_name,updated_at) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(project_id,version_id) DO UPDATE SET desired=1,remote_name=CASE WHEN state='purged' THEN excluded.remote_name ELSE remote_name END,phase=CASE WHEN state='purged' THEN 'upload' ELSE phase END,state=CASE WHEN state='purged' THEN 'pending' WHEN state='removed' THEN CASE WHEN file_id IS NOT NULL THEN 'uploaded' WHEN phase='recover' THEN 'uploading' ELSE 'pending' END ELSE state END,next_at=CASE WHEN state='purged' THEN 0 ELSE next_at END,error=CASE WHEN state='purged' THEN NULL ELSE error END",params![job,id,v.document,v.id,format!("cd_{install}_{id}_{}_{job}.{ext}",v.id),models::now()]).map_err(error)?;
    }
    tx.commit().map_err(error)
}
pub fn verified(root: &Path, v: &CloudVersion, limit: i64) -> Result<Vec<u8>> {
    if v.size <= 0 || v.size > limit || v.size as usize > extraction::MAX_SNAPSHOT {
        return Err(format!("Snapshot is {} bytes; cloud uploads require a nonempty file within the configured {}-byte limit and the 16 MiB application limit. No bytes uploaded.", v.size, limit));
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
    // Extraction already validates the 128 MiB expanded package, entry/ratio,
    // body XML and extracted-text limits. Package bytes include images and are
    // not a token count; do not impose the former 4 MiB package screen here.
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
