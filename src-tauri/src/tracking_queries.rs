use crate::{
    registry::{error, Registry, Result},
    storage,
    tracking_models::*,
};
use rusqlite::{params, OptionalExtension};
use std::path::Path;
#[derive(Clone)]
pub struct Source {
    pub id: String,
    pub project_id: String,
    pub document_id: String,
    pub name: String,
    pub relative: String,
    pub kind: String,
    pub size: i64,
    pub modified: Option<String>,
    pub status: String,
    pub disposition: String,
    pub last_hash: Option<String>,
    pub hashed_stamp: Option<String>,
    pub hashed_size: Option<i64>,
}
impl Registry {
    pub fn source(&self, id: &str) -> Result<Source> {
        self.db.query_row("SELECT id,project_id,document_id,name,relative_path,kind,size,modified_at,status,disposition,last_hash,hashed_stamp,hashed_size FROM source_files WHERE id=?1",[id],|r|Ok(Source{id:r.get(0)?,project_id:r.get(1)?,document_id:r.get(2)?,name:r.get(3)?,relative:r.get(4)?,kind:r.get(5)?,size:r.get(6)?,modified:r.get(7)?,status:r.get(8)?,disposition:r.get(9)?,last_hash:r.get(10)?,hashed_stamp:r.get(11)?,hashed_size:r.get(12)?})).map_err(error)
    }
    pub fn source_ids(&self, project: &str) -> Result<Vec<String>> {
        let mut q=self.db.prepare("SELECT id FROM source_files WHERE project_id=?1 AND status='available' ORDER BY relative_path COLLATE NOCASE").map_err(error)?;
        let rows = q
            .query_map([project], |r| r.get(0))
            .map_err(error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(error)?;
        Ok(rows)
    }
    pub fn enabled_projects(&self) -> Result<Vec<(String, String)>> {
        let mut q = self
            .db
            .prepare("SELECT id,path FROM projects WHERE removed_at IS NULL AND tracking_enabled=1 AND folder_state='ready'")
            .map_err(error)?;
        let rows = q
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(error)?;
        Ok(rows)
    }
    pub fn tracking_flags(&self, project: &str) -> Result<(bool, bool, bool)> {
        self.path(project)?;
        self.db.query_row("SELECT tracking_enabled,archive_enabled,tracking_initialized FROM projects WHERE id=?1",[project],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(error)
    }
    pub fn activity(&self, project: &str, kind: &str, message: &str) -> Result<()> {
        let last:Option<(String,String)>=self.db.query_row("SELECT kind,message FROM tracking_activity WHERE project_id=?1 ORDER BY rowid DESC LIMIT 1",[project],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(error)?;
        if last
            .as_ref()
            .is_some_and(|(k, m)| k == kind && m == message)
        {
            return Ok(());
        }
        self.db.execute("INSERT INTO tracking_activity(id,project_id,kind,message,created_at) VALUES(?1,?2,?3,?4,?5)",params![uuid::Uuid::new_v4().to_string(),project,kind,message,crate::models::now()]).map_err(error)?;
        Ok(())
    }
    pub fn watch_status(&self, project: &str, status: &str, message: Option<&str>) -> Result<()> {
        self.db
            .execute(
                "UPDATE projects SET watch_status=?1 WHERE id=?2",
                params![status, project],
            )
            .map_err(error)?;
        if let Some(message) = message {
            self.activity(project, "watcher", message)?;
        }
        Ok(())
    }
    pub fn tracking_view(&self, project: &str) -> Result<TrackingView> {
        let (enabled, archive_enabled, initialized) = self.tracking_flags(project)?;
        let (watch_status, problem): (String, Option<String>) = self
            .db
            .query_row(
                "SELECT watch_status,tracking_error FROM projects WHERE id=?1",
                [project],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(error)?;
        let mut q=self.db.prepare("SELECT d.id,d.family_name,d.kind,d.tracking_state,d.active_source_id,COALESCE(s.name,(SELECT source_name FROM document_versions WHERE id=d.current_version_id)),s.status,v.number,(SELECT COUNT(*) FROM document_versions h WHERE h.document_id=d.id),(SELECT MAX(captured_at) FROM document_versions h WHERE h.document_id=d.id),EXISTS(SELECT 1 FROM pending_matches m JOIN source_files f ON f.id=m.source_id WHERE f.document_id=d.id) FROM documents d LEFT JOIN source_files s ON s.id=d.active_source_id LEFT JOIN document_versions v ON v.id=d.current_version_id WHERE d.project_id=?1 AND (d.merged_into IS NULL OR d.tracking_state='grouped_history') ORDER BY d.family_name COLLATE NOCASE").map_err(error)?;
        let families = q
            .query_map([project], |r| {
                Ok(Family {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    kind: r.get(2)?,
                    state: r.get(3)?,
                    active_source_id: r.get(4)?,
                    active_filename: r.get(5)?,
                    source_status: r.get(6)?,
                    current_number: r.get(7)?,
                    version_count: r.get(8)?,
                    last_captured: r.get(9)?,
                    needs_review: r.get(10)?,
                })
            })
            .map_err(error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(error)?;
        let mut q=self.db.prepare("SELECT id,document_id,name,relative_path,status,disposition FROM source_files WHERE project_id=?1 AND status!='archived' ORDER BY relative_path COLLATE NOCASE").map_err(error)?;
        let sources = q
            .query_map([project], |r| {
                Ok(SourceChoice {
                    id: r.get(0)?,
                    document_id: r.get(1)?,
                    name: r.get(2)?,
                    relative_path: r.get(3)?,
                    status: r.get(4)?,
                    disposition: r.get(5)?,
                })
            })
            .map_err(error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(error)?;
        let mut q=self.db.prepare("SELECT m.source_id,s.name,s.relative_path,m.explanation,m.candidate_ids FROM pending_matches m JOIN source_files s ON s.id=m.source_id WHERE s.project_id=?1 ORDER BY m.created_at").map_err(error)?;
        let reviews = q
            .query_map([project], |r| {
                let json: String = r.get(4)?;
                Ok(Review {
                    source_id: r.get(0)?,
                    name: r.get(1)?,
                    relative_path: r.get(2)?,
                    explanation: r.get(3)?,
                    candidate_ids: serde_json::from_str(&json).unwrap_or_default(),
                })
            })
            .map_err(error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(error)?;
        let mut q=self.db.prepare("SELECT id,kind,message,created_at FROM tracking_activity WHERE project_id=?1 ORDER BY rowid DESC LIMIT 50").map_err(error)?;
        let activity = q
            .query_map([project], |r| {
                Ok(Activity {
                    id: r.get(0)?,
                    kind: r.get(1)?,
                    message: r.get(2)?,
                    created_at: r.get(3)?,
                })
            })
            .map_err(error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(error)?;
        let mut q=self.db.prepare("SELECT id,kind,state,error,created_at FROM operations WHERE project_id=?1 AND state!='done' ORDER BY created_at").map_err(error)?;
        let operations = q
            .query_map([project], |r| {
                Ok(PendingOperation {
                    id: r.get(0)?,
                    kind: r.get(1)?,
                    state: r.get(2)?,
                    error: r.get(3)?,
                    created_at: r.get(4)?,
                })
            })
            .map_err(error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(error)?;
        let path_rules = crate::path_selection::rules(&self.db, project)?;
        Ok(TrackingView {
            import_notice: self
                .db
                .query_row(
                    "SELECT message FROM initial_import_notices WHERE project_id=?1",
                    [project],
                    |r| r.get(0),
                )
                .optional()
                .map_err(error)?,
            enabled,
            archive_enabled,
            initialized,
            watch_status,
            error: problem,
            families,
            sources: sources
                .into_iter()
                .filter(|s| crate::path_selection::permits(&path_rules, &s.relative_path))
                .collect(),
            reviews: reviews
                .into_iter()
                .filter(|s| crate::path_selection::permits(&path_rules, &s.relative_path))
                .collect(),
            activity,
            operations,
        })
    }
    pub fn versions(&self, document: &str) -> Result<Vec<Version>> {
        let (project, current): (String, Option<String>) = self
            .db
            .query_row(
                "SELECT project_id,current_version_id FROM documents WHERE id=?1",
                [document],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(error)?;
        let root = self.path(&project)?;
        let mut q=self.db.prepare("SELECT id,number,source_name,source_relative,captured_at,modified_at,size,sha256,reason,snapshot_relative,archive_relative FROM document_versions WHERE document_id=?1 ORDER BY number DESC").map_err(error)?;
        let rows = q
            .query_map([document], |r| {
                let id: String = r.get(0)?;
                let snapshot: String = r.get(9)?;
                let archive: Option<String> = r.get(10)?;
                let snapshot_path = self.store.join(snapshot);
                let archive_path = archive.map(|a| Path::new(&root).join(a));
                Ok(Version {
                    id: id.clone(),
                    number: r.get(1)?,
                    source_name: r.get(2)?,
                    source_relative: r.get(3)?,
                    captured_at: r.get(4)?,
                    modified_at: r.get(5)?,
                    size: r.get(6)?,
                    sha256: r.get(7)?,
                    reason: r.get(8)?,
                    snapshot_available: snapshot_path.is_file(),
                    archive_available: archive_path.as_ref().is_some_and(|p| p.is_file()),
                    snapshot_path: snapshot_path.to_string_lossy().into(),
                    archive_path: archive_path.map(|p| p.to_string_lossy().into()),
                    current: current.as_deref() == Some(&id),
                })
            })
            .map_err(error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(error)?;
        Ok(rows)
    }
    pub fn version_location(&self, version: &str, archive: bool) -> Result<String> {
        let(project,snapshot,relative,hash):(String,String,Option<String>,String)=self.db.query_row("SELECT d.project_id,v.snapshot_relative,v.archive_relative,v.sha256 FROM document_versions v JOIN documents d ON d.id=v.document_id WHERE v.id=?1",[version],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(error)?;
        let root = self.path(&project)?;
        let (base, relative) = if archive {
            (
                Path::new(&root),
                relative
                    .as_deref()
                    .ok_or("This version has no archive copy.")?,
            )
        } else {
            (self.store.as_path(), snapshot.as_str())
        };
        let checked = storage::checked(base, relative, false)?;
        storage::verify(&checked.path, &hash)?;
        Ok(checked.path.to_string_lossy().into())
    }
    pub fn current_location(&self, document: &str) -> Result<String> {
        let(project,relative):(String,String)=self.db.query_row("SELECT d.project_id,s.relative_path FROM documents d JOIN source_files s ON s.id=d.active_source_id WHERE d.id=?1",[document],|r|Ok((r.get(0)?,r.get(1)?))).map_err(error)?;
        let root = self.path(&project)?;
        crate::folder_recovery::require_ready(&self.db, &project)?;
        let checked = storage::checked(Path::new(&root), &relative, false)?;
        if !std::fs::metadata(&checked.path)
            .map_err(|e| e.to_string())?
            .is_file()
        {
            return Err("The active source file is unavailable.".into());
        }
        Ok(checked.path.to_string_lossy().into())
    }
}
