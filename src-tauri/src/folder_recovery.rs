//! Persistent project-root recovery. Reassociation changes metadata, never source bytes.
use crate::{
    models,
    registry::{error, Registry, Result},
    scanner, storage,
};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::{collections::HashMap, fs, path::Path};

pub const LOST: &str = "Folder lost or inaccessible. Please reassign it before working in this project. Your saved history is retained.";

pub fn root_guard(path: &str) -> Result<(fs::File, String)> {
    let root = Path::new(path);
    let canonical = fs::canonicalize(root).map_err(|_| LOST.to_string())?;
    if models::key(&canonical.to_string_lossy()) != models::key(path) {
        return Err(
            "The registered folder now resolves to another location. Please reassign it.".into(),
        );
    }
    let guard = storage::directory_guard(root)?;
    fs::read_dir(root).map_err(|_| LOST.to_string())?;
    let identity = storage::file_identity(&guard)?;
    Ok((guard, identity))
}

pub fn mark_lost(db: &Connection, id: &str, reason: &str) -> Result<()> {
    let tx = if db.is_autocommit() {
        Some(db.unchecked_transaction().map_err(error)?)
    } else {
        None
    };
    db.execute("UPDATE projects SET folder_revision=folder_revision+CASE WHEN folder_state='ready' THEN 1 ELSE 0 END,folder_state='needs_reassignment',status='unavailable',scan_error=?2,watch_status='needs_folder' WHERE id=?1 AND removed_at IS NULL",params![id,format!("{LOST} {reason}")]).map_err(error)?;
    db.execute(
        "UPDATE source_files SET status='unavailable' WHERE project_id=?1 AND status!='archived'",
        [id],
    )
    .map_err(error)?;
    if let Some(tx) = tx {
        tx.commit().map_err(error)?;
    }
    Ok(())
}

