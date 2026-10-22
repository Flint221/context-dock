//! Ordered, replayable import of explicit numbered exports. Sources never move.
use crate::{
    matching, models,
    registry::{error, Registry, Result},
    storage,
    tracking_queries::Source,
};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    path::Path,
};

#[derive(Serialize, Deserialize)]
struct Item {
    source: String,
    expected_hash: Option<String>,
}
#[derive(Serialize, Deserialize)]
struct Import {
    items: Vec<Item>,
    position: usize,
}
type GroupKey = (String, String, String);
type OrderedSource = (Vec<u64>, Source);

impl Registry {
    pub(crate) fn prepare_initial_groups(
        &mut self,
        project: &str,
        ids: &[String],
        retained: bool,
    ) -> Result<HashSet<String>> {
        let mut groups: BTreeMap<GroupKey, Vec<OrderedSource>> = BTreeMap::new();
        for id in ids {
            let source = self.source(id)?;
            if source.project_id != project
                || source.status != "available"
                || source.disposition != "tracked"
                || !crate::path_selection::allowed(&self.db, project, &source.relative)?
            {
                continue;
            }
            let stem = matching::stem(&source.name);
            if let Some(order) = matching::version_order(&source.name) {
                if !stem.normalized.is_empty() {
                    groups
                        .entry((
                            matching::parent_key(&source.relative),
                            stem.normalized,
                            source.kind.clone(),
                        ))
                        .or_default()
                        .push((order, source));
                }
            }
        }
        let mut grouped = HashSet::new();
        let mut notices = Vec::new();
        for (_, mut members) in groups {
            if members.len() < 2 {
                continue;
            }
            members.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.relative.cmp(&b.1.relative)));
            if members.windows(2).any(|w| w[0].0 == w[1].0) {
                notices.push(
                    "Conflicting filename version numbers remain separate; no order was guessed.",
                );
                self.activity(
                    project,
                    "import_review",
                    "Conflicting filename version numbers remain separate; no order was guessed.",
                )?;
                continue;
            }
            let documents: HashSet<_> = members.iter().map(|(_, s)| &s.document_id).collect();
            if documents.len() != members.len() {
                continue;
            } // Already grouped or manually associated.
            if retained {
                let cloud_resources: bool = self.db.query_row("SELECT EXISTS(SELECT 1 FROM cloud_projects WHERE project_id=?1 AND (enabled=1 OR (store_id IS NOT NULL AND store_state!='deleted'))) OR EXISTS(SELECT 1 FROM cloud_jobs WHERE project_id=?1 AND file_id IS NOT NULL AND state!='purged')", [project], |r| r.get(0)).map_err(error)?;
                if cloud_resources {
                    notices.push("Existing separate histories cannot be regrouped while cloud resources remain. Delete project cloud data, then stop/resume tracking to retry. Local history is retained.");
                    self.activity(project, "import_review", "Existing separate histories were retained because cloud resources still exist. Delete project cloud data before resuming tracking to regroup initial filename versions safely.")?;
                    continue;
                }
            }
            let mut items = Vec::new();
            let mut safe = true;
            for (_, s) in &members {
                let versions: Vec<(String, String, String)> = {
                    let mut q = self.db.prepare("SELECT sha256,snapshot_relative,reason FROM document_versions WHERE document_id=?1").map_err(error)?;
                    let result = q
                        .query_map([&s.document_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
                        .map_err(error)?
                        .collect::<std::result::Result<Vec<_>, _>>()
                        .map_err(error)?;
                    result
                };
                let conflict: bool = self.db.query_row("SELECT EXISTS(SELECT 1 FROM operations WHERE document_id=?1 AND state NOT IN ('done','cancelled','restored')) OR EXISTS(SELECT 1 FROM pending_matches WHERE source_id=?2) OR EXISTS(SELECT 1 FROM document_aliases WHERE document_id=?1) OR (SELECT COUNT(*) FROM source_files WHERE document_id=?1)!=1",params![s.document_id,s.id],|r|r.get(0)).map_err(error)?;
                if conflict
                    || versions.len() > 1
                    || versions.first().is_some_and(|v| v.2 != "initial_tracking")
                {
                    safe = false;
                    break;
                }
                let expected_hash = if let Some((hash, snapshot, _)) = versions.first() {
                    let snapshot = storage::checked(&self.store, snapshot, false)?;
                    storage::verify(&snapshot.path, hash)?;
                    let root = self.path(project)?;
                    let staged = storage::stage(Path::new(&root), &s.relative, &self.store)?;
                    let same = staged.hash == *hash;
                    storage::discard_stage(&self.store, &staged.temporary)?;
                    if !same {
                        safe = false;
                        break;
                    }
                    Some(hash.clone())
                } else {
                    None
                };
                items.push(Item {
                    source: s.id.clone(),
                    expected_hash,
                });
            }
            if !safe {
                notices.push("Changed, manually assigned or multi-version histories were preserved separately. Automatic regrouping only repairs unchanged initial snapshots.");
                self.activity(project, "import_review", "A filename sequence has changed, manually assigned or multi-version histories. Its existing histories were preserved without automatic regrouping.")?;
                continue;
            }
            let target = members[0].1.document_id.clone();
            let operation = uuid::Uuid::new_v4().to_string();
            let payload =
                serde_json::to_string(&Import { items, position: 0 }).map_err(|e| e.to_string())?;
            let tx = self.db.transaction().map_err(error)?;
            for (_, source) in &members {
                if retained {
                    // Grouping changes family scope. Require explicit cloud re-selection
                    // rather than broadening an old family/history opt-in.
                    tx.execute(
                        "DELETE FROM cloud_documents WHERE project_id=?1 AND document_id=?2",
                        params![project, source.document_id],
                    )
                    .map_err(error)?;
                }
                tx.execute(
                    "UPDATE source_files SET document_id=?1 WHERE id=?2",
                    params![target, source.id],
                )
                .map_err(error)?;
                if source.document_id != target {
                    tx.execute("UPDATE documents SET merged_into=?1,active_source_id=NULL,tracking_state=CASE WHEN current_version_id IS NULL THEN 'merged' ELSE 'grouped_history' END WHERE id=?2",params![target,source.document_id]).map_err(error)?;
                }
            }
            tx.execute(
                "UPDATE documents SET family_name=?1 WHERE id=?2",
                params![matching::stem(&members[0].1.name).display, target],
            )
            .map_err(error)?;
            tx.execute("INSERT OR IGNORE INTO document_aliases(project_id,parent_key,stem,kind,document_id) VALUES(?1,?2,?3,?4,?5)",params![project,matching::parent_key(&members[0].1.relative),matching::stem(&members[0].1.name).normalized,members[0].1.kind,target]).map_err(error)?;
            if retained {
                tx.execute(
                    "UPDATE cloud_projects SET revision=revision+1 WHERE project_id=?1",
                    [project],
                )
                .map_err(error)?;
            }
            tx.execute("INSERT INTO operations(id,project_id,document_id,dedupe_key,kind,payload,created_at,updated_at) VALUES(?1,?2,?3,?4,'initial_import',?5,?6,?6)",params![operation,project,target,format!("initial_import:{target}"),payload,models::now()]).map_err(error)?;
            tx.commit().map_err(error)?;
            self.activity(project,"import_grouped",&format!("Grouped {} numbered exports in filename order. Original files and prior snapshots remain intact.",members.len()))?;
            grouped.extend(members.into_iter().map(|(_, s)| s.id));
        }
        notices.sort_unstable();
        notices.dedup();
        self.db
            .execute(
                "DELETE FROM initial_import_notices WHERE project_id=?1",
                [project],
            )
            .map_err(error)?;
        if !notices.is_empty() {
            self.db
                .execute(
                    "INSERT INTO initial_import_notices VALUES(?1,?2)",
                    params![project, notices.join(" ")],
                )
                .map_err(error)?;
        }
        Ok(grouped)
    }

