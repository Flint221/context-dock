//! Reviewed reversible source moves; snapshot copies are never restored over working files.
use crate::{
    cloud, matching, models,
    operations::ArchivePlan,
    registry::{error, Registry, Result},
    storage,
};
use rusqlite::{params, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewItem {
    pub from: String,
    pub to: String,
    pub action: String,
    pub problem: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct Review {
    pub token: String,
    pub enabled: bool,
    pub items: Vec<ReviewItem>,
    pub warnings: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct RestorePlan {
    pub archive_operation: String,
    pub archive: ArchivePlan,
    pub original: String,
}
struct Prepared {
    review: Review,
    archives: Vec<ArchivePlan>,
    restores: Vec<RestorePlan>,
    cancellations: Vec<String>,
}

fn mark(root: &Path, relative: &str) -> Value {
    let result = (|| -> Result<Value> {
        if !root
            .join(relative)
            .try_exists()
            .map_err(|e| e.to_string())?
        {
            return Ok(json!({"missing":true}));
        }
        let path = storage::checked(root, relative, false)?;
        let mut file = storage::locked_read(&path.path, false)?;
        let metadata = file.metadata().map_err(|e| e.to_string())?;
        Ok(
            json!({"hash":storage::hash(&mut file)?,"identity":storage::file_identity(&file)?,"size":metadata.len(),"stamp":storage::stamp(&metadata)}),
        )
    })();
    result.unwrap_or_else(|e| json!({"error":e}))
}
fn fingerprint(facts: &mut BTreeMap<String, Value>, root: &Path, relative: &str) -> Value {
    facts
        .entry(format!("{}:{relative}", root.display()))
        .or_insert_with(|| mark(root, relative))
        .clone()
}
fn encode<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value).map_err(|e| e.to_string())
}

impl Registry {
    fn prepare_archival(&self, project: &str, enabled: bool) -> Result<Prepared> {
        crate::folder_recovery::require_ready(&self.db, project)?;
        let root_string = self.path(project)?;
        let root = Path::new(&root_string);
        let mut archives = Vec::new();
        let mut restores = Vec::new();
        let mut cancellations = Vec::new();
        let mut items = Vec::new();
        let mut warnings = Vec::new();
        let mut facts = BTreeMap::new();
        let operations=cloud::rows(&self.db,"SELECT id,payload,state FROM operations WHERE project_id=?1 AND kind='archive' AND state NOT IN ('cancelled','restored') ORDER BY id",&[&project])?;
        if enabled {
            self.require_no_restores(project)?;
            let sources=cloud::rows(&self.db,"SELECT s.id FROM source_files s JOIN documents d ON d.id=s.document_id WHERE s.project_id=?1 AND s.disposition='superseded' AND s.status!='archived' AND d.merged_into IS NULL AND d.tracking_state='tracked' AND d.active_source_id!=s.id AND d.current_version_id IS NOT NULL ORDER BY s.relative_path",&[&project])?;
            for row in sources {
                let old = self.source(cloud::s(&row, "id"))?;
                if operations.iter().any(|op| {
                    cloud::s(op, "state") != "done"
                        && serde_json::from_str::<ArchivePlan>(cloud::s(op, "payload"))
                            .is_ok_and(|p| p.old_source == old.id)
                }) {
                    continue;
                }
                let (current, next, family):(String,String,String)=self.db.query_row("SELECT active_source_id,current_version_id,family_name FROM documents WHERE id=?1",[&old.document_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(error)?;
                let replacement = self.source(&current)?;
                if !crate::path_selection::allowed(&self.db, project, &old.relative)?
                    || !crate::path_selection::allowed(&self.db, project, &replacement.relative)?
                {
                    warnings.push(format!(
                        "{} is excluded by folder selection and will stay in place.",
                        old.relative
                    ));
                    continue;
                }
                let pending:bool=self.db.query_row("SELECT EXISTS(SELECT 1 FROM operations WHERE document_id=?1 AND kind IN ('capture','initial_import') AND state!='done') OR EXISTS(SELECT 1 FROM pending_matches WHERE source_id=?2 OR source_id=?3)",params![old.document_id,old.id,current],|r|r.get(0)).map_err(error)?;
                if pending {
                    warnings.push(format!("{} waits for its document history/review to finish. Review organization again afterward.",old.relative));
                    continue;
                }
                let version: Option<String> = self
                    .db
                    .query_row(
                        "SELECT id FROM document_versions WHERE document_id=?1 AND sha256=?2",
                        params![old.document_id, old.last_hash],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(error)?;
                let Some(version) = version else {
                    warnings.push(format!(
                        "{} has no matching verified snapshot and will stay in place.",
                        old.relative
                    ));
                    continue;
                };
                let old_mark = fingerprint(&mut facts, root, &old.relative);
                archives.push(ArchivePlan {
                    project: project.into(),
                    document: old.document_id.clone(),
                    old_version: version,
                    new_version: next,
                    old_source: old.id.clone(),
                    new_source: current,
                    destination: format!(
                        "Old Context Files/{} - {}/{} - {}",
                        matching::safe_name(&family),
                        old.document_id,
                        old.id,
                        old.name
                    ),
                    temporary: format!("Old Context Files/.unused-{}", old.id),
                    mode: "move".into(),
                    old_identity: old_mark["identity"].as_str().map(str::to_owned),
                });
            }
            let mut existing = Vec::new();
            for op in &operations {
                if cloud::s(op, "state") != "done" {
                    existing.push(
                        serde_json::from_str::<ArchivePlan>(cloud::s(op, "payload"))
                            .map_err(|_| "Invalid archive journal.")?,
                    );
                }
            }
            for plan in archives.iter().chain(existing.iter()) {
                let old = self.source(&plan.old_source)?;
                let new = self.source(&plan.new_source)?;
                let old_version = cloud::version(&self.db, &plan.old_version)?;
                let new_version = cloud::version(&self.db, &plan.new_version)?;
                let old_snap = fingerprint(&mut facts, &self.store, &old_version.snapshot);
                let new_snap = fingerprint(&mut facts, &self.store, &new_version.snapshot);
                let source = fingerprint(&mut facts, root, &old.relative);
                let replacement = fingerprint(&mut facts, root, &new.relative);
                let destination = fingerprint(&mut facts, root, &plan.destination);
                let problem = if old_snap["hash"] != old_version.hash
                    || new_snap["hash"] != new_version.hash
                {
                    Some("A required snapshot is missing, changed or unreadable.".into())
                } else if plan.mode == "move" && source["hash"] != old_version.hash {
                    Some("The older source changed, is missing or cannot be read; it will not be moved until verified.".into())
                } else if plan.mode == "move" && replacement["hash"] != new_version.hash {
                    Some(
                        "The current source changed or cannot be read; capture/review it first."
                            .into(),
                    )
                } else if destination["missing"] != true {
                    Some("The archive destination already exists or cannot be checked. No overwrite is allowed.".into())
                } else {
                    None
                };
                items.push(ReviewItem {
                    from: if plan.mode == "move" {
                        old.relative
                    } else {
                        format!("Saved snapshot: {}", old_version.id)
                    },
                    to: plan.destination.clone(),
                    action: if plan.mode == "move" {
                        "archive"
                    } else {
                        "copy"
                    }
                    .into(),
                    problem,
                });
            }
        } else {
            let mut copies = 0;
            for op in &operations {
                let archive: ArchivePlan = serde_json::from_str(cloud::s(op, "payload"))
                    .map_err(|_| "Invalid archive journal.")?;
                if cloud::s(op, "state") != "done" {
                    cancellations.push(cloud::s(op, "id").into());
                }
                if archive.mode != "move" {
                    copies += 1;
                    continue;
                }
                if archive.old_identity.is_none() {
                    if cloud::s(op, "state") == "done" {
                        warnings.push("An older archive has no move receipt; it cannot be restored automatically.".into());
                    }
                    continue;
                }
                let old = self.source(&archive.old_source)?;
                let version = cloud::version(&self.db, &archive.old_version)?;
                let snapshot = fingerprint(&mut facts, &self.store, &version.snapshot);
                let source = fingerprint(&mut facts, root, &archive.destination);
                let destination = fingerprint(&mut facts, root, &old.relative);
                let already = destination["hash"] == version.hash
                    && destination["identity"].as_str() == archive.old_identity.as_deref();
                let problem = if snapshot["hash"] != version.hash {
                    Some("The saved snapshot is missing, changed or unreadable.".into())
                } else if !already && destination["missing"] != true {
                    Some("The original location already contains a file or cannot be checked. It will not be overwritten.".into())
                } else if !already
                    && (source["hash"] != version.hash
                        || source["identity"].as_str() != archive.old_identity.as_deref())
                {
                    Some("The archived source is missing, changed, locked or has an unverified identity.".into())
                } else {
                    None
                };
                items.push(ReviewItem {
                    from: archive.destination.clone(),
                    to: old.relative.clone(),
                    action: "restore".into(),
                    problem,
                });
                restores.push(RestorePlan {
                    archive_operation: cloud::s(op, "id").into(),
                    archive,
                    original: old.relative,
                });
            }
            if copies > 0 {
                warnings.push(format!("{copies} snapshot-copy archive(s) stay in Old Context Files. They do not replace or overwrite current working files."));
            }
        }
        let flags = self.tracking_flags(project)?;
        let revision: i64 = self
            .db
            .query_row(
                "SELECT folder_revision FROM projects WHERE id=?1",
                [project],
                |r| r.get(0),
            )
            .map_err(error)?;
        let path_revision = crate::path_selection::revision(&self.db, project)?;
        let token = format!(
            "{:x}",
            Sha256::digest(encode(
                &json!({"project":project,"root":root_string,"folder":revision,"paths":path_revision,"flags":flags,"enabled":enabled,"items":items,"warnings":warnings,"archives":archives,"restores":restores,"cancel":cancellations,"facts":facts})
            )?)
        );
        Ok(Prepared {
            review: Review {
                token,
                enabled,
                items,
                warnings,
            },
            archives,
            restores,
            cancellations,
        })
    }
    pub fn archival_review(&mut self, project: &str, enabled: bool) -> Result<Review> {
        self.refresh(project)?;
        Ok(self.prepare_archival(project, enabled)?.review)
    }
    pub(crate) fn require_no_restores(&self, project: &str) -> Result<()> {
        let pending:bool=self.db.query_row("SELECT EXISTS(SELECT 1 FROM operations WHERE project_id=?1 AND kind='restore' AND state NOT IN ('done','cancelled','restored'))",[project],|r|r.get(0)).map_err(error)?;
        if pending {
            return Err("Finish the pending file restores before enabling archival again. Resolve the reported conflict and Retry pending operations.".into());
        }
        Ok(())
    }
    fn queue_archival(tx: &Transaction<'_>, project: &str, plan: &Prepared) -> Result<()> {
        for id in &plan.cancellations {
            tx.execute("UPDATE operations SET state='cancelled',error=NULL,updated_at=?1 WHERE id=?2 AND state!='done'",params![models::now(),id]).map_err(error)?;
        }
        for archive in &plan.archives {
            let id = uuid::Uuid::new_v4().to_string();
            tx.execute("INSERT INTO operations(id,project_id,document_id,dedupe_key,kind,payload,created_at,updated_at) VALUES(?1,?2,?3,?4,'archive',?5,?6,?6)",params![id,project,archive.document,format!("organize:{}:{id}",archive.old_source),encode(archive)?,models::now()]).map_err(error)?;
        }
        for restore in &plan.restores {
            tx.execute("INSERT OR IGNORE INTO operations(id,project_id,document_id,dedupe_key,kind,payload,created_at,updated_at) VALUES(?1,?2,?3,?4,'restore',?5,?6,?6)",params![uuid::Uuid::new_v4().to_string(),project,restore.archive.document,format!("restore:{}",restore.archive_operation),encode(restore)?,models::now()]).map_err(error)?;
        }
        tx.execute("INSERT INTO archive_intents VALUES(?1,?2,0) ON CONFLICT(project_id) DO UPDATE SET enabled=excluded.enabled,pending=0",params![project,plan.review.enabled]).map_err(error)?;
        Ok(())
    }
    pub fn apply_archival(&mut self, project: &str, enabled: bool, token: &str) -> Result<()> {
        self.refresh(project)?;
        if enabled && !self.tracking_flags(project)?.0 {
            return Err("Resume tracking before organizing older sources.".into());
        }
        let prepared = self.prepare_archival(project, enabled)?;
        if prepared.review.token != token {
            return Err("The archival review changed. Reload the review before moving files; nothing was applied.".into());
        }
        let tx = self.db.transaction().map_err(error)?;
        tx.execute(
            "UPDATE projects SET archive_enabled=?1 WHERE id=?2",
            params![enabled, project],
        )
        .map_err(error)?;
        Self::queue_archival(&tx, project, &prepared)?;
        let saved: bool = tx
            .query_row(
                "SELECT archive_enabled FROM projects WHERE id=?1",
                [project],
                |r| r.get(0),
            )
            .map_err(error)?;
        if saved != enabled {
            return Err("Archival setting verification failed; no moves were started.".into());
        }
        tx.commit().map_err(error)?;
        let errors = self.recover_operations(project, true)?;
        self.set_tracking_error(project, &errors)?;
        self.activity(project,"settings",if enabled{"Archival enabled; reviewed older exports organized where verification succeeded."}else{"Archival off; moved sources restored where verification succeeded. Snapshot copies retained."})
    }
    pub(crate) fn finish_archival_intent(&mut self, project: &str) -> Result<Vec<String>> {
        let intent: Option<bool> = self
            .db
            .query_row(
                "SELECT enabled FROM archive_intents WHERE project_id=?1 AND pending=1",
                [project],
                |r| r.get(0),
            )
            .optional()
            .map_err(error)?;
        let Some(enabled) = intent else {
            return Ok(vec![]);
        };
        let (tracking, archive, _) = self.tracking_flags(project)?;
        if !tracking || archive != enabled {
            return Ok(vec![]);
        }
        if enabled {
            let imports:bool=self.db.query_row("SELECT EXISTS(SELECT 1 FROM operations WHERE project_id=?1 AND kind IN ('capture','initial_import') AND state!='done')",[project],|r|r.get(0)).map_err(error)?;
            if imports {
                return Ok(vec![]);
            }
        }
        let plan = self.prepare_archival(project, enabled)?;
        let tx = self.db.transaction().map_err(error)?;
        Self::queue_archival(&tx, project, &plan)?;
        tx.commit().map_err(error)?;
        self.recover_operations(project, true)
    }
    pub(crate) fn finish_restore(&mut self, id: &str, plan: &RestorePlan) -> Result<()> {
        let p = &plan.archive;
        if self.tracking_flags(&p.project)?.1 {
            return Err("Restoration waits until archival is off.".into());
        }
        if p.mode != "move" || p.old_identity.is_none() {
            return Err("This archive is not a verified source move.".into());
        }
        let source = self.source(&p.old_source)?;
        if source.project_id != p.project || source.relative != plan.original {
            return Err("The original source association changed; restore requires review.".into());
        }
        let version = cloud::version(&self.db, &p.old_version)?;
        let snapshot = storage::checked(&self.store, &version.snapshot, false)?;
        storage::verify(&snapshot.path, &version.hash)?;
        let root_string = self.path(&p.project)?;
        let root = Path::new(&root_string);
        let destination = storage::checked(root, &plan.original, true)?;
        if destination.path.try_exists().map_err(|e| e.to_string())? {
            let mut file = storage::locked_read(&destination.path, false)?;
            if storage::hash(&mut file)? != version.hash
                || Some(storage::file_identity(&file)?).as_ref() != p.old_identity.as_ref()
            {
                return Err(format!("Restore conflict: {} already exists and is not the recorded moved source. Neither file was overwritten or deleted.",plan.original));
            }
        } else {
            let archived = storage::checked(root, &p.destination, false)?;
            let mut file = storage::locked_read(&archived.path, true)
                .map_err(|e| format!("Cannot restore {}: {e}", plan.original))?;
            if storage::hash(&mut file)? != version.hash
                || Some(storage::file_identity(&file)?).as_ref() != p.old_identity.as_ref()
            {
                return Err(
                    "Archived bytes or file identity changed. The archive was left untouched."
                        .into(),
                );
            }
            storage::rename_verified(&file, &destination.path)?;
        }
        self.db
            .execute(
                "UPDATE operations SET state='finalized',error=NULL,updated_at=?1 WHERE id=?2",
                params![models::now(), id],
            )
            .map_err(error)?;
        let mut file = storage::locked_read(&destination.path, false)?;
        if storage::hash(&mut file)? != version.hash
            || Some(storage::file_identity(&file)?).as_ref() != p.old_identity.as_ref()
        {
            return Err("Restored source changed before metadata verification. It was not overwritten; resolve the conflict before retrying.".into());
        }
        let metadata = file.metadata().map_err(|e| e.to_string())?;
        let tx = self.db.transaction().map_err(error)?;
        tx.execute("UPDATE document_versions SET archive_relative=NULL WHERE id=?1 AND archive_relative=?2",params![p.old_version,p.destination]).map_err(error)?;
        tx.execute("UPDATE source_files SET status='available',disposition=CASE WHEN id=(SELECT active_source_id FROM documents WHERE id=source_files.document_id) THEN 'tracked' ELSE 'superseded' END,last_hash=?1,hashed_size=?2,hashed_stamp=?3 WHERE id=?4",params![version.hash,metadata.len() as i64,storage::stamp(&metadata),source.id]).map_err(error)?;
        tx.execute(
            "UPDATE operations SET state='restored',error=NULL,updated_at=?1 WHERE id=?2",
            params![models::now(), plan.archive_operation],
        )
        .map_err(error)?;
        tx.execute(
            "UPDATE operations SET state='done',error=NULL,updated_at=?1 WHERE id=?2",
            params![models::now(), id],
        )
        .map_err(error)?;
        tx.commit().map_err(error)?;
        self.activity(
            &p.project,
            "restored",
            &format!(
                "Restored moved source to {}. Saved history and current-source selection retained.",
                plan.original
            ),
        )
    }
}
