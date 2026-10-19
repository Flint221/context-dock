use std::path::Path;
use rusqlite::{params, Connection};
use crate::{models::{self, Document, Project, Snapshot}, scanner};
type Result<T> = std::result::Result<T, String>;
fn error(e: rusqlite::Error) -> String { format!("Local database operation failed: {e}") }
pub struct Registry { pub db: Connection }
impl Registry {
 pub fn open(path: &Path) -> Result<Self> {
  let mut db = Connection::open(path).map_err(error)?;
  db.busy_timeout(std::time::Duration::from_secs(10)).map_err(error)?;
  db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;").map_err(error)?;
  let version: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0)).map_err(error)?;
  if version > 1 { return Err("This database requires a newer ContextDock version.".into()); }
  if version == 0 { let tx = db.transaction().map_err(error)?; tx.execute_batch(include_str!("../migrations/001_registry.sql")).map_err(error)?; tx.pragma_update(None, "user_version", 1).map_err(error)?; tx.commit().map_err(error)?; }
  Ok(Self { db })
 }
 pub fn add(&mut self, name: &str, path: &str) -> Result<String> {
  let name = valid_name(name)?; let path = scanner::canonical(path)?; let id = uuid::Uuid::new_v4().to_string();
  let exists: bool = self.db.query_row("SELECT EXISTS(SELECT 1 FROM projects WHERE path_key=?1)", [models::key(&path)], |r| r.get(0)).map_err(error)?;
  if exists { return Err("This folder is already registered.".into()); }
  self.db.execute("INSERT INTO projects(id,name,path,path_key,created_at) VALUES(?1,?2,?3,?4,?5)", params![id,name,path,models::key(&path),models::now()]).map_err(error)?;
  self.refresh(&id)?; Ok(id)
 }
 pub fn path(&self, id: &str) -> Result<String> { self.db.query_row("SELECT path FROM projects WHERE id=?1", [id], |r| r.get(0)).map_err(|_| "Project registration was not found.".into()) }
 pub fn rename(&self, id: &str, name: &str) -> Result<()> { let name = valid_name(name)?; self.path(id)?; self.db.execute("UPDATE projects SET name=?1 WHERE id=?2", params![name,id]).map_err(error)?; Ok(()) }
 pub fn remove(&self, id: &str) -> Result<()> { self.db.execute("DELETE FROM projects WHERE id=?1", [id]).map_err(error)?; Ok(()) }
 pub fn refresh(&mut self, id: &str) -> Result<()> {
  let path = self.path(id)?; let now = scanner::observed_at();
  let scan = scanner::scan(Path::new(&path));
  let tx = self.db.transaction().map_err(error)?;
  match scan {
   Err(message) => { tx.execute("UPDATE projects SET status='unavailable',last_scan=?1,scan_error=?2 WHERE id=?3", params![now,message,id]).map_err(error)?; tx.execute("UPDATE documents SET status='unavailable' WHERE project_id=?1",[id]).map_err(error)?; }
   Ok(scan) => {
    let complete = scan.errors.is_empty();
    // Only a complete traversal proves a previously observed file is missing.
    tx.execute("UPDATE documents SET status=?1 WHERE project_id=?2",params![if complete { "missing" } else { "unavailable" },id]).map_err(error)?;
    for f in scan.files { tx.execute("INSERT INTO documents(id,project_id,name,relative_path,path_key,kind,size,modified_at,last_observed,status) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,'available') ON CONFLICT(project_id,path_key) DO UPDATE SET name=excluded.name,relative_path=excluded.relative_path,kind=excluded.kind,size=excluded.size,modified_at=excluded.modified_at,last_observed=excluded.last_observed,status='available'", params![uuid::Uuid::new_v4().to_string(),id,f.name,f.relative,f.key,f.kind,f.size,f.modified,now]).map_err(error)?; }
    let message = if complete { None } else { Some(format!("Some locations could not be read ({} errors). {}",scan.errors.len(),scan.errors[0])) };
    tx.execute("UPDATE projects SET status=?1,last_scan=?2,scan_error=?3 WHERE id=?4",params![if complete { "available" } else { "partial" },now,message,id]).map_err(error)?;
   }
  }
  tx.commit().map_err(error)?; Ok(())
 }
 pub fn theme(&self, theme: &str) -> Result<()> { if !["system","light","dark"].contains(&theme) { return Err("Invalid theme.".into()) } self.db.execute("INSERT INTO preferences(key,value) VALUES('theme',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[theme]).map_err(error)?; Ok(()) }
 pub fn snapshot(&self) -> Result<Snapshot> {
  let mut q = self.db.prepare("SELECT p.id,p.name,p.path,p.created_at,p.status,p.last_scan,p.scan_error,(SELECT COUNT(*) FROM documents d WHERE d.project_id=p.id),(SELECT MAX(modified_at) FROM documents d WHERE d.project_id=p.id AND d.status='available') FROM projects p ORDER BY p.name COLLATE NOCASE").map_err(error)?;
  let projects = q.query_map([], |r| Ok(Project { id:r.get(0)?,name:r.get(1)?,path:r.get(2)?,created_at:r.get(3)?,status:r.get(4)?,last_scan:r.get(5)?,scan_error:r.get(6)?,document_count:r.get(7)?,latest_modified:r.get(8)? })).map_err(error)?.collect::<std::result::Result<Vec<_>,_>>().map_err(error)?;
  let mut q = self.db.prepare("SELECT d.id,d.project_id,d.name,d.relative_path,p.path,d.kind,d.size,d.modified_at,d.last_observed,d.status FROM documents d JOIN projects p ON p.id=d.project_id ORDER BY d.relative_path COLLATE NOCASE").map_err(error)?;
  let documents = q.query_map([], |r| { let root:String=r.get(4)?; let relative:String=r.get(3)?; Ok(Document { id:r.get(0)?,project_id:r.get(1)?,name:r.get(2)?,absolute_path:Path::new(&root).join(&relative).to_string_lossy().into(),relative_path:relative,kind:r.get(5)?,size:r.get(6)?,modified_at:r.get(7)?,last_observed:r.get(8)?,status:r.get(9)? }) }).map_err(error)?.collect::<std::result::Result<Vec<_>,_>>().map_err(error)?;
  let theme = self.db.query_row("SELECT value FROM preferences WHERE key='theme'", [], |r| r.get(0)).unwrap_or_else(|_| "system".into());
  Ok(Snapshot { projects, documents, theme })
 }
}
fn valid_name(name: &str) -> Result<&str> { let n=name.trim(); if n.is_empty() || n.chars().count()>120 { Err("Project name must contain 1–120 characters.".into()) } else { Ok(n) } }
