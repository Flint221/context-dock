use crate::{
    models::{self, Document, Project, Snapshot},
    scanner,
};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
pub type Result<T> = std::result::Result<T, String>;
pub fn error(e: rusqlite::Error) -> String {
    format!("Local database operation failed: {e}")
}
pub struct Registry {
    pub db: Connection,
    pub store: PathBuf,
}
impl Registry {
    pub fn open(path: &Path) -> Result<Self> {
        let mut db = Connection::open(path).map_err(error)?;
        db.busy_timeout(std::time::Duration::from_secs(10))
            .map_err(error)?;
        db.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;",
        )
        .map_err(error)?;
        let version: i64 = db
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(error)?;
        if version > 11 {
            return Err("This database requires a newer ContextDock version.".into());
        }
        if version < 11 {
            db.pragma_update(None, "foreign_keys", false)
                .map_err(error)?;
        }
        let tx = db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(error)?;
        let version: i64 = tx
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(error)?;
        if version > 11 {
            return Err("This database requires a newer ContextDock version.".into());
        }
        if version < 1 {
            tx.execute_batch(include_str!("../migrations/001_registry.sql"))
                .map_err(error)?;
        }
        if version < 2 {
            tx.execute_batch(include_str!("../migrations/002_tracking.sql"))
                .map_err(error)?;
        }
        if version < 3 {
            tx.execute_batch(include_str!("../migrations/003_cloud_indexing.sql"))
                .map_err(error)?;
            tx.execute(
                "INSERT INTO cloud_installation(id) VALUES(?1)",
                [uuid::Uuid::new_v4().to_string()],
            )
            .map_err(error)?;
        }
        if version < 4 {
            tx.execute_batch(include_str!("../migrations/004_ai_analysis.sql"))
                .map_err(error)?;
        }
        if version < 5 {
            tx.execute_batch(include_str!("../migrations/005_project_chats.sql"))
                .map_err(error)?;
        }
        if version < 6 {
            tx.execute_batch(include_str!("../migrations/006_folder_recovery.sql"))
                .map_err(error)?;
        }
        if version < 7 {
            tx.execute_batch(include_str!("../migrations/007_path_selection.sql"))
                .map_err(error)?;
        }
        if version < 8 {
            tx.execute_batch(include_str!("../migrations/008_ordered_import.sql"))
                .map_err(error)?;
        }
        if version < 9 {
            tx.execute_batch(include_str!("../migrations/009_reversible_archival.sql"))
                .map_err(error)?;
        }
        if version < 10 {
            tx.execute_batch(include_str!("../migrations/010_unified_selection.sql"))
                .map_err(error)?;
        }
        if version < 11 {
            tx.execute_batch(include_str!("../migrations/011_save_revisions.sql"))
                .map_err(error)?;
        }
        let invalid: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check)",
                [],
                |r| r.get(0),
            )
            .map_err(error)?;
        if invalid {
            return Err("Registry migration found invalid history references. No migration changes were committed.".into());
        }
        tx.pragma_update(None, "user_version", 11).map_err(error)?;
        tx.commit().map_err(error)?;
        db.pragma_update(None, "foreign_keys", true)
            .map_err(error)?;
        let store = std::fs::canonicalize(path.parent().unwrap_or(Path::new(".")))
            .map_err(|e| format!("Application storage cannot be resolved: {e}"))?;
        Ok(Self { db, store })
    }
    pub fn add(&mut self, name: &str, path: &str) -> Result<String> {
        let name = valid_name(name)?;
        let path = scanner::canonical(path)?;
        let key = models::key(&path);
        let existing: Option<(String, Option<String>)> = self
            .db
            .query_row(
                "SELECT id,removed_at FROM projects WHERE path_key=?1",
                [&key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(error)?;
        let id = if let Some((id, removed)) = existing {
            if removed.is_none() {
                return Err("This folder is already registered.".into());
            }
            self.db.execute("UPDATE projects SET removed_at=NULL,name=?1,path=?2,tracking_enabled=0,archive_enabled=0,watch_status='off' WHERE id=?3",params![name,path,id]).map_err(error)?;
            id
        } else {
            let id = uuid::Uuid::new_v4().to_string();
            self.db
                .execute(
                    "INSERT INTO projects(id,name,path,path_key,created_at) VALUES(?1,?2,?3,?4,?5)",
                    params![id, name, path, key, models::now()],
                )
                .map_err(error)?;
            id
        };
        self.refresh(&id)?;
        Ok(id)
    }
    pub fn path(&self, id: &str) -> Result<String> {
        self.db
            .query_row(
                "SELECT path FROM projects WHERE id=?1 AND removed_at IS NULL",
                [id],
                |r| r.get(0),
            )
            .map_err(|_| "Project registration was not found.".into())
    }
    pub fn rename(&self, id: &str, name: &str) -> Result<()> {
        let name = valid_name(name)?;
        self.path(id)?;
        self.db
            .execute("UPDATE projects SET name=?1 WHERE id=?2", params![name, id])
            .map_err(error)?;
        Ok(())
    }
    #[cfg(test)]
    pub fn remove(&self, id: &str) -> Result<()> {
        self.remove_with_cloud_choice(id, true)
    }
    pub fn remove_with_cloud_choice(&self, id: &str, retain_cloud: bool) -> Result<()> {
        // Tombstone the registration: captured history and bytes can be recovered by registering the same root again.
        let tx = self.db.unchecked_transaction().map_err(error)?;
        let p = crate::cloud::project(&tx, id)?;
        if !retain_cloud && crate::cloud_cleanup::has_remote(&tx, &p)? {
            return Err("Delete and verify this project's cloud data first, or explicitly choose to retain it.".into());
        }
        tx.execute("UPDATE cloud_projects SET enabled=0,paused=1,sync_requested=0,delete_requested=0,auto_sync=0,status='removed_registration',revision=revision+1 WHERE project_id=?1",[id]).map_err(error)?;
        tx.execute("UPDATE projects SET removed_at=?1,tracking_enabled=0,archive_enabled=0,watch_status='off' WHERE id=?2",params![models::now(),id]).map_err(error)?;
        let removed: i64=tx.query_row("SELECT COUNT(*) FROM projects WHERE id=?1 AND removed_at IS NOT NULL AND tracking_enabled=0",[id],|r|r.get(0)).map_err(error)?;
        if removed != 1 {
            return Err("Project removal could not be verified.".into());
        }
        tx.commit().map_err(error)?;
        Ok(())
    }
    pub fn refresh(&mut self, id: &str) -> Result<()> {
        let path = self.path(id)?;
        if crate::folder_recovery::require_ready(&self.db, id).is_err() {
            return Ok(());
        }
        let now = models::now();
        let scan = scanner::scan(Path::new(&path));
        let tx = self.db.transaction().map_err(error)?;
        match scan {
            Err(message) => {
                tx.execute("UPDATE projects SET folder_state='needs_reassignment',folder_revision=folder_revision+1,status='unavailable',last_scan=?1,scan_error=?2,watch_status='needs_folder' WHERE id=?3",params![now,format!("{} {message}",crate::folder_recovery::LOST),id]).map_err(error)?;
                tx.execute("UPDATE source_files SET status='unavailable' WHERE project_id=?1 AND status!='archived'",[id]).map_err(error)?;
            }
            Ok(scan) => {
                let complete = scan.errors.is_empty();
                let previously_scanned: bool = tx
                    .query_row(
                        "SELECT last_scan IS NOT NULL FROM projects WHERE id=?1",
                        [id],
                        |r| r.get(0),
                    )
                    .map_err(error)?;
                let mut selection_changed = false;
                tx.execute(
                    "UPDATE source_files SET status=?1 WHERE project_id=?2 AND status!='archived'",
                    params![if complete { "missing" } else { "unavailable" }, id],
                )
                .map_err(error)?;
                for f in scan.files {
                    let existing: Option<String> = tx
                        .query_row(
                            "SELECT id FROM source_files WHERE project_id=?1 AND path_key=?2",
                            params![id, f.key],
                            |r| r.get(0),
                        )
                        .optional()
                        .map_err(error)?;
                    if let Some(sid) = existing {
                        tx.execute("UPDATE source_files SET name=?1,relative_path=?2,kind=?3,size=?4,modified_at=?5,last_observed=?6,status='available',disposition=CASE WHEN disposition='archived' THEN 'superseded' ELSE disposition END WHERE id=?7",params![f.name,f.relative,f.kind,f.size,f.modified,now,sid]).map_err(error)?;
                    } else {
                        let did = uuid::Uuid::new_v4().to_string();
                        let sid = uuid::Uuid::new_v4().to_string();
                        let name = crate::matching::stem(&f.name).display;
                        tx.execute("INSERT INTO documents(id,project_id,family_name,kind,active_source_id) VALUES(?1,?2,?3,?4,?5)",params![did,id,name,f.kind,sid]).map_err(error)?;
                        tx.execute("INSERT INTO source_files(id,project_id,document_id,name,relative_path,path_key,kind,size,modified_at,last_observed,status) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,'available')",params![sid,id,did,f.name,f.relative,f.key,f.kind,f.size,f.modified,now]).map_err(error)?;
                        if previously_scanned {
                            let path = crate::path_selection::normalized(&f.relative);
                            tx.execute("INSERT OR IGNORE INTO path_exclusions(project_id,path,kind) VALUES(?1,?2,'file')", params![id,path]).map_err(error)?;
                            tx.execute(
                                "INSERT OR IGNORE INTO new_paths(project_id,path) VALUES(?1,?2)",
                                params![id, path],
                            )
                            .map_err(error)?;
                            selection_changed = true;
                        }
                    }
                }
                if selection_changed {
                    tx.execute("INSERT INTO path_selection VALUES(?1,1) ON CONFLICT(project_id) DO UPDATE SET revision=revision+1", [id]).map_err(error)?;
                }
                let message = if complete {
                    None
                } else {
                    Some(format!(
                        "Some locations could not be read ({} errors). {}",
                        scan.errors.len(),
                        scan.errors[0]
                    ))
                };
                tx.execute(
                    "UPDATE projects SET status=?1,last_scan=?2,scan_error=?3 WHERE id=?4",
                    params![
                        if complete { "available" } else { "partial" },
                        now,
                        message,
                        id
                    ],
                )
                .map_err(error)?;
            }
        }
        tx.commit().map_err(error)?;
        Ok(())
    }
    pub fn theme(&self, theme: &str) -> Result<()> {
        if !["system", "light", "dark"].contains(&theme) {
            return Err("Invalid theme.".into());
        }
        self.db.execute("INSERT INTO preferences(key,value) VALUES('theme',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[theme]).map_err(error)?;
        Ok(())
    }
    pub fn snapshot(&self) -> Result<Snapshot> {
        self.check_folders()?;
        let mut q=self.db.prepare("SELECT p.id,p.name,p.path,p.created_at,p.status,p.last_scan,p.scan_error,(SELECT COUNT(*) FROM source_files s WHERE s.project_id=p.id AND s.status!='archived'),(SELECT MAX(modified_at) FROM source_files s WHERE s.project_id=p.id AND s.status='available'),p.tracking_enabled,p.archive_enabled,(SELECT COALESCE(MAX(a.rowid),0) FROM tracking_activity a WHERE a.project_id=p.id),p.folder_state FROM projects p WHERE p.removed_at IS NULL ORDER BY p.name COLLATE NOCASE").map_err(error)?;
        let projects = q
            .query_map([], |r| {
                Ok(Project {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    path: r.get(2)?,
                    created_at: r.get(3)?,
                    status: r.get(4)?,
                    last_scan: r.get(5)?,
                    scan_error: r.get(6)?,
                    document_count: r.get(7)?,
                    latest_modified: r.get(8)?,
                    tracking_enabled: r.get(9)?,
                    archive_enabled: r.get(10)?,
                    tracking_revision: r.get(11)?,
                    folder_state: r.get(12)?,
                })
            })
            .map_err(error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(error)?;
        let mut q=self.db.prepare("SELECT s.id,s.project_id,s.name,s.relative_path,p.path,s.kind,s.size,s.modified_at,s.last_observed,s.status,s.document_id FROM source_files s JOIN projects p ON p.id=s.project_id WHERE p.removed_at IS NULL AND s.status!='archived' ORDER BY s.relative_path COLLATE NOCASE").map_err(error)?;
        let documents = q
            .query_map([], |r| {
                let root: String = r.get(4)?;
                let relative: String = r.get(3)?;
                Ok(Document {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    name: r.get(2)?,
                    absolute_path: Path::new(&root).join(&relative).to_string_lossy().into(),
                    relative_path: relative,
                    kind: r.get(5)?,
                    size: r.get(6)?,
                    modified_at: r.get(7)?,
                    last_observed: r.get(8)?,
                    status: r.get(9)?,
                    document_id: r.get(10)?,
                })
            })
            .map_err(error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(error)?;
        let theme = self
            .db
            .query_row("SELECT value FROM preferences WHERE key='theme'", [], |r| {
                r.get(0)
            })
            .unwrap_or_else(|_| "system".into());
        Ok(Snapshot {
            projects,
            documents,
            theme,
        })
    }
}
fn valid_name(name: &str) -> Result<&str> {
    let n = name.trim();
    if n.is_empty() || n.chars().count() > 120 {
        Err("Project name must contain 1–120 characters.".into())
    } else {
        Ok(n)
    }
}