    pub(crate) fn finish_initial_import(&mut self, operation: &str, payload: &str) -> Result<()> {
        let mut plan: Import = serde_json::from_str(payload).map_err(|e| e.to_string())?;
        let (project,document): (String,String) = self.db.query_row("SELECT project_id,document_id FROM operations WHERE id=?1 AND kind='initial_import'",[operation],|r|Ok((r.get(0)?,r.get(1)?))).map_err(error)?;
        while let Some(item) = plan.items.get(plan.position) {
            let source = self.source(&item.source)?;
            if source.project_id != project || source.document_id != document {
                return Err("An imported file's association changed. Import stopped without capturing into another family.".into());
            }
            let pending: bool = self.db.query_row("SELECT EXISTS(SELECT 1 FROM operations WHERE document_id=?1 AND kind='capture' AND state!='done')",[&source.document_id],|r|r.get(0)).map_err(error)?;
            if pending {
                return Err("A numbered-file import is waiting for its interrupted capture to recover. Scan for changes to retry.".into());
            }
            if !crate::path_selection::allowed(&self.db, &source.project_id, &source.relative)? {
                return Err(format!("Import paused by folder selection: {}. Include it and Save changes to continue.",source.relative));
            }
            let root = self.path(&source.project_id)?;
            let staged = storage::stage(Path::new(&root), &source.relative, &self.store)
                .map_err(|e| format!("{}: {e}", source.relative))?;
            if item
                .expected_hash
                .as_ref()
                .is_some_and(|hash| *hash != staged.hash)
            {
                storage::discard_stage(&self.store, &staged.temporary)?;
                return Err(format!("{} changed during regrouping. Earlier snapshots are retained; restore the original bytes before retrying this import.",source.relative));
            }
            self.capture(&source, staged, "initial_tracking", false)?;
            plan.position += 1;
            self.db
                .execute(
                    "UPDATE operations SET payload=?1,error=NULL,updated_at=?2 WHERE id=?3",
                    params![
                        serde_json::to_string(&plan).map_err(|e| e.to_string())?,
                        models::now(),
                        operation
                    ],
                )
                .map_err(error)?;
        }
        self.db
            .execute(
                "UPDATE operations SET state='done',error=NULL,updated_at=?1 WHERE id=?2",
                params![models::now(), operation],
            )
            .map_err(error)?;
        Ok(())
    }
}
