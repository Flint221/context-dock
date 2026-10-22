//! Durable, replayable capture/archival state machine. Committed snapshots are never overwritten.
use crate::{
    matching, models,
    registry::{error, Registry, Result},
    storage,
    tracking_queries::Source,
};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::{Component, Path},
};
#[derive(Serialize, Deserialize, Clone)]
pub struct CapturePlan {
    pub project: String,
    pub document: String,
    pub source: String,
    pub version: String,
    pub number: i64,
    pub name: String,
    pub relative: String,
    pub modified: Option<String>,
    pub size: i64,
    pub hash: String,
    pub temporary: String,
    pub snapshot: String,
    pub reason: String,
    pub captured_at: String,
    pub previous_version: Option<String>,
    pub previous_source: Option<String>,
    pub may_archive: bool,
    #[serde(default)]
    pub version_key: Option<String>,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct ArchivePlan {
    pub project: String,
    pub document: String,
    pub old_version: String,
    pub new_version: String,
    pub old_source: String,
    pub new_source: String,
    pub destination: String,
    pub temporary: String,
    pub mode: String,
    pub old_identity: Option<String>,
}
fn encode<T: Serialize>(p: &T) -> Result<String> {
    serde_json::to_string(p).map_err(|e| e.to_string())
}
fn event(db: &Connection, project: &str, kind: &str, message: &str) -> Result<()> {
    db.execute("INSERT INTO tracking_activity(id,project_id,kind,message,created_at) VALUES(?1,?2,?3,?4,?5)",params![uuid::Uuid::new_v4().to_string(),project,kind,message,models::now()]).map_err(error)?;
    Ok(())
}
/// Final archive paths keep the source filename, including its author version.
/// Journal IDs remain internal; readable subfolders disambiguate collisions.
pub(crate) fn archive_destination(
    db: &Connection,
    project: &str,
    family: &str,
    relative: &str,
    capture: Option<i64>,
    reserved: &[String],
) -> Result<String> {
    let source = Path::new(relative);
    if source
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("Unsafe original archive path.".into());
    }
    let name = source
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Archive source has no filename.")?;
    let mut folder = matching::safe_name(family);
    let device = folder.split('.').next().unwrap_or("").to_uppercase();
    if matches!(
        device.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|prefix| {
        device.strip_prefix(prefix).is_some_and(|n| {
            matches!(
                n,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
    }) {
        folder.push_str(" documents");
    }
    let mut directory = Path::new("Old Context Files")
        .join(source.parent().unwrap_or(Path::new("")))
        .join(folder);
    if let Some(number) = capture {
        directory.push(format!("Capture {number}"));
    }
    let root: String = db
        .query_row("SELECT path FROM projects WHERE id=?1", [project], |r| {
            r.get(0)
        })
        .map_err(error)?;
    let mut q = db.prepare("SELECT CASE kind WHEN 'restore' THEN json_extract(payload,'$.archive.destination') ELSE json_extract(payload,'$.destination') END FROM operations WHERE project_id=?1 AND ((kind='archive' AND state NOT IN ('cancelled','restored')) OR (kind='restore' AND state NOT IN ('done','cancelled','restored')))").map_err(error)?;
    let mut occupied = q
        .query_map([project], |r| r.get::<_, Option<String>>(0))
        .map_err(error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(error)?
        .into_iter()
        .flatten()
        .map(|p| crate::path_selection::normalized(&p))
        .collect::<HashSet<_>>();
    occupied.extend(
        reserved
            .iter()
            .map(|p| crate::path_selection::normalized(p)),
    );
    for index in 1..=10_000 {
        let candidate = if index == 1 {
            directory.join(name)
        } else {
            directory.join(format!("Archive {index}")).join(name)
        };
        let relative = candidate.to_string_lossy().replace('\\', "/");
        let path = Path::new(&root).join(&candidate);
        let exists = match std::fs::symlink_metadata(&path) {
            Ok(_) => true,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
            Err(e) => return Err(format!("Cannot check archive destination {relative}: {e}")),
        };
        if !exists && !occupied.contains(&crate::path_selection::normalized(&relative)) {
            return Ok(relative);
        }
    }
    Err("Too many archive name conflicts; review Old Context Files before retrying.".into())
}
fn enqueue_archive(
    db: &Connection,
    source: &Source,
    previous: Option<&str>,
    old_source: Option<&str>,
    next: &str,
    allowed: bool,
) -> Result<()> {
    let project = &source.project_id;
    let document = &source.document_id;
    let new_source = &source.id;
    if !allowed {
        return Ok(());
    }
    let (Some(previous), Some(old_source)) = (previous, old_source) else {
        return Ok(());
    };
    let old_path: String = db
        .query_row(
            "SELECT relative_path FROM source_files WHERE id=?1",
            [old_source],
            |r| r.get(0),
        )
        .map_err(error)?;
    if !crate::path_selection::allowed(db, project, &old_path)? {
        return Ok(());
    }
    if previous == next {
        return Ok(());
    }
    let (number, relative, already): (i64, String, Option<String>) = db
        .query_row(
            "SELECT number,source_relative,archive_relative FROM document_versions WHERE id=?1",
            [previous],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(error)?;
    if already.is_some() {
        return Ok(());
    }
    let family: String = db
        .query_row(
            "SELECT family_name FROM documents WHERE id=?1",
            [document],
            |r| r.get(0),
        )
        .map_err(error)?;
    let status: String = db
        .query_row(
            "SELECT status FROM source_files WHERE id=?1",
            [old_source],
            |r| r.get(0),
        )
        .map_err(error)?;
    let id = uuid::Uuid::new_v4().to_string();
    let copy = old_source == new_source || status == "missing";
    let destination = archive_destination(
        db,
        project,
        &family,
        if copy { &relative } else { &old_path },
        copy.then_some(number),
        &[],
    )?;
    let directory = Path::new(&destination)
        .parent()
        .ok_or("Archive has no directory.")?
        .to_string_lossy()
        .into_owned();
    let plan = ArchivePlan {
        project: project.into(),
        document: document.into(),
        old_version: previous.into(),
        new_version: next.into(),
        old_source: old_source.into(),
        new_source: new_source.into(),
        destination,
        old_identity: None,
        temporary: format!("{directory}/.contextdock-{id}.tmp"),
        mode: if copy { "copy" } else { "move" }.into(),
    };
    let key = format!("archive:{previous}:{next}");
    let terminal:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM operations WHERE dedupe_key=?1 AND state IN ('cancelled','restored'))",[&key],|r|r.get(0)).map_err(error)?;
    let key = if terminal { format!("{key}:{id}") } else { key };
    db.execute("INSERT OR IGNORE INTO operations(id,project_id,document_id,dedupe_key,kind,payload,created_at,updated_at) VALUES(?1,?2,?3,?4,'archive',?5,?6,?6)",params![id,project,document,key,encode(&plan)?,models::now()]).map_err(error)?;
    Ok(())
}
fn activate(
    db: &Connection,
    source: &Source,
    version: &str,
    hash: &str,
    size: i64,
    stamp: Option<&str>,
) -> Result<()> {
    db.execute("UPDATE source_files SET disposition='superseded' WHERE document_id=?1 AND id!=?2 AND disposition='tracked'",params![source.document_id,source.id]).map_err(error)?;
    db.execute("UPDATE source_files SET disposition='tracked',last_hash=?1,hashed_stamp=?2,hashed_size=?3 WHERE id=?4",params![hash,stamp,size,source.id]).map_err(error)?;
    db.execute("UPDATE documents SET tracking_state='tracked',active_source_id=?1,current_version_id=?2 WHERE id=?3",params![source.id,version,source.document_id]).map_err(error)?;
    db.execute(
        "DELETE FROM pending_matches WHERE source_id=?1",
        [&source.id],
    )
    .map_err(error)?;
    Ok(())
}
impl Registry {
    pub fn capture(
        &mut self,
        source: &Source,
        staged: storage::Staged,
        reason: &str,
        may_archive: bool,
    ) -> Result<()> {
        if !crate::path_selection::allowed(&self.db, &source.project_id, &source.relative)? {
            storage::discard_stage(&self.store, &staged.temporary)?;
            return Err("File capture is paused by the saved folder selection.".into());
        }
        let (previous, old_source): (Option<String>, Option<String>) = self
            .db
            .query_row(
                "SELECT current_version_id,active_source_id FROM documents WHERE id=?1",
                [&source.document_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(error)?;
        if reason != "manual_assignment"
            && crate::versioning::would_lower(&self.db, &source.document_id, &source.name)?
        {
            self.queue_review(source, &staged, vec![source.document_id.clone()], "The filename version decreased. Confirm the document family or restore the higher filename version. Saved history is retained; nothing will be archived or synced until reviewed.")?;
            return storage::discard_stage(&self.store, &staged.temporary);
        }
        let group = crate::versioning::effective(&self.db, &source.document_id, &source.name)?;
        let existing = crate::versioning::content_match(
            &self.db,
            &source.document_id,
            &staged.hash,
            &source.name,
        )?;
        if let Some(version) = existing {
            let tx = self.db.transaction().map_err(error)?;
            activate(
                &tx,
                source,
                &version,
                &staged.hash,
                staged.size,
                staged.modified.as_deref(),
            )?;
            enqueue_archive(
                &tx,
                source,
                previous.as_deref(),
                old_source.as_deref(),
                &version,
                may_archive,
            )?;
            if previous.as_deref() != Some(&version) || old_source.as_deref() != Some(&source.id) {
                event(
                    &tx,
                    &source.project_id,
                    "known_content",
                    &format!(
                        "{}: identical preserved content reused; no new version created.",
                        source.name
                    ),
                )?;
            }
            tx.commit().map_err(error)?;
            storage::discard_stage(&self.store, &staged.temporary)?;
            return Ok(());
        }
        let pending:bool=self.db.query_row("SELECT EXISTS(SELECT 1 FROM operations WHERE document_id=?1 AND kind='capture' AND state!='done')",[&source.document_id],|r|r.get(0)).map_err(error)?;
        if pending {
            storage::discard_stage(&self.store, &staged.temporary)?;
            return Err("A previous capture for this family needs recovery before another can be committed.".into());
        }
        let number: i64 = self
            .db
            .query_row(
                "SELECT COALESCE(MAX(number),0)+1 FROM document_versions WHERE document_id=?1",
                [&source.document_id],
                |r| r.get(0),
            )
            .map_err(error)?;
        let version = uuid::Uuid::new_v4().to_string();
        let id = uuid::Uuid::new_v4().to_string();
        let extension = Path::new(&source.name)
            .extension()
            .and_then(|n| n.to_str())
            .unwrap_or("md")
            .to_lowercase();
        let plan = CapturePlan {
            project: source.project_id.clone(),
            document: source.document_id.clone(),
            source: source.id.clone(),
            version: version.clone(),
            number,
            name: source.name.clone(),
            relative: source.relative.clone(),
            modified: staged.modified,
            size: staged.size,
            hash: staged.hash,
            temporary: staged.temporary,
            snapshot: format!(
                "snapshots/{}/{}/{version}.{extension}",
                source.project_id, source.document_id
            ),
            reason: reason.into(),
            captured_at: models::now(),
            previous_version: previous,
            previous_source: old_source,
            may_archive,
            version_key: Some(group.clone()),
        };
        self.db.execute("INSERT INTO operations(id,project_id,document_id,dedupe_key,kind,payload,created_at,updated_at) VALUES(?1,?2,?3,?4,'capture',?5,?6,?6)",params![id,source.project_id,source.document_id,format!("capture:{}:{}:{}",source.document_id,group,plan.hash),encode(&plan)?,models::now()]).map_err(error)?;
        match self.finish_capture(&id, &plan, false) {
            Ok(()) => Ok(()),
            Err(e) => {
                self.block_operation(&id, &source.project_id, &e)?;
                Err(e)
            }
        }
    }
    fn finish_capture(&mut self, id: &str, plan: &CapturePlan, finalized: bool) -> Result<()> {
        storage::snapshot_finalize(
            &self.store,
            &plan.temporary,
            &plan.snapshot,
            &plan.hash,
            finalized,
        )?;
        self.db
            .execute(
                "UPDATE operations SET state='finalized',error=NULL,updated_at=?1 WHERE id=?2",
                params![models::now(), id],
            )
            .map_err(error)?;
        storage::snapshot_seal(&self.store, &plan.temporary, &plan.snapshot)?;
        let source = self.source(&plan.source)?;
        let tx = self.db.transaction().map_err(error)?;
        let version_key = match &plan.version_key {
            Some(key) => key.clone(),
            None => crate::versioning::effective(&tx, &plan.document, &plan.name)?,
        };
        tx.execute("INSERT INTO document_versions(id,document_id,number,source_name,source_relative,captured_at,modified_at,size,sha256,snapshot_relative,reason,version_key) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",params![plan.version,plan.document,plan.number,plan.name,plan.relative,plan.captured_at,plan.modified,plan.size,plan.hash,plan.snapshot,plan.reason,version_key]).map_err(error)?;
        activate(
            &tx,
            &source,
            &plan.version,
            &plan.hash,
            plan.size,
            plan.modified.as_deref(),
        )?;
        // Archive intent is committed together with the new version, never in a later best-effort step.
        enqueue_archive(
            &tx,
            &source,
            plan.previous_version.as_deref(),
            plan.previous_source.as_deref(),
            &plan.version,
            plan.may_archive,
        )?;
        tx.execute(
            "UPDATE operations SET state='done',error=NULL,updated_at=?1 WHERE id=?2",
            params![models::now(), id],
        )
        .map_err(error)?;
        let revision = plan
            .previous_version
            .as_deref()
            .map(|previous| crate::versioning::same(&tx, previous, &plan.name))
            .transpose()?
            .unwrap_or(false);
        event(
            &tx,
            &plan.project,
            if revision {
                "revision_saved"
            } else {
                "captured"
            },
            &format!(
                "{}: {} saved and SHA-256 verified (revision #{}, {})",
                plan.name,
                crate::matching::version_label(&plan.name).unwrap_or_else(|| "Working copy".into()),
                plan.number,
                plan.reason
            ),
        )?;
        tx.commit().map_err(error)?;
        Ok(())
    }
    fn finish_archive(&mut self, id: &str, plan: &ArchivePlan, finalized: bool) -> Result<()> {
        let mut plan = plan.clone();
        let (enabled, archive, _) = self.tracking_flags(&plan.project)?;
        if !enabled || !archive {
            return Ok(());
        }
        let root_string = self.path(&plan.project)?;
        let root = Path::new(&root_string);
        let (old_snapshot, old_hash): (String, String) = self
            .db
            .query_row(
                "SELECT snapshot_relative,sha256 FROM document_versions WHERE id=?1",
                [&plan.old_version],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(error)?;
        let (new_snapshot, new_hash): (String, String) = self
            .db
            .query_row(
                "SELECT snapshot_relative,sha256 FROM document_versions WHERE id=?1",
                [&plan.new_version],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(error)?;
        let old = storage::checked(&self.store, &old_snapshot, false)?;
        let new = storage::checked(&self.store, &new_snapshot, false)?;
        storage::verify(&old.path, &old_hash)?;
        storage::verify(&new.path, &new_hash)?;
        let dest = storage::checked(root, &plan.destination, true)?;
        if plan.mode == "copy" {
            let prior_temp = storage::checked(root, &plan.temporary, true)?;
            if !dest.path.exists() && prior_temp.path.exists() {
                // A crash can leave an incomplete temporary. Retain it, journal a fresh private
                // name and copy again; never truncate/delete a file whose ownership is uncertain.
                let parent = Path::new(&plan.destination)
                    .parent()
                    .ok_or("Archive has no parent")?;
                plan.temporary = parent
                    .join(format!(".contextdock-{}.tmp", uuid::Uuid::new_v4()))
                    .to_string_lossy()
                    .into();
                self.db
                    .execute(
                        "UPDATE operations SET payload=?1,updated_at=?2 WHERE id=?3",
                        params![encode(&plan)?, models::now(), id],
                    )
                    .map_err(error)?;
                self.activity(
                    &plan.project,
                    "recovered",
                    "Incomplete archive staging retained; retrying into a fresh private temporary.",
                )?;
            }
            storage::archive_copy(
                &old.path,
                root,
                &plan.temporary,
                &plan.destination,
                &old_hash,
                finalized,
            )?;
        } else {
            let previous = self.source(&plan.old_source)?;
            let replacement = self.source(&plan.new_source)?;
            let source = storage::checked(root, &previous.relative, false)?;
            if dest.path.exists() {
                // Crash after handle rename but before SQLite commit: complete metadata only; never delete either path.
                if source.path.exists() {
                    return Err("Archive destination already exists while the previous source remains. Nothing was overwritten or moved.".into());
                }
                storage::verify(&dest.path, &old_hash)?;
                let identity = storage::file_identity(&storage::locked_read(&dest.path, false)?)?;
                if plan.old_identity.as_deref() != Some(&identity) {
                    return Err("Archive ownership could not be proven after interruption. Existing destination was left untouched.".into());
                }
            } else {
                let current = storage::checked(root, &replacement.relative, false)?;
                let mut current_file = storage::locked_read(&current.path, false)?;
                if storage::hash(&mut current_file)? != new_hash {
                    return Err("Replacement changed after capture. Previous source was left intact; reconcile and review the pending archive.".into());
                }
                let mut old_file = storage::locked_read(&source.path, true)?;
                if storage::hash(&mut old_file)? != old_hash {
                    return Err("Previous source changed unexpectedly. Archival conflict: nothing was moved.".into());
                }
                let identity = storage::file_identity(&old_file)?;
                if plan
                    .old_identity
                    .as_ref()
                    .is_some_and(|expected| expected != &identity)
                {
                    return Err("Previous source identity changed. Nothing was moved.".into());
                }
                if plan.old_identity.is_none() {
                    plan.old_identity = Some(identity);
                    self.db
                        .execute(
                            "UPDATE operations SET payload=?1,updated_at=?2 WHERE id=?3",
                            params![encode(&plan)?, models::now(), id],
                        )
                        .map_err(error)?;
                }
                storage::rename_verified(&old_file, &dest.path)?;
            }
        }
        self.db
            .execute(
                "UPDATE operations SET state='finalized',error=NULL,updated_at=?1 WHERE id=?2",
                params![models::now(), id],
            )
            .map_err(error)?;
        let tx = self.db.transaction().map_err(error)?;
        tx.execute(
            "UPDATE document_versions SET archive_relative=?1 WHERE id=?2",
            params![plan.destination, plan.old_version],
        )
        .map_err(error)?;
        if plan.mode == "move" {
            tx.execute(
                "UPDATE source_files SET status='archived',disposition='archived' WHERE id=?1",
                [&plan.old_source],
            )
            .map_err(error)?;
        }
        tx.execute(
            "UPDATE operations SET state='done',error=NULL,updated_at=?1 WHERE id=?2",
            params![models::now(), id],
        )
        .map_err(error)?;
        event(
            &tx,
            &plan.project,
            "archived",
            &format!("Previous version preserved in {}", plan.destination),
        )?;
        tx.commit().map_err(error)?;
        if plan.mode == "copy" {
            storage::cleanup_archive_temp(root, &plan.temporary, &plan.destination);
        }
        Ok(())
    }
    pub fn block_operation(&self, id: &str, project: &str, message: &str) -> Result<()> {
        // Keep finalized ownership evidence for retry, even when later metadata/sealing work failed.
        self.db.execute("UPDATE operations SET state=CASE WHEN state='finalized' THEN state ELSE 'blocked' END,error=?1,updated_at=?2 WHERE id=?3",params![message,models::now(),id]).map_err(error)?;
        self.activity(project, "operation_failed", message)
    }
    pub fn recover_operations(
        &mut self,
        project: &str,
        retry_blocked: bool,
    ) -> Result<Vec<String>> {
        let tracking = self.tracking_flags(project)?.0;
        if !tracking {
            let restores: bool = self.db.query_row("SELECT EXISTS(SELECT 1 FROM operations WHERE project_id=?1 AND kind='restore' AND (state IN ('pending','finalized') OR (?2 AND state='blocked')))", params![project, retry_blocked], |r| r.get(0)).map_err(error)?;
            if !restores {
                return Ok(vec![]);
            }
        }
        crate::folder_recovery::require_ready(&self.db, project)?;
        let mut q=self.db.prepare("SELECT id,kind,payload,state FROM operations WHERE project_id=?1 AND (state IN ('pending','finalized') OR (?2 AND state='blocked')) ORDER BY CASE kind WHEN 'capture' THEN 0 ELSE 1 END,created_at").map_err(error)?;
        let ops = q
            .query_map(params![project, retry_blocked], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })
            .map_err(error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(error)?;
        drop(q);
        let mut errors = vec![];
        for (id, kind, payload, state) in ops {
            if !tracking && kind != "restore" {
                continue;
            }
            if kind == "archive" && !self.tracking_flags(project)?.1 {
                continue;
            }
            let value: serde_json::Value =
                serde_json::from_str(&payload).map_err(|_| "Invalid pending operation.")?;
            if kind == "capture"
                && !crate::path_selection::allowed(
                    &self.db,
                    project,
                    value["relative"]
                        .as_str()
                        .ok_or("Capture path is missing.")?,
                )?
            {
                continue;
            }
            if kind == "archive" {
                let old = self.source(
                    value["old_source"]
                        .as_str()
                        .ok_or("Archive source is missing.")?,
                )?;
                let new = self.source(
                    value["new_source"]
                        .as_str()
                        .ok_or("Archive replacement is missing.")?,
                )?;
                if !crate::path_selection::allowed(&self.db, project, &old.relative)?
                    || !crate::path_selection::allowed(&self.db, project, &new.relative)?
                {
                    continue;
                }
            }
            let result = match kind.as_str() {
                "restore" => serde_json::from_str::<crate::archival::RestorePlan>(&payload)
                    .map_err(|e| e.to_string())
                    .and_then(|p| self.finish_restore(&id, &p)),
                "initial_import" => self.finish_initial_import(&id, &payload),
                "capture" => serde_json::from_str::<CapturePlan>(&payload)
                    .map_err(|e| e.to_string())
                    .and_then(|p| self.finish_capture(&id, &p, state == "finalized")),
                "archive" => serde_json::from_str::<ArchivePlan>(&payload)
                    .map_err(|e| e.to_string())
                    .and_then(|p| self.finish_archive(&id, &p, state == "finalized")),
                _ => Err("Unknown journal operation; no filesystem work attempted.".into()),
            };
            if let Err(e) = result {
                self.block_operation(&id, project, &e)?;
                errors.push(e);
            } else if retry_blocked || state != "pending" {
                self.activity(
                    project,
                    "recovered",
                    &format!("Recovered interrupted {kind} operation {id}."),
                )?;
            }
        }
        Ok(errors)
    }
}
