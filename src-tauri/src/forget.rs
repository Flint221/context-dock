//! Explicit removal of a missing path, never an implicit consequence of Ignore.
use crate::{
    cloud::{self, s},
    folder_recovery, path_selection,
    registry::{error, Registry, Result},
    storage,
};
use rusqlite::params;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub path: String,
    pub revision_count: usize,
    pub bytes: i64,
    pub blockers: Vec<String>,
    pub token: String,
}
struct Review {
    plan: Plan,
    source: String,
    versions: Vec<Value>,
    documents: BTreeSet<String>,
}
impl Registry {
    fn forget_review(&mut self, project: &str, path: &str) -> Result<Review> {
        folder_recovery::require_ready(&self.db, project)?;
        self.refresh(project)?;
        folder_recovery::require_ready(&self.db, project)?;
        let project_row = cloud::rows(
            &self.db,
            "SELECT folder_revision,folder_state,scan_error FROM projects WHERE id=?1",
            &[&project],
        )?
        .remove(0);
        if !s(&project_row, "scan_error").is_empty() {
            return Err("Complete a successful folder scan before forgetting a file. An unreadable folder is not a deleted file.".into());
        }
        let path = path_selection::normalized(path);
        let sources = cloud::rows(&self.db,"SELECT * FROM source_files WHERE project_id=?1 AND lower(replace(relative_path,char(92),'/'))=?2", &[&project,&path])?;
        let source = sources
            .first()
            .ok_or("This file is no longer in discovery. Reload the folder tree.")?;
        if sources.len() != 1 {
            return Err("Multiple discovered paths differ only by case. Resolve that ambiguity before forgetting a file.".into());
        }
        if s(source, "status") != "missing" {
            return Err("Only a confirmed missing file can be forgotten. This file is present or inaccessible; nothing was removed.".into());
        }
        let versions = cloud::rows(&self.db,"SELECT v.* FROM document_versions v JOIN documents d ON d.id=v.document_id WHERE d.project_id=?1 AND lower(replace(v.source_relative,char(92),'/'))=?2 ORDER BY v.id", &[&project,&path])?;
        let mut documents = BTreeSet::from([s(source, "document_id").to_string()]);
        documents.extend(versions.iter().map(|v| s(v, "document_id").to_string()));
        let mut blockers = BTreeSet::new();
        let mut family_state = vec![];
        for document in &documents {
            let state = cloud::rows(&self.db,"SELECT d.*, (SELECT count(*) FROM source_files f WHERE f.document_id=d.id) sources,(SELECT count(*) FROM document_versions v WHERE v.document_id=d.id) revisions FROM documents d WHERE id=?1", &[&document])?;
            let deleting_empty_family = state.first().is_some_and(|d| {
                cloud::n(d, "sources") == i64::from(document == s(source, "document_id"))
                    && cloud::n(d, "revisions")
                        == versions
                            .iter()
                            .filter(|v| s(v, "document_id") == document)
                            .count() as i64
            });
            family_state.push(state);
            let pending: bool = self.db.query_row("SELECT EXISTS(SELECT 1 FROM operations WHERE document_id=?1 AND state NOT IN ('done','cancelled','restored'))",[document],|r|r.get(0)).map_err(error)?;
            if pending {
                blockers.insert("Tracking or archival work is unfinished for this document family. Finish or resolve it before deleting history.".to_string());
            }
            let merged: bool = self
                .db
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM documents d WHERE d.merged_into=?1 AND (EXISTS(SELECT 1 FROM source_files WHERE document_id=d.id) OR EXISTS(SELECT 1 FROM document_versions WHERE document_id=d.id) OR EXISTS(SELECT 1 FROM operations WHERE document_id=d.id) OR EXISTS(SELECT 1 FROM documents WHERE merged_into=d.id) OR EXISTS(SELECT 1 FROM pending_matches m,json_each(m.candidate_ids) c WHERE c.value=d.id)))",
                    [document],
                    |r| r.get(0),
                )
                .map_err(error)?;
            if merged && deleting_empty_family {
                blockers.insert("Earlier grouped histories still refer to this family. Its local history cannot be permanently removed here.".to_string());
            }
            let candidates: bool = self.db.query_row("SELECT EXISTS(SELECT 1 FROM pending_matches m, json_each(m.candidate_ids) c WHERE c.value=?1 AND m.source_id!=?2)",params![document,s(source,"id")],|r|r.get(0)).map_err(error)?;
            if candidates {
                blockers.insert("Another file is awaiting a family-match decision that uses this history. Resolve that match first.".to_string());
            }
        }
        let mut cloud_state = vec![];
        for version in &versions {
            let jobs = cloud::rows(
                &self.db,
                "SELECT * FROM cloud_jobs WHERE version_id=?1",
                &[&s(version, "id")],
            )?;
            if jobs.iter().any(|j| {
                !s(j, "file_id").is_empty()
                    || cloud::n(j, "attempts") > 0
                    || !["pending", "excluded"].contains(&s(j, "state"))
                    || s(j, "phase") != "upload"
            }) {
                blockers.insert("This file has uploaded or unresolved cloud data. Its cloud references must be cleaned up and verified before permanently deleting local history. Ignore or exclude keeps those references safe.".into());
            }
            cloud_state.push(jobs);
            let baseline: bool = self
                .db
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM ai_baselines WHERE source_version=?1)",
                    [s(version, "id")],
                    |r| r.get(0),
                )
                .map_err(error)?;
            if baseline {
                blockers.insert("An approved AI baseline refers to this saved revision. That retained baseline history prevents deleting this revision.".into());
            }
            if !s(version, "archive_relative").is_empty() {
                blockers.insert("A saved revision has an archival record. Restore its archived copy before forgetting this path; archive files will not be deleted.".into());
            }
        }
        let cloud_revision = cloud::rows(
            &self.db,
            "SELECT revision FROM cloud_projects WHERE project_id=?1",
            &[&project],
        )?;
        let fingerprint = json!([
            project,
            path,
            source,
            versions,
            family_state,
            cloud_state,
            cloud_revision,
            project_row,
            path_selection::revision(&self.db, project)?
        ]);
        let token = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&fingerprint).map_err(|e| e.to_string())?)
        );
        Ok(Review {
            plan: Plan {
                path: s(source, "relative_path").to_string(),
                revision_count: versions.len(),
                bytes: versions.iter().map(|v| cloud::n(v, "size")).sum(),
                blockers: blockers.into_iter().collect(),
                token,
            },
            source: s(source, "id").to_string(),
            versions,
            documents,
        })
    }
    pub fn preview_forget(&mut self, project: &str, path: &str) -> Result<Plan> {
        Ok(self.forget_review(project, path)?.plan)
    }
    pub fn forget_missing(
        &mut self,
        project: &str,
        path: &str,
        token: &str,
    ) -> Result<Vec<String>> {
        let review = self.forget_review(project, path)?;
        if review.plan.token != token {
            return Err("This file or its history changed. Review Forget missing file again. Nothing was removed.".into());
        }
        if !review.plan.blockers.is_empty() {
            return Err(review.plan.blockers.join(" "));
        }
        let path = path_selection::normalized(path);
        let tx = self.db.transaction().map_err(error)?;
        for v in &review.versions {
            let relative = s(v, "snapshot_relative");
            if !relative.starts_with(&format!("snapshots/{project}/")) {
                return Err("Snapshot storage identity mismatch. Nothing was removed.".into());
            }
            // Validate the path even if its bytes have already gone missing.
            if relative
                .replace('\\', "/")
                .split('/')
                .any(|c| c.is_empty() || c == "." || c == ".." || c.contains(':'))
            {
                return Err("Unsafe snapshot path. Nothing was removed.".into());
            }
            tx.execute("INSERT INTO forgotten_snapshots(snapshot_relative,sha256,project_id) VALUES(?1,?2,?3)",params![relative,s(v,"sha256"),project]).map_err(error)?;
            tx.execute(
                "DELETE FROM cloud_exclusions WHERE version_id=?1",
                [s(v, "id")],
            )
            .map_err(error)?;
            tx.execute("DELETE FROM cloud_jobs WHERE version_id=?1", [s(v, "id")])
                .map_err(error)?;
            tx.execute("DELETE FROM document_versions WHERE id=?1", [s(v, "id")])
                .map_err(error)?;
        }
        tx.execute(
            "DELETE FROM pending_matches WHERE source_id=?1",
            [&review.source],
        )
        .map_err(error)?;
        tx.execute("DELETE FROM source_files WHERE id=?1", [&review.source])
            .map_err(error)?;
        for document in &review.documents {
            tx.execute("UPDATE documents SET current_version_id=(SELECT id FROM document_versions WHERE document_id=?1 ORDER BY number DESC LIMIT 1) WHERE id=?1 AND (current_version_id IS NULL OR NOT EXISTS(SELECT 1 FROM document_versions WHERE id=current_version_id))",[document]).map_err(error)?;
            tx.execute("UPDATE documents SET active_source_id=(SELECT id FROM source_files WHERE document_id=?1 ORDER BY CASE status WHEN 'available' THEN 0 ELSE 1 END,relative_path LIMIT 1) WHERE id=?1 AND (active_source_id IS NULL OR NOT EXISTS(SELECT 1 FROM source_files WHERE id=active_source_id))",[document]).map_err(error)?;
            let empty:bool=tx.query_row("SELECT NOT EXISTS(SELECT 1 FROM source_files WHERE document_id=?1) AND NOT EXISTS(SELECT 1 FROM document_versions WHERE document_id=?1)",[document],|r|r.get(0)).map_err(error)?;
            if empty {
                // Initial import leaves empty merged placeholders. Retained
                // histories were blocked above; remove only these empty shells.
                tx.execute("DELETE FROM document_aliases WHERE document_id IN (SELECT id FROM documents WHERE merged_into=?1)",[document]).map_err(error)?;
                tx.execute("DELETE FROM cloud_documents WHERE document_id IN (SELECT id FROM documents WHERE merged_into=?1)",[document]).map_err(error)?;
                tx.execute("DELETE FROM documents WHERE merged_into=?1", [document])
                    .map_err(error)?;
                tx.execute(
                    "DELETE FROM document_aliases WHERE document_id=?1",
                    [document],
                )
                .map_err(error)?;
                tx.execute(
                    "DELETE FROM cloud_documents WHERE document_id=?1",
                    [document],
                )
                .map_err(error)?;
                tx.execute("DELETE FROM operations WHERE document_id=?1", [document])
                    .map_err(error)?;
                tx.execute("DELETE FROM documents WHERE id=?1", [document])
                    .map_err(error)?;
            }
        }
        for table in ["path_exclusions", "new_paths", "review_decisions"] {
            tx.execute(
                &format!("DELETE FROM {table} WHERE project_id=?1 AND path=?2"),
                params![project, path],
            )
            .map_err(error)?;
        }
        tx.execute("INSERT INTO path_selection VALUES(?1,1) ON CONFLICT(project_id) DO UPDATE SET revision=revision+1",[project]).map_err(error)?;
        tx.execute(
            "UPDATE cloud_projects SET revision=revision+1 WHERE project_id=?1",
            [project],
        )
        .map_err(error)?;
        tx.execute("INSERT INTO tracking_activity(id,project_id,kind,message,created_at) VALUES(?1,?2,'forget',?3,?4)",params![uuid::Uuid::new_v4().to_string(),project,format!("Forgot one missing file and {} captured local revisions by explicit confirmation.",review.versions.len()),crate::models::now()]).map_err(error)?;
        tx.commit().map_err(error)?;
        Ok(self.purge_forgotten_snapshots().unwrap_or_else(|e| vec![format!("Removal saved. Local snapshot cleanup is pending and will retry after restart: {e}")]))
    }
    pub fn purge_forgotten_snapshots(&self) -> Result<Vec<String>> {
        let rows = cloud::rows(
            &self.db,
            "SELECT * FROM forgotten_snapshots ORDER BY snapshot_relative",
            &[],
        )?;
        let mut warnings = vec![];
        for row in rows {
            let path = s(&row, "snapshot_relative");
            let result = (|| {
                if !path.starts_with(&format!("snapshots/{}/", s(&row, "project_id"))) {
                    return Err("Snapshot path is outside its project storage.".into());
                }
                let referenced: bool = self
                    .db
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM document_versions WHERE snapshot_relative=?1)",
                        [path],
                        |r| r.get(0),
                    )
                    .map_err(error)?;
                if referenced {
                    return Err("A remaining revision still uses this snapshot.".into());
                }
                storage::delete_snapshot(&self.store, path, s(&row, "sha256"))
            })();
            match result {
                Ok(()) => {
                    self.db
                        .execute(
                            "DELETE FROM forgotten_snapshots WHERE snapshot_relative=?1",
                            [path],
                        )
                        .map_err(error)?;
                }
                Err(message) => {
                    self.db
                        .execute(
                            "UPDATE forgotten_snapshots SET error=?2 WHERE snapshot_relative=?1",
                            params![path, message],
                        )
                        .map_err(error)?;
                    warnings.push(format!("Local snapshot cleanup is pending: {message} Close applications using the snapshot, then reload the folder tree or restart ContextDock to retry."));
                }
            }
        }
        Ok(warnings)
    }
}
