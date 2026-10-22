//! One idempotent pipeline for initialization, watcher events, manual refresh and startup recovery.
use crate::{
    matching, models,
    registry::{error, Registry, Result},
    storage,
    tracking_queries::Source,
};
use rusqlite::{params, OptionalExtension};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};
impl Registry {
    pub fn configure_tracking(
        &mut self,
        project: &str,
        enabled: bool,
        archive: bool,
        selected: Vec<String>,
    ) -> Result<()> {
        let root_string = self.path(project)?;
        let root = Path::new(&root_string);
        let (was_enabled, was_archive, initialized) = self.tracking_flags(project)?;
        if archive && !was_archive {
            self.require_no_restores(project)?;
        }
        if enabled {
            crate::folder_recovery::require_ready(&self.db, project)?;
            let canonical =
                std::fs::canonicalize(root).map_err(|_| "The project folder is unavailable.")?;
            if canonical.starts_with(&self.store) || self.store.starts_with(&canonical) {
                return Err("Tracking is not allowed for folders overlapping ContextDock's application storage. Choose a dedicated project folder.".into());
            }
            let _guard = storage::directory_guard(&canonical)?;
            self.refresh(project)?;
        }
        let selected: HashSet<String> = selected.into_iter().collect();
        for id in &selected {
            let s = self.source(id)?;
            if s.project_id != project
                || s.status != "available"
                || !crate::path_selection::allowed(&self.db, project, &s.relative)?
            {
                return Err("Select available documents from this project only.".into());
            }
        }
        let tx = self.db.transaction().map_err(error)?;
        if enabled && !initialized {
            tx.execute(
                "UPDATE source_files SET disposition='ignored' WHERE project_id=?1",
                [project],
            )
            .map_err(error)?;
            tx.execute(
                "UPDATE documents SET tracking_state='ignored' WHERE project_id=?1",
                [project],
            )
            .map_err(error)?;
            for sid in &selected {
                tx.execute(
                    "UPDATE source_files SET disposition='tracked' WHERE id=?1",
                    [sid],
                )
                .map_err(error)?;
                tx.execute("UPDATE documents SET tracking_state='tracked' WHERE id=(SELECT document_id FROM source_files WHERE id=?1)",[sid]).map_err(error)?;
            }
            tx.execute("INSERT OR IGNORE INTO path_exclusions SELECT project_id,lower(replace(relative_path,char(92),'/')),'file' FROM source_files WHERE project_id=?1 AND disposition='ignored'",[project]).map_err(error)?;
            tx.execute("INSERT INTO path_selection VALUES(?1,1) ON CONFLICT(project_id) DO UPDATE SET revision=revision+1",[project]).map_err(error)?;
        }
        tx.execute("UPDATE projects SET tracking_enabled=?1,archive_enabled=?2,tracking_initialized=CASE WHEN ?1 AND tracking_initialized=0 THEN 2 ELSE tracking_initialized END,watch_status=CASE WHEN ?1 THEN CASE WHEN tracking_enabled=0 THEN 'starting' ELSE watch_status END ELSE 'off' END WHERE id=?3",params![enabled,archive,project]).map_err(error)?;
        if archive != was_archive {
            tx.execute("INSERT INTO archive_intents VALUES(?1,?2,1) ON CONFLICT(project_id) DO UPDATE SET enabled=excluded.enabled,pending=1",params![project,archive]).map_err(error)?;
        }
        tx.commit().map_err(error)?;
        self.verify_tracking_settings(project, enabled, archive)?;
        self.activity(
            project,
            "settings",
            if enabled {
                if archive {
                    "Tracking enabled; automatic archival separately enabled."
                } else {
                    "Tracking enabled; source archival disabled."
                }
            } else {
                "Tracking disabled. All captured versions and source files retained."
            },
        )?;
        if enabled && !initialized {
            let selected_ids: Vec<_> = selected.iter().cloned().collect();
            let grouped = self.prepare_initial_groups(project, &selected_ids, false)?;
            self.db
                .execute(
                    "UPDATE projects SET tracking_initialized=1 WHERE id=?1",
                    [project],
                )
                .map_err(error)?;
            let mut problems = vec![];
            for sid in selected {
                if grouped.contains(&sid) {
                    continue;
                }
                let source = self.source(&sid)?;
                self.db
                    .execute(
                        "UPDATE documents SET family_name=?1 WHERE id=?2",
                        params![matching::stem(&source.name).display, source.document_id],
                    )
                    .map_err(error)?;
                let result = storage::stage(root, &source.relative, &self.store)
                    .and_then(|stage| self.capture(&source, stage, "initial_tracking", false));
                if let Err(e) = result {
                    self.activity(
                        project,
                        "capture_deferred",
                        &format!("{}: {e}", source.relative),
                    )?;
                    problems.push(e);
                }
            }
            problems.extend(self.recover_operations(project, true)?);
            self.set_tracking_error(project, &problems)?;
        } else if enabled && !was_enabled {
            let ids = self.source_ids(project)?;
            self.prepare_initial_groups(project, &ids, true)?;
            self.reconcile(project, "startup_reconciliation", &HashSet::new(), true)?;
        } else if enabled && archive {
            let errors = self.recover_operations(project, true)?;
            self.set_tracking_error(project, &errors)?;
        }
        let errors = self.finish_archival_intent(project)?;
        if !errors.is_empty() {
            self.set_tracking_error(project, &errors)?;
        }
        self.verify_tracking_settings(project, enabled, archive)
    }
    fn verify_tracking_settings(&self, project: &str, enabled: bool, archive: bool) -> Result<()> {
        let (saved_enabled, saved_archive, _) = self.tracking_flags(project)?;
        if saved_enabled != enabled || saved_archive != archive {
            return Err("Tracking setting verification failed. Refresh documents to check the saved state before trying again.".into());
        }
        Ok(())
    }
    pub fn set_tracking_error(&self, project: &str, errors: &[String]) -> Result<()> {
        let message = if errors.is_empty() {
            None
        } else {
            Some(format!(
                "{} operation(s) need attention. {}",
                errors.len(),
                errors[0]
            ))
        };
        self.db
            .execute(
                "UPDATE projects SET tracking_error=?1 WHERE id=?2",
                params![message, project],
            )
            .map_err(error)?;
        Ok(())
    }
    pub fn reconcile(
        &mut self,
        project: &str,
        origin: &str,
        dirty: &HashSet<String>,
        full: bool,
    ) -> Result<bool> {
        self.refresh(project)?;
        let (enabled, archive, _) = self.tracking_flags(project)?;
        if !enabled {
            return Ok(false);
        }
        if crate::folder_recovery::require_ready(&self.db, project).is_err() {
            return Ok(false);
        }
        let root_string = self.path(project)?;
        let root = Path::new(&root_string);
        let available: bool = self
            .db
            .query_row(
                "SELECT status!='unavailable' FROM projects WHERE id=?1",
                [project],
                |r| r.get(0),
            )
            .map_err(error)?;
        if !available {
            self.activity(project,"folder_unavailable","Watched folder is missing or inaccessible. History is retained; reconnect the drive or refresh after access is restored.")?;
            return Ok(false);
        }
        let preparing: bool = self
            .db
            .query_row(
                "SELECT tracking_initialized=2 FROM projects WHERE id=?1",
                [project],
                |r| r.get(0),
            )
            .map_err(error)?;
        if preparing {
            let ids = self.source_ids(project)?;
            self.prepare_initial_groups(project, &ids, false)?;
            self.db
                .execute(
                    "UPDATE projects SET tracking_initialized=1 WHERE id=?1",
                    [project],
                )
                .map_err(error)?;
        }
        let mut problems = self.finish_archival_intent(project)?;
        problems.extend(self.recover_operations(project, full)?);
        let path_rules = crate::path_selection::rules(&self.db, project)?;
        let ids = self.source_ids(project)?;
        let mut incoming: HashMap<(String, String, String), usize> = HashMap::new();
        for sid in &ids {
            let s = self.source(sid)?;
            if s.disposition == "discovered"
                && crate::path_selection::permits(&path_rules, &s.relative)
            {
                *incoming
                    .entry((
                        matching::parent_key(&s.relative),
                        matching::stem(&s.name).normalized,
                        s.kind,
                    ))
                    .or_default() += 1;
            }
        }
        let mut retry = false;
        for sid in ids {
            let source = self.source(&sid)?;
            let importing: bool = self.db.query_row("SELECT EXISTS(SELECT 1 FROM operations WHERE document_id=?1 AND kind='initial_import' AND state!='done')",[&source.document_id],|r|r.get(0)).map_err(error)?;
            if importing {
                continue;
            }
            if !crate::path_selection::permits(&path_rules, &source.relative) {
                continue;
            }
            if source.disposition == "ignored" || source.disposition == "archived" {
                continue;
            }
            let changed = full
                || dirty.contains(&models::key(&source.relative))
                || source.hashed_size != Some(source.size)
                || source.hashed_stamp != source.modified
                || source.last_hash.is_none();
            if !changed {
                continue;
            }
            let staged = match storage::stage(root, &source.relative, &self.store) {
                Ok(s) => s,
                Err(e) => {
                    self.activity(
                        project,
                        "capture_deferred",
                        &format!("{}: {e}", source.relative),
                    )?;
                    problems.push(e);
                    retry = true;
                    continue;
                }
            };
            let result = (|| {
                if source.disposition == "superseded" {
                    if source.last_hash.as_deref() != Some(&staged.hash) {
                        self.queue_review(&source,&staged,vec![source.document_id.clone()],"A superseded source changed. Confirm its document family before capturing; no source will be moved.")?;
                    } else {
                        self.record_observed_hash(&source, &staged)?;
                    }
                    return storage::discard_stage(&self.store, &staged.temporary);
                }
                if source.disposition == "tracked" {
                    let reason = if origin == "startup_reconciliation" {
                        origin
                    } else {
                        "file_modified"
                    };
                    return self.capture(&source, staged, reason, archive);
                }
                let candidates = self.match_candidates(&source, &staged.hash)?;
                let batch_count = incoming
                    .get(&(
                        matching::parent_key(&source.relative),
                        matching::stem(&source.name).normalized.clone(),
                        source.kind.clone(),
                    ))
                    .copied()
                    .unwrap_or(0);
                let (mut matches, explanation, high) = candidates;
                if source.disposition == "review" || batch_count > 1 {
                    self.queue_review(&source,&staged,matches,"Several existing or incoming files may belong to this family, or an earlier match is awaiting confirmation. Review is required; nothing was moved.")?;
                    return storage::discard_stage(&self.store, &staged.temporary);
                }
                if high && matches.len() == 1 {
                    let target = matches.remove(0);
                    self.activity(
                        project,
                        "matched",
                        &format!(
                            "High-confidence match: {} -> {}. {}",
                            source.relative, target, explanation
                        ),
                    )?;
                    self.associate(&source, &target)?;
                    let assigned = self.source(&source.id)?;
                    let reason = if origin == "startup_reconciliation" {
                        origin
                    } else {
                        "new_replacement"
                    };
                    self.capture(&assigned, staged, reason, archive)
                } else if matches.is_empty() {
                    self.db
                        .execute(
                            "UPDATE documents SET tracking_state='tracked' WHERE id=?1",
                            [&source.document_id],
                        )
                        .map_err(error)?;
                    self.db
                        .execute(
                            "UPDATE source_files SET disposition='tracked' WHERE id=?1",
                            [&source.id],
                        )
                        .map_err(error)?;
                    let source = self.source(&source.id)?;
                    self.capture(
                        &source,
                        staged,
                        if origin == "startup_reconciliation" {
                            origin
                        } else {
                            "new_document"
                        },
                        false,
                    )
                } else {
                    self.queue_review(&source, &staged, matches, &explanation)?;
                    storage::discard_stage(&self.store, &staged.temporary)
                }
            })();
            if let Err(e) = result {
                self.activity(
                    project,
                    "capture_deferred",
                    &format!("{}: {e}", source.relative),
                )?;
                problems.push(e);
                retry = true;
            }
        }
        problems.extend(self.recover_operations(project, false)?);
        problems.extend(self.finish_archival_intent(project)?);
        let blocked:i64=self.db.query_row("SELECT COUNT(*) FROM operations WHERE project_id=?1 AND error IS NOT NULL AND state NOT IN ('done','cancelled','restored')",[project],|r|r.get(0)).map_err(error)?;
        if blocked > 0 && problems.is_empty() {
            problems.push("A journal operation needs review or an explicit retry below.".into());
        }
        self.set_tracking_error(project, &problems)?;
        Ok(retry)
    }
    fn record_observed_hash(&self, source: &Source, stage: &storage::Staged) -> Result<()> {
        self.db
            .execute(
                "UPDATE source_files SET last_hash=?1,hashed_stamp=?2,hashed_size=?3 WHERE id=?4",
                params![stage.hash, stage.modified, stage.size, source.id],
            )
            .map_err(error)?;
        Ok(())
    }
    fn queue_review(
        &self,
        source: &Source,
        stage: &storage::Staged,
        mut candidates: Vec<String>,
        explanation: &str,
    ) -> Result<()> {
        candidates.sort();
        candidates.dedup();
        let encoded = serde_json::to_string(&candidates).map_err(|e| e.to_string())?;
        let previous: Option<String> = self
            .db
            .query_row(
                "SELECT observed_hash FROM pending_matches WHERE source_id=?1",
                [&source.id],
                |r| r.get(0),
            )
            .optional()
            .map_err(error)?;
        self.db.execute("INSERT INTO pending_matches(source_id,explanation,candidate_ids,observed_hash,created_at) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(source_id) DO UPDATE SET explanation=excluded.explanation,candidate_ids=excluded.candidate_ids,observed_hash=excluded.observed_hash",params![source.id,explanation,encoded,stage.hash,models::now()]).map_err(error)?;
        self.db
            .execute(
                "UPDATE source_files SET disposition='review' WHERE id=?1",
                [&source.id],
            )
            .map_err(error)?;
        self.record_observed_hash(source, stage)?;
        if previous.as_deref() != Some(&stage.hash) {
            self.activity(
                &source.project_id,
                "needs_review",
                &format!(
                    "{} needs a document-family decision. {explanation}",
                    source.relative
                ),
            )?;
        }
        Ok(())
    }
    fn match_candidates(&self, source: &Source, hash: &str) -> Result<(Vec<String>, String, bool)> {
        let blocked = crate::path_selection::blocked_documents(&self.db, &source.project_id)?;
        let stem = matching::stem(&source.name);
        let parent = matching::parent_key(&source.relative);
        let mut q=self.db.prepare("SELECT DISTINCT a.document_id FROM document_aliases a JOIN documents d ON d.id=a.document_id WHERE a.project_id=?1 AND a.parent_key=?2 AND a.stem=?3 AND a.kind=?4 AND d.merged_into IS NULL AND d.tracking_state='tracked'").map_err(error)?;
        let aliases = q
            .query_map(
                params![source.project_id, parent, stem.normalized, source.kind],
                |r| r.get(0),
            )
            .map_err(error)?
            .collect::<std::result::Result<Vec<String>, _>>()
            .map_err(error)?
            .into_iter()
            .filter(|id| !blocked.contains(id))
            .collect::<Vec<_>>();
        if !aliases.is_empty() {
            return Ok((
                aliases,
                "User-established association for this normalized name and directory.".into(),
                true,
            ));
        }
        let mut q=self.db.prepare("SELECT DISTINCT d.id,s.status FROM document_versions v JOIN documents d ON d.id=v.document_id LEFT JOIN source_files s ON s.id=d.active_source_id WHERE d.project_id=?1 AND d.kind=?2 AND d.tracking_state='tracked' AND d.merged_into IS NULL AND v.sha256=?3 AND d.id!=?4").map_err(error)?;
        let duplicates = q
            .query_map(
                params![source.project_id, source.kind, hash, source.document_id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?)),
            )
            .map_err(error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(error)?
            .into_iter()
            .filter(|(id, _)| !blocked.contains(id))
            .collect::<Vec<_>>();
        if !duplicates.is_empty() {
            let relocated = duplicates.len() == 1 && duplicates[0].1.as_deref() == Some("missing");
            return Ok((duplicates.into_iter().map(|(id,_)|id).collect(),"Identical preserved content at another path: possible relocation or duplicate. No new content version is needed.".into(),relocated));
        }
        let mut q=self.db.prepare("SELECT d.id,s.name,s.relative_path FROM documents d JOIN source_files s ON s.id=d.active_source_id WHERE d.project_id=?1 AND d.kind=?2 AND d.tracking_state='tracked' AND d.merged_into IS NULL AND d.id!=?3").map_err(error)?;
        let rows = q
            .query_map(
                params![source.project_id, source.kind, source.document_id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                },
            )
            .map_err(error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(error)?;
        let mut matches = vec![];
        let mut high = false;
        for (id, name, relative) in rows {
            if blocked.contains(&id) {
                continue;
            }
            let other = matching::stem(&name);
            if !stem.normalized.is_empty() && stem.normalized == other.normalized {
                high = matching::parent_key(&relative) == parent
                    && (stem.versioned || other.versioned);
                matches.push(id);
            }
        }
        if matches.len() != 1 {
            high = false
        }
        Ok((matches,if high {"Exact normalized stem, type and directory, with a recognizable version/date suffix and one matching family."} else {"Normalized stems agree after removing trailing dates/versions. Multiple candidates, a different directory, or lack of a version suffix needs confirmation."}.into(),high))
    }
    fn associate(&mut self, source: &Source, target: &str) -> Result<()> {
        let tx = self.db.transaction().map_err(error)?;
        tx.execute(
            "UPDATE source_files SET document_id=?1,disposition='tracked' WHERE id=?2",
            params![target, source.id],
        )
        .map_err(error)?;
        if target != source.document_id {
            tx.execute("UPDATE documents SET tracking_state='merged',merged_into=?1,active_source_id=NULL WHERE id=?2 AND current_version_id IS NULL AND NOT EXISTS(SELECT 1 FROM source_files WHERE document_id=?2)",params![target,source.document_id]).map_err(error)?;
        }
        tx.commit().map_err(error)?;
        Ok(())
    }
    pub fn resolve_match(
        &mut self,
        project: &str,
        source_id: &str,
        action: &str,
        target: Option<String>,
    ) -> Result<()> {
        self.path(project)?;
        crate::folder_recovery::require_ready(&self.db, project)?;
        let source = self.source(source_id)?;
        if source.project_id != project {
            return Err("Source is outside this project.".into());
        }
        if action == "ignore" {
            if crate::path_selection::allowed(&self.db, project, &source.relative)? {
                let mut rules = crate::path_selection::rules(&self.db, project)?;
                rules.push(crate::path_selection::Rule {
                    path: source.relative.clone(),
                    kind: "file".into(),
                });
                let revision = crate::path_selection::revision(&self.db, project)?;
                self.save_path_selection(project, revision, rules)?;
            }
            self.db
                .execute(
                    "UPDATE source_files SET disposition='ignored' WHERE id=?1",
                    [source_id],
                )
                .map_err(error)?;
            self.db
                .execute(
                    "DELETE FROM pending_matches WHERE source_id=?1",
                    [source_id],
                )
                .map_err(error)?;
            self.db.execute("UPDATE documents SET tracking_state='ignored' WHERE id=?1 AND current_version_id IS NULL",[&source.document_id]).map_err(error)?;
            self.activity(
                project,
                "ignored",
                &format!("{} ignored for tracking. Source retained.", source.relative),
            )?;
            return Ok(());
        }
        if !crate::path_selection::allowed(&self.db, project, &source.relative)? {
            return Err("This file is excluded by the saved folder selection. Include it in the folder tree and save first.".into());
        }
        let (enabled, archive, _) = self.tracking_flags(project)?;
        if !enabled {
            return Err("Enable tracking before assigning or capturing a document.".into());
        }
        let root = self.path(project)?;
        let stage = storage::stage(Path::new(&root), &source.relative, &self.store)?;
        let target = match action {
            "assign" => {
                let target = target.ok_or("Choose a document family.")?;
                if crate::path_selection::blocked_documents(&self.db, project)?.contains(&target) {
                    storage::discard_stage(&self.store, &stage.temporary)?;
                    return Err(
                        "Include the target family's folder before assigning this file.".into(),
                    );
                }
                let valid:bool=self.db.query_row("SELECT EXISTS(SELECT 1 FROM documents WHERE id=?1 AND project_id=?2 AND kind=?3 AND merged_into IS NULL AND tracking_state='tracked')",params![target,project,source.kind],|r|r.get(0)).map_err(error)?;
                if !valid {
                    storage::discard_stage(&self.store, &stage.temporary)?;
                    return Err(
                        "Choose an existing tracked family of the same type in this project."
                            .into(),
                    );
                }
                target
            }
            "new" => {
                let count: i64 = self
                    .db
                    .query_row(
                        "SELECT COUNT(*) FROM document_versions WHERE document_id=?1",
                        [&source.document_id],
                        |r| r.get(0),
                    )
                    .map_err(error)?;
                if count == 0 {
                    source.document_id.clone()
                } else {
                    let id = uuid::Uuid::new_v4().to_string();
                    self.db.execute("INSERT INTO documents(id,project_id,family_name,kind,tracking_state) VALUES(?1,?2,?3,?4,'tracked')",params![id,project,matching::stem(&source.name).display,source.kind]).map_err(error)?;
                    id
                }
            }
            _ => {
                storage::discard_stage(&self.store, &stage.temporary)?;
                return Err("Unknown match decision.".into());
            }
        };
        self.associate(&source, &target)?;
        self.db
            .execute(
                "UPDATE documents SET tracking_state='tracked' WHERE id=?1",
                [&target],
            )
            .map_err(error)?;
        if action == "assign" {
            self.db.execute("INSERT OR IGNORE INTO document_aliases(project_id,parent_key,stem,kind,document_id) VALUES(?1,?2,?3,?4,?5)",params![project,matching::parent_key(&source.relative),matching::stem(&source.name).normalized,source.kind,target]).map_err(error)?;
        }
        let assigned = self.source(source_id)?;
        self.capture(
            &assigned,
            stage,
            "manual_assignment",
            archive && action == "assign",
        )?;
        let errors = self.recover_operations(project, false)?;
        self.set_tracking_error(project, &errors)?;
        Ok(())
    }
}
