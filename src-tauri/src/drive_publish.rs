//! One Drive identity per tracked family. Persist intent before remote writes.
use crate::{
    drive_api::{self, ApiError, DriveApi, Remote},
    registry::{error, Registry},
    storage,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::json;
use std::{collections::BTreeMap, path::Path};
type Result<T> = drive_api::Result<T>;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Publication {
    pub document_id: String,
    pub file_id: String,
    pub verified_hash: Option<String>,
    pub verified_name: Option<String>,
    pub verified_size: Option<i64>,
    pub verified_revision: Option<String>,
    pub verified_at: Option<String>,
    pub pending_hash: Option<String>,
    pub pending_name: Option<String>,
    pub pending_size: Option<i64>,
    pub pending_version: Option<String>,
    pub pending_source: Option<String>,
    pub pending_relative: Option<String>,
    #[serde(skip)]
    pub pending_stage: Option<String>,
    pub pending_generation: Option<String>,
    pub state: String,
    pub attempts: i64,
    pub next_at: i64,
    pub error: Option<String>,
}
pub fn publications(db: &Connection, project: &str) -> Result<Vec<Publication>> {
    let mut query = db.prepare("SELECT document_id,file_id,verified_hash,verified_name,verified_size,verified_revision,verified_at,pending_hash,pending_name,pending_size,pending_version,pending_source,pending_relative,pending_stage,pending_generation,state,attempts,next_at,error FROM drive_documents WHERE project_id=?1 ORDER BY document_id").map_err(error)?;
    let rows = query
        .query_map([project], |r| {
            Ok(Publication {
                document_id: r.get(0)?,
                file_id: r.get(1)?,
                verified_hash: r.get(2)?,
                verified_name: r.get(3)?,
                verified_size: r.get(4)?,
                verified_revision: r.get(5)?,
                verified_at: r.get(6)?,
                pending_hash: r.get(7)?,
                pending_name: r.get(8)?,
                pending_size: r.get(9)?,
                pending_version: r.get(10)?,
                pending_source: r.get(11)?,
                pending_relative: r.get(12)?,
                pending_stage: r.get(13)?,
                pending_generation: r.get(14)?,
                state: r.get(15)?,
                attempts: r.get(16)?,
                next_at: r.get(17)?,
                error: r.get(18)?,
            })
        })
        .map_err(error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(error)?;
    Ok(rows)
}
fn publication(db: &Connection, project: &str, document: &str) -> Result<Option<Publication>> {
    Ok(publications(db, project)?
        .into_iter()
        .find(|p| p.document_id == document))
}
pub fn configure(
    db: &Connection,
    project: &str,
    account: &str,
    enabled: bool,
    automatic: bool,
) -> Result<()> {
    let tx = db.unchecked_transaction().map_err(error)?;
    let tracking: bool = tx
        .query_row(
            "SELECT tracking_enabled FROM projects WHERE id=?1 AND removed_at IS NULL",
            [project],
            |r| r.get(0),
        )
        .map_err(error)?;
    if enabled {
        crate::folder_recovery::require_ready(&tx, project)?;
        if !tracking {
            return Err(ApiError::local(
                "Enable file tracking before publishing this project.",
            ));
        }
    }
    let existing: Option<String> = tx
        .query_row(
            "SELECT account_id FROM drive_projects WHERE project_id=?1",
            [project],
            |r| r.get(0),
        )
        .optional()
        .map_err(error)?;
    if existing.is_some_and(|id| id != account) {
        return Err(ApiError::local("This project is bound to another Google account. Reconnect that account to preserve its file identities."));
    }
    tx.execute(
        "INSERT OR IGNORE INTO drive_accounts(account_id) VALUES(?1)",
        [account],
    )
    .map_err(error)?;
    tx.execute("INSERT INTO drive_projects(project_id,account_id,enabled,automatic) VALUES(?1,?2,?3,?4) ON CONFLICT(project_id) DO UPDATE SET enabled=excluded.enabled,automatic=excluded.automatic,error=NULL",params![project,account,enabled,automatic]).map_err(error)?;
    let saved: (bool, bool) = tx
        .query_row(
            "SELECT enabled,automatic FROM drive_projects WHERE project_id=?1",
            [project],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(error)?;
    if saved != (enabled, automatic) {
        return Err(ApiError::local(
            "The publication settings did not save. Refresh and retry.",
        ));
    }
    tx.commit().map_err(error)?;
    Ok(())
}
pub fn require_enabled(db: &Connection, project: &str, account: &str) -> Result<()> {
    let enabled: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM drive_projects dp JOIN projects p ON p.id=dp.project_id WHERE dp.project_id=?1 AND dp.account_id=?2 AND dp.enabled=1 AND p.tracking_enabled=1 AND p.removed_at IS NULL)",params![project,account],|r|r.get(0)).map_err(error)?;
    if !enabled {
        return Err(ApiError::local(
            "Drive publication or tracking is paused, or this account does not match the project.",
        ));
    }
    crate::folder_recovery::require_ready(db, project)?;
    Ok(())
}
fn properties(
    db: &Connection,
    project: &str,
    document: Option<&str>,
) -> Result<BTreeMap<String, String>> {
    let mut map = BTreeMap::from([
        ("contextdock".into(), crate::cloud::install(db)?),
        ("project".into(), project.into()),
    ]);
    if let Some(document) = document {
        map.insert("document".into(), document.into());
    }
    Ok(map)
}
fn folder_valid(remote: &Remote, id: &str, parent: &str) -> Result<()> {
    if remote.id != id
        || remote.mime_type != drive_api::FOLDER
        || remote.trashed
        || (parent != "root" && remote.parents != [parent])
    {
        return Err(ApiError::local(
            "The mapped Drive folder identity or location changed. Review the destination.",
        ));
    }
    Ok(())
}
fn ensure_folder(
    db: &Connection,
    account: &str,
    project: Option<&str>,
    parent: &str,
    name: &str,
    api: &dyn DriveApi,
) -> Result<String> {
    let (sql, save, ready, key) = if let Some(project) = project {
        (
            "SELECT folder_id,folder_ready FROM drive_projects WHERE project_id=?1",
            "UPDATE drive_projects SET folder_id=?2,folder_ready=0 WHERE project_id=?1",
            "UPDATE drive_projects SET folder_ready=1 WHERE project_id=?1",
            project,
        )
    } else {
        (
            "SELECT root_id,root_ready FROM drive_accounts WHERE account_id=?1",
            "UPDATE drive_accounts SET root_id=?2,root_ready=0 WHERE account_id=?1",
            "UPDATE drive_accounts SET root_ready=1 WHERE account_id=?1",
            account,
        )
    };
    let (mut id, mut was_ready): (Option<String>, bool) = db
        .query_row(sql, [key], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(error)?;
    if id.is_none() {
        let folders = api.folders(parent, name)?;
        if folders.len() > 1 {
            return Err(ApiError::local(
                "Multiple matching Drive folders need review. No additional folder was created.",
            ));
        }
        id = Some(match folders.into_iter().next() {
            Some(folder) => {
                folder_valid(&folder, &folder.id, parent)?;
                if project.is_some_and(|p| {
                    folder
                        .app_properties
                        .get("project")
                        .is_some_and(|id| id != p)
                }) {
                    return Err(ApiError::local("A same-name Drive folder belongs to another tracked project. Choose a distinct project name."));
                }
                was_ready = true;
                folder.id
            }
            None => {
                if project.is_none() {
                    let allowed: bool = db
                        .query_row(
                            "SELECT create_root FROM drive_accounts WHERE account_id=?1",
                            [account],
                            |r| r.get(0),
                        )
                        .map_err(error)?;
                    if !allowed {
                        return Err(ApiError::local("Choose your existing ContextDock folder in Settings, or explicitly create Drive folders here before the first publication."));
                    }
                }
                api.generate_id()?
            }
        });
        drive_api::valid_id(id.as_deref().unwrap())?;
        db.execute(save, params![key, id]).map_err(error)?;
        let persisted: Option<String> = db.query_row(sql, [key], |r| r.get(0)).map_err(error)?;
        if persisted != id {
            return Err(ApiError::local(
                "The Drive folder identity was not saved. No folder was created.",
            ));
        }
        if was_ready {
            db.execute(ready, [key]).map_err(error)?;
            let ready_saved: bool = db.query_row(sql, [key], |r| r.get(1)).map_err(error)?;
            if !ready_saved {
                return Err(ApiError::local(
                    "The adopted folder receipt did not save. No new folder was created.",
                ));
            }
        }
    }
    let id = id.unwrap();
    let remote = match api.get(&id) {
        Ok(remote) => remote,
        Err(e) if e.missing && !was_ready => api.create_folder(
            &id,
            parent,
            name,
            &properties(db, project.unwrap_or("root"), None)?,
        )?,
        Err(e) => return Err(e),
    };
    folder_valid(&remote, &id, parent)?;
    db.execute(ready, [key]).map_err(error)?;
    Ok(id)
}
pub fn ensure_destination(
    db: &Connection,
    project: &str,
    account: &str,
    api: &dyn DriveApi,
) -> Result<String> {
    require_enabled(db, project, account)?;
    let root = ensure_folder(db, account, None, "root", "ContextDock", api)?;
    let name: String = db
        .query_row("SELECT name FROM projects WHERE id=?1", [project], |r| {
            r.get(0)
        })
        .map_err(error)?;
    ensure_folder(db, account, Some(project), &root, &name, api)
}
pub fn adopt_root(db: &Connection, account: &str, id: &str, api: &dyn DriveApi) -> Result<()> {
    drive_api::valid_id(id)?;
    let folder = api.get(id)?;
    if folder.trashed || folder.mime_type != drive_api::FOLDER || folder.name != "ContextDock" {
        return Err(ApiError::local(
            "Choose the existing folder named ContextDock in My Drive.",
        ));
    }
    if !api
        .folders("root", "ContextDock")?
        .iter()
        .any(|r| r.id == id)
    {
        return Err(ApiError::local(
            "Choose a ContextDock folder directly under My Drive.",
        ));
    }
    let tx = db.unchecked_transaction().map_err(error)?;
    let previous: Option<String> = tx
        .query_row(
            "SELECT root_id FROM drive_accounts WHERE account_id=?1",
            [account],
            |r| r.get(0),
        )
        .optional()
        .map_err(error)?
        .flatten();
    let used:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM drive_projects WHERE account_id=?1 AND folder_id IS NOT NULL)",[account],|r|r.get(0)).map_err(error)?;
    if used && previous.as_deref() != Some(id) {
        return Err(ApiError::local("This account already has mapped project folders. Keep its existing root to preserve cloud identities."));
    }
    tx.execute("INSERT INTO drive_accounts(account_id,root_id,root_ready) VALUES(?1,?2,1) ON CONFLICT(account_id) DO UPDATE SET root_id=excluded.root_id,root_ready=1",params![account,id]).map_err(error)?;
    let saved: Option<String> = tx
        .query_row(
            "SELECT root_id FROM drive_accounts WHERE account_id=?1 AND root_ready=1",
            [account],
            |r| r.get(0),
        )
        .optional()
        .map_err(error)?
        .flatten();
    if saved.as_deref() != Some(id) {
        return Err(ApiError::local("The selected Drive folder did not save."));
    }
    tx.commit().map_err(error)?;
    Ok(())
}
#[derive(Clone)]
pub struct Current {
    pub document: String,
    pub version: String,
    pub source: String,
    pub name: String,
    pub relative: String,
    pub hash: String,
    pub size: i64,
    pub root: String,
}
pub fn current(db: &Connection, project: &str, document: &str) -> Result<Current> {
    let current = db.query_row("SELECT d.id,d.current_version_id,s.id,s.name,s.relative_path,v.sha256,v.size,p.path FROM documents d JOIN projects p ON p.id=d.project_id JOIN source_files s ON s.id=d.active_source_id AND s.document_id=d.id JOIN document_versions v ON v.id=d.current_version_id WHERE d.id=?1 AND d.project_id=?2 AND d.merged_into IS NULL AND d.tracking_state='tracked' AND s.status='available' AND d.kind IN ('markdown','word') AND NOT EXISTS(SELECT 1 FROM operations o WHERE o.document_id=d.id AND o.state!='done') AND NOT EXISTS(SELECT 1 FROM pending_matches m JOIN source_files sf ON sf.id=m.source_id WHERE sf.document_id=d.id OR EXISTS(SELECT 1 FROM json_each(m.candidate_ids) j WHERE j.value=d.id))",params![document,project],|r|Ok(Current{document:r.get(0)?,version:r.get(1)?,source:r.get(2)?,name:r.get(3)?,relative:r.get(4)?,hash:r.get(5)?,size:r.get(6)?,root:r.get(7)?})).map_err(|_|ApiError {retry:true,waiting:true,..ApiError::local("Waiting for a stable, selected document without unresolved duplicates or moves.")})?;
    if !crate::path_selection::allowed(db, project, &current.relative)? {
        return Err(ApiError::local(
            "This document is excluded from the saved folder selection.",
        ));
    }
    Ok(current)
}
pub fn eligible_documents(db: &Connection, project: &str) -> Result<Vec<String>> {
    let mut q = db.prepare("SELECT id FROM documents WHERE project_id=?1 AND tracking_state='tracked' AND merged_into IS NULL ORDER BY family_name").map_err(error)?;
    let all = q
        .query_map([project], |r| r.get(0))
        .map_err(error)?
        .collect::<std::result::Result<Vec<String>, _>>()
        .map_err(error)?;
    Ok(all
        .into_iter()
        .filter(|id| current(db, project, id).is_ok())
        .collect())
}
pub fn generation(current: &Current) -> String {
    crate::cloud::scope(&format!(
        "{}:{}:{}:{}:{}:{}",
        current.document,
        current.version,
        current.source,
        current.name,
        current.relative,
        current.hash
    ))
}
fn owns(
    db: &Connection,
    project: &str,
    document: &str,
    parent: &str,
    file: &str,
    remote: &Remote,
) -> Result<()> {
    if remote.id != file
        || remote.trashed
        || remote.parents != [parent]
        || remote.mime_type.starts_with("application/vnd.google-apps.")
        || properties(db, project, Some(document))?
            .iter()
            .any(|(k, v)| remote.app_properties.get(k) != Some(v))
    {
        return Err(ApiError::local("The mapped cloud file has a different identity, owner marker or parent folder. Nothing was overwritten."));
    }
    Ok(())
}
fn clear_pending(db: &Connection, store: &Path, p: &Publication) -> Result<()> {
    db.execute("UPDATE drive_documents SET pending_hash=NULL,pending_name=NULL,pending_size=NULL,pending_version=NULL,pending_source=NULL,pending_relative=NULL,pending_stage=NULL,pending_generation=NULL WHERE document_id=?1",[&p.document_id]).map_err(error)?;
    if let Some(stage) = &p.pending_stage {
        let _ = storage::discard_stage(store, stage);
    }
    Ok(())
}
fn receipt(db: &Connection, p: &Publication, remote: &Remote, hash: &str) -> Result<()> {
    db.execute("UPDATE drive_documents SET verified_hash=?2,verified_name=?3,verified_size=?4,verified_revision=?5,verified_at=?6,state='queued',attempts=0,next_at=0,error=NULL WHERE document_id=?1",params![p.document_id,hash,remote.name,remote.size.as_deref().and_then(|s|s.parse::<i64>().ok()),remote.version,crate::models::now()]).map_err(error)?;
    let saved: (Option<String>,Option<String>,Option<String>) = db.query_row("SELECT verified_hash,verified_name,verified_revision FROM drive_documents WHERE document_id=?1",[&p.document_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(error)?;
    if saved
        != (
            Some(hash.into()),
            Some(remote.name.clone()),
            remote.version.clone(),
        )
    {
        return Err(ApiError::local(
            "The verified upload receipt did not save. Publication remains incomplete.",
        ));
    }
    Ok(())
}
struct Expected<'a> {
    hash: &'a str,
    name: &'a str,
    size: i64,
}
fn verify_remote(
    db: &Connection,
    project: &str,
    parent: &str,
    p: &Publication,
    api: &dyn DriveApi,
    expected: Expected<'_>,
) -> Result<Remote> {
    let before = api.get(&p.file_id)?;
    owns(db, project, &p.document_id, parent, &p.file_id, &before)?;
    if before.name != expected.name
        || before.size.as_deref().and_then(|n| n.parse::<i64>().ok()) != Some(expected.size)
    {
        return Err(ApiError::local(
            "The uploaded cloud filename or size did not match. Verification remains incomplete.",
        ));
    }
    if api.download_hash(&p.file_id, expected.size)? != expected.hash {
        return Err(ApiError::local(
            "The downloaded cloud contents did not match. No file was marked synced or recycled.",
        ));
    }
    let after = api.get(&p.file_id)?;
    owns(db, project, &p.document_id, parent, &p.file_id, &after)?;
    if before.version.is_none()
        || before.version != after.version
        || before.name != after.name
        || before.size != after.size
    {
        return Err(ApiError {
            retry: true,
            ..ApiError::local(
                "The cloud file changed during verification. It remains queued for another check.",
            )
        });
    }
    Ok(after)
}
/// One invocation publishes at most one current document. Every attempted write
/// has a reserved ID and pending generation in SQLite before it reaches Drive.
pub fn publish(
    registry: &Registry,
    project: &str,
    account: &str,
    document: &str,
    parent: &str,
    api: &dyn DriveApi,
) -> Result<()> {
    let db = &registry.db;
    require_enabled(db, project, account)?;
    let destination: Option<String> = db
        .query_row(
            "SELECT folder_id FROM drive_projects WHERE project_id=?1 AND folder_ready=1",
            [project],
            |r| r.get(0),
        )
        .optional()
        .map_err(error)?
        .flatten();
    if destination.as_deref() != Some(parent) {
        return Err(ApiError::local(
            "The verified project destination does not match this publication.",
        ));
    }
    if publication(db, project, document)?.is_none() {
        let c = current(db, project, document)?;
        let id = api.generate_id()?;
        drive_api::valid_id(&id)?;
        db.execute(
            "INSERT INTO drive_documents(document_id,project_id,file_id) VALUES(?1,?2,?3)",
            params![c.document, project, id],
        )
        .map_err(error)?;
    }
    let mut p = publication(db, project, document)?
        .ok_or_else(|| ApiError::local("Publication intent was not saved."))?;
    let mut remote = match api.get(&p.file_id) {
        Ok(r) => Some(r),
        Err(e) if e.missing && p.verified_hash.is_none() => None,
        Err(e) => return Err(e),
    };
    if let Some(r) = &remote {
        owns(db, project, document, parent, &p.file_id, r)?;
    }
    // Recover a completed upload whose response/receipt was lost, even when a
    // newer local version arrived while the old request was in flight.
    if let (Some(hash), Some(name), Some(size), Some(r)) =
        (&p.pending_hash, &p.pending_name, p.pending_size, &remote)
    {
        if r.name == *name
            && r.size.as_deref().and_then(|s| s.parse::<i64>().ok()) == Some(size)
            && api.download_hash(&p.file_id, size)? == *hash
        {
            let verified =
                verify_remote(db, project, parent, &p, api, Expected { hash, name, size })?;
            receipt(db, &p, &verified, hash)?;
            clear_pending(db, &registry.store, &p)?;
            p = publication(db, project, document)?.unwrap();
            remote = Some(verified);
        }
    }
    let c = current(db, project, document)?;
    if !source_matches(&c) {
        return Err(ApiError {
            retry: true,
            waiting: true,
            ..ApiError::local("Waiting for the latest local file to finish saving and tracking.")
        });
    }
    if p.verified_hash.as_deref() == Some(&c.hash) && p.verified_name.as_deref() == Some(&c.name) {
        // An explicit manual check still verifies the actual remote content.
        verify_remote(
            db,
            project,
            parent,
            &p,
            api,
            Expected {
                hash: &c.hash,
                name: &c.name,
                size: c.size,
            },
        )?;
        db.execute("UPDATE drive_documents SET state='synced',error=NULL,attempts=0,next_at=0 WHERE document_id=?1",[document]).map_err(error)?;
        return Ok(());
    }
    if let Some(r) = &remote {
        if let Some(previous) = &p.verified_hash {
            let size = r
                .size
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .ok_or_else(|| ApiError::local("The existing cloud size is unavailable."))?;
            if api.download_hash(&p.file_id, size)? != *previous {
                return Err(ApiError::local("The cloud file was edited outside ContextDock. Review the conflict before replacing it."));
            }
        } else if p.pending_hash.is_none() {
            return Err(ApiError::local("An unverified cloud file already occupies the reserved identity. Review it before replacing it."));
        }
    }
    let staged = storage::stage(Path::new(&c.root), &c.relative, &registry.store)?;
    if staged.hash != c.hash || staged.size != c.size {
        let _ = storage::discard_stage(&registry.store, &staged.temporary);
        return Err(ApiError {
            retry: true,
            waiting: true,
            ..ApiError::local("Waiting for file tracking to finish the latest save.")
        });
    }
    let old_stage = p.pending_stage.clone();
    db.execute("UPDATE drive_documents SET pending_hash=?2,pending_name=?3,pending_size=?4,pending_version=?5,pending_source=?6,pending_relative=?7,pending_stage=?8,pending_generation=?9,state='uploading',error=NULL WHERE document_id=?1",params![document,c.hash,c.name,c.size,c.version,c.source,c.relative,staged.temporary,generation(&c)]).map_err(error)?;
    if let Some(old) = old_stage {
        if old != staged.temporary {
            let _ = storage::discard_stage(&registry.store, &old);
        }
    }
    p = publication(db, project, document)?.unwrap();
    if p.pending_generation.as_deref() != Some(generation(&c).as_str())
        || p.pending_hash.as_deref() != Some(c.hash.as_str())
        || p.pending_stage.as_deref() != Some(staged.temporary.as_str())
    {
        return Err(ApiError::local(
            "The pending upload intent did not save. No upload was sent.",
        ));
    }
    require_enabled(db, project, account)?;
    if generation(&current(db, project, document)?) != generation(&c) {
        return Err(ApiError {retry:true,..ApiError::local("A newer local version arrived. Publication will retry with its current generation.")});
    }
    let mut properties = properties(db, project, Some(document))?;
    properties.insert("sha256".into(), c.hash.clone());
    properties.insert("generation".into(), generation(&c));
    let mime = if c.name.to_lowercase().ends_with(".docx") {
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    } else {
        "text/markdown"
    };
    let mut metadata = json!({"name":c.name,"mimeType":mime,"appProperties":properties});
    if remote.is_none() {
        metadata["parents"] = json!([parent]);
    }
    let checked = storage::checked(&registry.store, &staged.temporary, false)?;
    storage::verify(&checked.path, &c.hash)?;
    api.upload(
        &p.file_id,
        metadata,
        &checked.path,
        remote.is_none(),
        remote.as_ref().and_then(|r| r.etag.as_deref()),
    )?;
    db.execute(
        "UPDATE drive_documents SET state='verifying' WHERE document_id=?1",
        [document],
    )
    .map_err(error)?;
    let verified = verify_remote(
        db,
        project,
        parent,
        &p,
        api,
        Expected {
            hash: &c.hash,
            name: &c.name,
            size: c.size,
        },
    )?;
    receipt(db, &p, &verified, &c.hash)?;
    clear_pending(db, &registry.store, &p)?;
    // Verified replacement may be recorded after the local generation moved
    // ahead. Keep that receipt for recovery, but never call a stale file synced.
    let is_current = current(db, project, document)
        .is_ok_and(|next| generation(&next) == generation(&c) && source_matches(&next));
    db.execute(
        "UPDATE drive_documents SET state=?2 WHERE document_id=?1",
        params![document, if is_current { "synced" } else { "queued" }],
    )
    .map_err(error)?;
    Ok(())
}
pub fn source_matches(current: &Current) -> bool {
    storage::checked(Path::new(&current.root), &current.relative, false)
        .and_then(|p| storage::verify(&p.path, &current.hash))
        .is_ok()
}