/// Also checks the live root so stale UI/worker state cannot authorize project work.
pub fn require_ready(db: &Connection, id: &str) -> Result<()> {
    let (path, state, saved): (String, String, Option<String>) = db.query_row(
        "SELECT path,folder_state,folder_identity FROM projects WHERE id=?1 AND removed_at IS NULL",[id],
        |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)))
        .map_err(|_| "Project registration was not found.".to_string())?;
    if state != "ready" {
        return Err(LOST.into());
    }
    match root_guard(&path) {
        Ok((_guard, identity)) if saved.as_ref().is_none_or(|s| *s == identity) => {
            if saved.is_none() {
                db.execute("UPDATE projects SET folder_identity=?2 WHERE id=?1 AND folder_identity IS NULL",params![id,identity]).map_err(error)?;
            }
            Ok(())
        }
        result => {
            let reason = result
                .err()
                .unwrap_or_else(|| "A different folder now occupies the saved path.".into());
            mark_lost(db, id, &reason)?;
            Err(LOST.into())
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderPlan {
    pub path: String,
    pub previous_path: String,
    pub found: usize,
    pub unchanged: usize,
    pub changed: usize,
    pub missing: Vec<String>,
    pub additional: usize,
    pub token: String,
}

impl Registry {
    /// Lightweight health checks include projects whose tracking is off.
    pub fn check_folders(&self) -> Result<Vec<String>> {
        let mut q = self
            .db
            .prepare("SELECT id,folder_state FROM projects WHERE removed_at IS NULL")
            .map_err(error)?;
        let projects = q
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(error)?;
        drop(q);
        let mut changed = vec![];
        for (id, state) in projects {
            if state == "ready" && require_ready(&self.db, &id).is_err() {
                changed.push(id);
            }
        }
        Ok(changed)
    }

    pub fn folder_plan(&self, id: &str, selected: &str) -> Result<FolderPlan> {
        let previous_path = self.path(id)?;
        if scanner::is_link(Path::new(selected)) {
            return Err("Choose the actual project folder, not a link or junction.".into());
        }
        let path = scanner::canonical(selected)?;
        let root = Path::new(&path);
        if root.starts_with(&self.store) || self.store.starts_with(root) {
            return Err(
                "Choose a dedicated project folder outside ContextDock's application storage."
                    .into(),
            );
        }
        let (_guard, identity) = root_guard(&path)?;
        let mut q = self
            .db
            .prepare("SELECT path FROM projects WHERE id!=?1")
            .map_err(error)?;
        let others = q
            .query_map([id], |r| r.get::<_, String>(0))
            .map_err(error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(error)?;
        for other in others {
            let other = models::key(&other);
            let target = models::key(&path);
            if Path::new(&target).starts_with(&other) || Path::new(&other).starts_with(&target) {
                return Err("This folder overlaps another current or retained project registration. Choose this project's own folder; histories cannot be merged.".into());
            }
        }
        let mut scan = scanner::scan(root)?;
        if !scan.errors.is_empty() {
            return Err(format!(
                "The replacement folder could not be fully checked. Fix access and retry. {}",
                scan.errors[0]
            ));
        }
        scan.files.sort_by(|a, b| a.key.cmp(&b.key));
        let files: HashMap<_, _> = scan.files.iter().map(|f| (f.key.clone(), f)).collect();
        let mut q = self.db.prepare("SELECT s.id,s.relative_path,s.last_hash,s.disposition,d.current_version_id FROM source_files s JOIN documents d ON d.id=s.document_id WHERE s.project_id=?1 AND s.disposition NOT IN ('archived','superseded') ORDER BY s.id").map_err(error)?;
        let known = q
            .query_map([id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, Option<String>>(4)?,
                ))
            })
            .map_err(error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(error)?;
        let mut found = 0;
        let mut unchanged = 0;
        let mut changed = 0;
        let mut missing = vec![];
        let mut observed = vec![];
        for (_, relative, hash, _, _) in &known {
            if let Some(f) = files.get(&models::key(relative)) {
                let checked = storage::checked(root, &f.relative, false)?;
                let mut file = storage::locked_read(&checked.path, false)?;
                let actual = storage::hash(&mut file)?;
                if let Some(expected) = hash {
                    if *expected == actual {
                        unchanged += 1;
                    } else {
                        changed += 1;
                    }
                }
                found += 1;
                observed.push(serde_json::json!([f.relative, f.size, f.modified, actual]));
            } else {
                missing.push(relative.clone());
            }
        }
        missing.sort();
        let (revision, state): (i64, String) = self
            .db
            .query_row(
                "SELECT folder_revision,folder_state FROM projects WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(error)?;
        // Bind the review to the registered root, exact selected directory, source identities,
        // bytes and additional-file inventory. Apply rechecks it under a transaction.
        let inventory: Vec<_> = scan
            .files
            .iter()
            .map(|f| serde_json::json!([f.relative, f.size, f.modified]))
            .collect();
        let token = crate::cloud::scope(
            &serde_json::json!([
                id,
                previous_path,
                path,
                identity,
                revision,
                state,
                known,
                observed,
                inventory
            ])
            .to_string(),
        );
        Ok(FolderPlan {
            path,
            previous_path,
            found,
            unchanged,
            changed,
            missing,
            additional: files.len().saturating_sub(found),
            token,
        })
    }

    pub fn reassign_folder(&mut self, id: &str, path: &str, token: &str) -> Result<()> {
        let tx = self.db.unchecked_transaction().map_err(error)?;
        let plan = self.folder_plan(id, path)?;
        if token != plan.token {
            return Err("The folder or project changed since review. Choose the folder again and review the updated checks.".into());
        }
        let (_guard, identity) = root_guard(&plan.path)?;
        tx.execute("UPDATE projects SET path=?2,path_key=?3,folder_identity=?4,folder_state='ready',folder_revision=folder_revision+1,status='available',scan_error=NULL,watch_status=CASE WHEN tracking_enabled=1 THEN 'starting' ELSE 'off' END WHERE id=?1 AND removed_at IS NULL",params![id,plan.path,models::key(&plan.path),identity]).map_err(error)?;
        let verified:bool=tx.query_row("SELECT path=?2 AND folder_identity=?3 AND folder_state='ready' FROM projects WHERE id=?1",params![id,plan.path,identity],|r|r.get(0)).map_err(error)?;
        if !verified {
            return Err(
                "Folder reassignment could not be verified. The previous registration is retained."
                    .into(),
            );
        }
        tx.execute(
            "UPDATE source_files SET hashed_stamp=NULL,hashed_size=NULL WHERE project_id=?1",
            [id],
        )
        .map_err(error)?;
        tx.execute(
            "UPDATE cloud_projects SET revision=revision+1 WHERE project_id=?1",
            [id],
        )
        .map_err(error)?;
        self.activity(id,"folder_reassigned",&format!("Project folder reassigned from {} to {}. {} known files found; {} missing. History and project identity retained.",plan.previous_path,plan.path,plan.found,plan.missing.len()))?;
        tx.commit().map_err(error)?;
        self.refresh(id)
    }
}
