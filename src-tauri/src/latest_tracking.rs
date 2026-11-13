use crate::{
    models, operations,
    registry::{error, Registry, Result},
    storage,
    tracking_queries::Source,
};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
pub fn enabled(db: &Connection) -> bool {
    crate::review::preference(db, "workflow.latest_only", true)
}
pub(crate) fn newer_same_path_family(old: &Source, next: &Source) -> bool {
    old.kind == next.kind
        && crate::matching::parent_key(&old.relative) == crate::matching::parent_key(&next.relative)
        && crate::matching::stem(&old.name).normalized
            == crate::matching::stem(&next.name).normalized
        && crate::versioning::key(&old.name) != crate::versioning::key(&next.name)
        && matches!((crate::matching::version_order(&old.name),crate::matching::version_order(&next.name)),(Some(a),Some(b)) if b>a)
}
impl Registry {
    /// Recognize a move without granting permission to monitor its destination.
    pub(crate) fn reconcile_excluded_relocations(
        &mut self,
        project: &str,
        root: &Path,
        rules: &[crate::path_selection::Rule],
    ) -> Result<()> {
        if !enabled(&self.db) {
            return Ok(());
        }
        let complete: bool = self.db.query_row("SELECT status='available' AND scan_error IS NULL AND folder_state='ready' FROM projects WHERE id=?1", [project], |r| r.get(0)).map_err(error)?;
        if !complete {
            return Ok(());
        }
        let missing = crate::cloud::rows(&self.db,"SELECT s.id FROM source_files s JOIN documents d ON d.id=s.document_id WHERE s.project_id=?1 AND s.status='missing' AND s.disposition IN ('tracked','superseded','ignored') AND s.last_hash IS NOT NULL AND d.tracking_state IN ('tracked','ignored') AND d.merged_into IS NULL AND NOT EXISTS(SELECT 1 FROM operations o WHERE o.document_id=d.id AND o.state NOT IN ('done','cancelled','restored')) AND NOT EXISTS(SELECT 1 FROM drive_documents p WHERE p.document_id=d.id AND p.pending_hash IS NOT NULL) AND NOT EXISTS(SELECT 1 FROM drive_recycle r WHERE r.source_id=s.id) AND NOT EXISTS(SELECT 1 FROM pending_matches m WHERE m.source_id=s.id OR EXISTS(SELECT 1 FROM json_each(m.candidate_ids) j WHERE j.value=d.id))", &[&project])?
            .iter().map(|r| self.source(crate::cloud::s(r,"id"))).collect::<Result<Vec<_>>>()?;
        if missing.is_empty() {
            return Ok(());
        }
        let observed = self
            .source_ids(project)?
            .iter()
            .map(|id| self.source(id))
            .collect::<Result<Vec<_>>>()?;
        for source in &observed {
            if source.disposition != "discovered" || source.last_hash.is_some() {
                continue;
            }
            let path = crate::path_selection::normalized(&source.relative);
            let is_new: bool = self
                .db
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM new_paths WHERE project_id=?1 AND path=?2)",
                    params![project, path],
                    |r| r.get(0),
                )
                .map_err(error)?;
            let explicit_rules = rules
                .iter()
                .filter(|r| !(is_new && r.kind == "file" && r.path == path))
                .cloned()
                .collect::<Vec<_>>();
            if crate::path_selection::permits(&explicit_rules, &path) {
                continue;
            }
            // A live copy, competing destination or renamed/changed document
            // cannot establish this conservative excluded-folder move.
            let same_name = |s: &&Source| {
                s.kind == source.kind
                    && s.name.eq_ignore_ascii_case(&source.name)
                    && s.size == source.size
            };
            if observed.iter().filter(same_name).count() != 1 {
                continue;
            }
            let mut candidates = missing
                .iter()
                .filter(same_name)
                .filter(|old| {
                    std::fs::symlink_metadata(root.join(&old.relative))
                        .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
                })
                .collect::<Vec<_>>();
            if candidates.is_empty() {
                continue;
            }
            let Ok(staged) = storage::stage_metadata(root, &source.relative) else {
                continue;
            };
            candidates.retain(|old| old.last_hash.as_deref() == Some(&staged.hash));
            if candidates.len() != 1 {
                continue;
            }
            let old = candidates[0];
            let pristine: bool = self.db.query_row("SELECT NOT EXISTS(SELECT 1 FROM document_versions WHERE document_id=?1) AND NOT EXISTS(SELECT 1 FROM operations WHERE document_id=?1) AND NOT EXISTS(SELECT 1 FROM pending_matches WHERE source_id=?2) AND NOT EXISTS(SELECT 1 FROM drive_documents WHERE document_id=?1) AND (SELECT COUNT(*) FROM source_files WHERE document_id=?1)=1", params![source.document_id,source.id], |r| r.get(0)).map_err(error)?;
            if !pristine
                || !std::fs::symlink_metadata(root.join(&old.relative))
                    .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
            {
                continue;
            }
            let tx = self.db.transaction().map_err(error)?;
            tx.execute("UPDATE documents SET active_source_id=NULL,tracking_state='merged',merged_into=?1 WHERE id=?2", params![old.document_id,source.document_id]).map_err(error)?;
            tx.execute("DELETE FROM source_files WHERE id=?1", [&source.id])
                .map_err(error)?;
            tx.execute("UPDATE source_files SET name=?2,relative_path=?3,path_key=?4,size=?5,modified_at=?6,last_observed=?7,status='available',disposition='ignored',hashed_stamp=?6,hashed_size=?5 WHERE id=?1", params![old.id,source.name,source.relative,models::key(&source.relative),staged.size,staged.modified,models::now()]).map_err(error)?;
            tx.execute(
                "UPDATE documents SET tracking_state='ignored' WHERE id=?1 AND active_source_id=?2",
                params![old.document_id, old.id],
            )
            .map_err(error)?;
            tx.execute(
                "DELETE FROM new_paths WHERE project_id=?1 AND path=?2",
                params![project, path],
            )
            .map_err(error)?;
            tx.execute("INSERT INTO path_selection VALUES(?1,1) ON CONFLICT(project_id) DO UPDATE SET revision=revision+1", [project]).map_err(error)?;
            tx.execute("INSERT OR IGNORE INTO document_aliases(project_id,parent_key,stem,kind,document_id) VALUES(?1,?2,?3,?4,?5)", params![project,crate::matching::parent_key(&source.relative),crate::matching::stem(&source.name).normalized,source.kind,old.document_id]).map_err(error)?;
            tx.commit().map_err(error)?;
            self.activity(project,"source_relocated",&format!("{} -> {}. Verified identical-content move; source and Drive identities retained. Destination remains excluded from monitoring and publication.",old.relative,source.relative))?;
        }
        Ok(())
    }
    /// Repair already-captured rename ghosts without deleting version metadata,
    /// source IDs or publication receipts. Never infer this from names alone.
    pub(crate) fn retire_version_rename_paths(
        &mut self,
        project: &str,
        root: &Path,
        rules: &[crate::path_selection::Rule],
    ) -> Result<()> {
        if !enabled(&self.db) {
            return Ok(());
        }
        let blocked = crate::path_selection::blocked_documents(&self.db, project)?;
        let rows = crate::cloud::rows(&self.db,"SELECT s.id FROM source_files s JOIN documents d ON d.id=s.document_id WHERE s.project_id=?1 AND s.status IN ('missing','available') AND s.disposition IN ('tracked','superseded') AND s.last_hash IS NOT NULL AND d.tracking_state='tracked' AND d.merged_into IS NULL AND NOT EXISTS(SELECT 1 FROM operations o WHERE o.document_id=d.id AND o.state NOT IN ('done','cancelled','restored')) AND NOT EXISTS(SELECT 1 FROM pending_matches m JOIN source_files f ON f.id=m.source_id WHERE f.document_id=d.id) AND NOT EXISTS(SELECT 1 FROM drive_documents p WHERE p.document_id=d.id AND p.pending_hash IS NOT NULL) AND NOT EXISTS(SELECT 1 FROM drive_recycle r WHERE r.source_id=s.id)",&[&project])?;
        let sources = rows
            .iter()
            .map(|r| self.source(crate::cloud::s(r, "id")))
            .collect::<Result<Vec<_>>>()?;
        let mut pairs = vec![];
        for old in sources.iter().filter(|s| {
            s.status == "missing"
                && !blocked.contains(&s.document_id)
                && crate::path_selection::permits(rules, &s.relative)
        }) {
            if !std::fs::symlink_metadata(root.join(&old.relative))
                .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
            {
                continue;
            }
            let candidates = sources
                .iter()
                .filter(|next| {
                    next.status == "available"
                        && next.document_id == old.document_id
                        && next.last_hash == old.last_hash
                        && newer_same_path_family(old, next)
                        && crate::path_selection::permits(rules, &next.relative)
                })
                .collect::<Vec<_>>();
            if candidates.len() == 1 {
                pairs.push((old, candidates[0]));
            }
        }
        for (old, next) in &pairs {
            if pairs
                .iter()
                .filter(|(_, candidate)| candidate.id == next.id)
                .count()
                != 1
            {
                continue;
            }
            let new_path = storage::checked(root, &next.relative, false)?;
            let old_path = storage::checked(root, &old.relative, false)?;
            let Ok(mut file) = storage::locked_read(&new_path.path, false) else {
                continue;
            };
            if storage::hash(&mut file).ok().as_deref() != old.last_hash.as_deref()
                || !std::fs::symlink_metadata(&old_path.path)
                    .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
            {
                continue;
            }
            let tx = self.db.transaction().map_err(error)?;
            let changed=tx.execute("UPDATE source_files SET disposition='renamed' WHERE id=?1 AND status='missing' AND last_hash=?2 AND NOT EXISTS(SELECT 1 FROM documents WHERE active_source_id=?1) AND NOT EXISTS(SELECT 1 FROM drive_documents WHERE document_id=?3 AND pending_hash IS NOT NULL)",params![old.id,old.last_hash,old.document_id]).map_err(error)?;
            tx.commit().map_err(error)?;
            if changed == 1 {
                self.activity(project,"source_relocated",&format!("{} -> {}. Retired an absent older path after unique identical-content version-rename verification; version metadata and cloud identity retained.",old.relative,next.relative))?;
            }
        }
        Ok(())
    }
    fn latest_source(&self, project: &str, path: &str) -> Result<serde_json::Value> {
        let sources=crate::cloud::rows(&self.db,"SELECT id,document_id,relative_path,status,disposition,last_hash FROM source_files WHERE project_id=?1",&[&project])?;
        let sources = sources
            .into_iter()
            .filter(|s| {
                crate::path_selection::normalized(crate::cloud::s(s, "relative_path")) == path
            })
            .collect::<Vec<_>>();
        if sources.len() != 1 {
            return Err("The missing path is ambiguous or no longer registered. Refresh and review it again.".into());
        }
        Ok(sources.into_iter().next().unwrap())
    }
    pub(crate) fn latest_forget_plan(
        &mut self,
        project: &str,
        path: &str,
    ) -> Result<crate::forget::Plan> {
        crate::folder_recovery::require_ready(&self.db, project)?;
        self.refresh(project)?;
        let path = crate::path_selection::normalized(path);
        let source = self.latest_source(project, &path)?;
        if crate::cloud::s(&source, "status") != "missing" {
            return Err("Only a confirmed missing file can stop being tracked here. Present or inaccessible files are kept.".into());
        }
        if !["tracked", "superseded"].contains(&crate::cloud::s(&source, "disposition")) {
            return Err("Only a tracked file can be forgotten here.".into());
        }
        let revision = crate::path_selection::revision(&self.db, project)?;
        let folder_revision: i64 = self
            .db
            .query_row(
                "SELECT folder_revision FROM projects WHERE id=?1",
                [project],
                |r| r.get(0),
            )
            .map_err(error)?;
        let token = crate::cloud::scope(&format!(
            "{project}:{path}:{revision}:{folder_revision}:{}",
            source
        ));
        Ok(crate::forget::Plan {
            path,
            revision_count: 0,
            bytes: 0,
            blockers: vec![],
            token,
        })
    }
    pub(crate) fn latest_forget(
        &mut self,
        project: &str,
        path: &str,
        token: &str,
    ) -> Result<Vec<String>> {
        let plan = self.latest_forget_plan(project, path)?;
        if plan.token != token {
            return Err(
                "The file or folder selection changed. Review the missing file again.".into(),
            );
        }
        let source = self.latest_source(project, &plan.path)?;
        let source_id = crate::cloud::s(&source, "id");
        let tx = self.db.transaction().map_err(error)?;
        tx.execute(
            "INSERT OR IGNORE INTO path_exclusions(project_id,path,kind) VALUES(?1,?2,'file')",
            params![project, plan.path],
        )
        .map_err(error)?;
        tx.execute("INSERT INTO path_selection(project_id,revision) VALUES(?1,1) ON CONFLICT(project_id) DO UPDATE SET revision=revision+1",[project]).map_err(error)?;
        tx.execute("UPDATE documents SET tracking_state='ignored' WHERE project_id=?1 AND active_source_id=?2",params![project,source_id]).map_err(error)?;
        tx.execute(
            "UPDATE source_files SET disposition='ignored' WHERE project_id=?1 AND id=?2",
            params![project, source_id],
        )
        .map_err(error)?;
        tx.execute(
            "DELETE FROM new_paths WHERE project_id=?1 AND path=?2",
            params![project, plan.path],
        )
        .map_err(error)?;
        tx.commit().map_err(error)?;
        Ok(vec![])
    }
    pub(crate) fn finish_tracking_archival(&mut self, project: &str) -> Result<Vec<String>> {
        if enabled(&self.db) {
            Ok(vec![])
        } else {
            self.finish_archival_intent(project)
        }
    }
    /// Explicit historical-regression fixture; production always opens through
    /// `open` and the current migration's latest-only policy.
    #[cfg(test)]
    pub(crate) fn open_legacy_fixture(path: &Path) -> Result<Self> {
        let r = Self::open(path)?;
        crate::review::set_preference(&r.db, "workflow.latest_only", false)?;
        Ok(r)
    }
    pub(crate) fn stage_tracking(&self, root: &Path, relative: &str) -> Result<storage::Staged> {
        if enabled(&self.db) {
            storage::stage_metadata(root, relative)
        } else {
            storage::stage(root, relative, &self.store)
        }
    }
    pub(crate) fn capture_metadata(
        &mut self,
        source: &Source,
        staged: storage::Staged,
        reason: &str,
    ) -> Result<()> {
        let group = crate::versioning::effective(&self.db, &source.document_id, &source.name)?;
        let previous:Option<(String,String,String,String)>=self.db.query_row("SELECT v.id,v.snapshot_relative,COALESCE(v.version_key,''),v.sha256 FROM documents d JOIN document_versions v ON v.id=d.current_version_id WHERE d.id=?1",[&source.document_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(error)?;
        let reusable = previous
            .as_ref()
            .filter(|(_, path, key, _)| path.starts_with("metadata/") && *key == group)
            .map(|(id, _, _, _)| id.clone());
        let tx = self.db.transaction().map_err(error)?;
        let version = if let Some(version) = reusable {
            tx.execute("UPDATE document_versions SET source_name=?2,source_relative=?3,captured_at=?4,modified_at=?5,size=?6,sha256=?7,reason=?8 WHERE id=?1",params![version,source.name,source.relative,models::now(),staged.modified,staged.size,staged.hash,reason]).map_err(error)?;
            version
        } else {
            let version = uuid::Uuid::new_v4().to_string();
            let number: i64 = tx
                .query_row(
                    "SELECT COALESCE(MAX(number),0)+1 FROM document_versions WHERE document_id=?1",
                    [&source.document_id],
                    |r| r.get(0),
                )
                .map_err(error)?;
            let placeholder = format!("metadata/{}/{version}", source.document_id);
            tx.execute("INSERT INTO document_versions(id,document_id,number,source_name,source_relative,captured_at,modified_at,size,sha256,snapshot_relative,reason,version_key) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",params![version,source.document_id,number,source.name,source.relative,models::now(),staged.modified,staged.size,staged.hash,placeholder,reason,group]).map_err(error)?;
            version
        };
        operations::activate(
            &tx,
            source,
            &version,
            &staged.hash,
            staged.size,
            staged.modified.as_deref(),
        )?;
        tx.commit().map_err(error)?;
        storage::discard_stage(&self.store, &staged.temporary)?;
        if previous.is_none_or(|(_, _, _, hash)| hash != staged.hash) {
            self.activity(
                &source.project_id,
                "tracked",
                &format!(
                    "{}: latest contents tracked and SHA-256 verified.",
                    source.name
                ),
            )?;
        }
        Ok(())
    }
}
