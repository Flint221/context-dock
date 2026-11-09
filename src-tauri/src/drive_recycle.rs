//! Optional, journaled local cleanup after fresh remote and local verification.
use crate::{
    cloud,
    drive_api::DriveApi,
    drive_publish as publication, models,
    recycle_bin::Bin,
    registry::{error, Registry},
    storage,
};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
type Result<T> = std::result::Result<T, String>;

pub fn configure(db: &Connection, project: &str, value: bool) -> Result<()> {
    let tx = db.unchecked_transaction().map_err(error)?;
    let exists:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM drive_projects WHERE project_id=?1 AND (?2=0 OR enabled=1))",params![project,value],|r|r.get(0)).map_err(error)?;
    if !exists {
        return Err("Enable Drive publication before choosing automatic recycling.".into());
    }
    tx.execute("UPDATE drive_projects SET recycle=?2,requested=CASE WHEN ?2 THEN 1 ELSE requested END WHERE project_id=?1",params![project,value]).map_err(error)?;
    tx.execute("INSERT INTO drive_cleanup(project_id,error) VALUES(?1,NULL) ON CONFLICT(project_id) DO UPDATE SET error=NULL",[project]).map_err(error)?;
    if value {
        tx.execute(
            "UPDATE drive_documents SET state='queued' WHERE project_id=?1 AND state='synced'",
            [project],
        )
        .map_err(error)?;
        tx.execute(
            "UPDATE drive_schedule SET observed_signature=NULL,settle_until=0 WHERE project_id=?1",
            [project],
        )
        .map_err(error)?;
    }
    let saved: bool = tx
        .query_row(
            "SELECT recycle FROM drive_projects WHERE project_id=?1",
            [project],
            |r| r.get(0),
        )
        .map_err(error)?;
    if saved != value {
        return Err("The recycling preference did not save.".into());
    }
    tx.commit().map_err(error)
}
fn opted_in(db: &Connection, project: &str) -> Result<bool> {
    db.query_row(
        "SELECT recycle FROM drive_projects WHERE project_id=?1",
        [project],
        |r| r.get(0),
    )
    .optional()
    .map(|v| v.unwrap_or(false))
    .map_err(error)
}
fn check(
    r: &Registry,
    p: &str,
    a: &str,
    c: &publication::Current,
    api: &dyn DriveApi,
) -> Result<()> {
    api.checkpoint(true).map_err(|e| e.message)?;
    publication::require_enabled(&r.db, p, a).map_err(|e| e.message)?;
    if !opted_in(&r.db, p)? || crate::desktop::uploads_paused(&r.db) {
        return Err("Older-file recycling is paused. Files are retained.".into());
    }
    let now = publication::current(&r.db, p, &c.document).map_err(|e| e.message)?;
    if publication::generation(&now) != publication::generation(c) {
        return Err("The newest local generation changed. Older files are retained.".into());
    }
    Ok(())
}
pub fn after_publication(
    r: &Registry,
    p: &str,
    a: &str,
    doc: &str,
    parent: &str,
    api: &dyn DriveApi,
) {
    if !opted_in(&r.db, p).unwrap_or(false) {
        return;
    }
    let result = run(r, p, a, doc, parent, api, &crate::recycle_bin::NativeBin);
    if let Err(failure) = result {
        let _ = r.db.execute(
        "INSERT INTO drive_cleanup(project_id,error) VALUES(?1,?2) ON CONFLICT(project_id) DO UPDATE SET error=excluded.error",
        params![p, failure],
    );
    }
}
pub(crate) fn run(
    r: &Registry,
    p: &str,
    a: &str,
    doc: &str,
    parent: &str,
    api: &dyn DriveApi,
    bin: &dyn Bin,
) -> Result<usize> {
    if !opted_in(&r.db, p)? {
        return Ok(0);
    }
    let c = publication::verify_for_cleanup(r, p, a, doc, parent, api).map_err(|e| e.message)?;
    let Some(latest_order) = crate::matching::version_order(&c.name) else {
        return Ok(0);
    };
    check(r, p, a, &c, api)?;
    let latest_path = storage::checked(Path::new(&c.root), &c.relative, false)?;
    // Pin the newest file without write/delete sharing throughout cleanup.
    let mut latest = storage::locked_read(&latest_path.path, false)?;
    if storage::hash(&mut latest)? != c.hash {
        return Err("The newest local file changed. Older files stay in place.".into());
    }
    let mut count = 0;
    for source in r.source_ids(p)? {
        let old = r.source(&source)?;
        if old.document_id != doc
            || old.id == c.source
            || old.disposition != "superseded"
            || old.last_hash.is_none()
            || old.size != old.hashed_size.unwrap_or(-1)
            || old.modified != old.hashed_stamp
            || crate::matching::stem(&old.name).normalized
                != crate::matching::stem(&c.name).normalized
            || !crate::path_selection::allowed(&r.db, p, &old.relative)?
        {
            continue;
        }
        let Some(order) = crate::matching::version_order(&old.name) else {
            continue;
        };
        if order >= latest_order {
            continue;
        }
        let hash = old.last_hash.as_deref().unwrap();
        let attempted: bool =
            r.db.query_row(
                "SELECT EXISTS(SELECT 1 FROM drive_recycle WHERE source_id=?1 AND sha256=?2)",
                params![old.id, hash],
                |row| row.get(0),
            )
            .map_err(error)?;
        if attempted {
            continue;
        }
        check(r, p, a, &c, api)?;
        let original = storage::checked(Path::new(&c.root), &old.relative, false)?;
        bin.supported(&original.path)?;
        let mut file = storage::locked_read(&original.path, true)?;
        if storage::hash(&mut file)? != hash
            || file.metadata().map_err(|e| e.to_string())?.len() != old.size as u64
        {
            return Err(format!(
                "{} changed since it was tracked. It was kept in place.",
                old.name
            ));
        }
        let identity = storage::file_identity(&file)?;
        let id = uuid::Uuid::new_v4().to_string();
        let safety_relative = format!(".contextdock/recycle/{id}/{}", old.name);
        let safety = storage::checked(Path::new(&c.root), &safety_relative, true)?;
        // The intent is durable before any rename. A crash after the rename
        // can restore the exact same identity without guessing or overwriting.
        r.db.execute("INSERT INTO drive_recycle(id,project_id,document_id,source_id,root,relative,safety_relative,sha256,file_identity,generation,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![id,p,doc,old.id,c.root,old.relative,safety_relative,hash,identity,publication::generation(&c),models::now()]).map_err(error)?;
        {
            let tx = r.db.unchecked_transaction().map_err(error)?;
            // Acquire the write lock before checking the latest settings, so
            // an opt-out cannot be acknowledged before this guarded rename.
            tx.execute(
                "UPDATE drive_recycle SET state='prepared' WHERE id=?1",
                [&id],
            )
            .map_err(error)?;
            check(r, p, a, &c, api)?;
            storage::rename_verified(&file, &safety.path)?;
            tx.execute(
                "UPDATE drive_recycle SET state='quarantined' WHERE id=?1",
                [&id],
            )
            .map_err(error)?;
            tx.commit().map_err(error)?;
        }
        drop(file);
        let operation = (|| {
            check(r, p, a, &c, api)?;
            let mut pinned = storage::locked_read(&safety.path, false)?;
            if storage::file_identity(&pinned)? != identity || storage::hash(&mut pinned)? != hash {
                return Err("The safety copy changed. It was retained for review.".into());
            }
            drop(pinned);
            r.db.execute(
                "UPDATE drive_recycle SET state='bin_pending' WHERE id=?1",
                [&id],
            )
            .map_err(error)?;
            let receipt = bin.recycle(&safety.path, hash)?;
            let tx = r.db.unchecked_transaction().map_err(error)?;
            tx.execute(
                "UPDATE drive_recycle SET state='recycled',bin_receipt=?2,error=NULL WHERE id=?1",
                params![id, receipt],
            )
            .map_err(error)?;
            tx.execute("UPDATE source_files SET status='archived',disposition='recycled' WHERE id=?1 AND document_id=?2",params![old.id,doc]).map_err(error)?;
            tx.commit().map_err(error)?;
            Ok::<(), String>(())
        })();
        if let Err(failure) = operation {
            let restored = restore(r, &id);
            let message = if let Err(restore_error) = restored {
                format!("{failure} Recovery needs attention: {restore_error}")
            } else {
                failure
            };
            r.db.execute(
                "UPDATE drive_recycle SET error=?2 WHERE id=?1",
                params![id, message],
            )
            .map_err(error)?;
            return Err(message);
        }
        count += 1;
        r.activity(p,"recycled",&format!("{}: unchanged older version verified and sent to the Windows Recycle Bin. Newest local file kept.",old.name))?;
    }
    Ok(count)
}
fn restore(r: &Registry, id: &str) -> Result<()> {
    let row = cloud::rows(&r.db, "SELECT * FROM drive_recycle WHERE id=?1", &[&id])?
        .into_iter()
        .next()
        .ok_or("Recycling intent is missing.")?;
    let project = cloud::s(&row, "project_id");
    let root = cloud::s(&row, "root");
    if r.path(project)? != root {
        return Err(
            "The project root changed. The safety file is retained at the previous root.".into(),
        );
    }
    let original = storage::checked(Path::new(root), cloud::s(&row, "relative"), false)?;
    let safety = storage::checked(Path::new(root), cloud::s(&row, "safety_relative"), false)?;
    let identity = cloud::s(&row, "file_identity");
    let hash = cloud::s(&row, "sha256");
    if safety.path.exists() {
        let mut file = storage::locked_read(&safety.path, true)?;
        if storage::file_identity(&file)? != identity || storage::hash(&mut file)? != hash {
            return Err("The safety file no longer matches. It was not moved or deleted.".into());
        }
        if original.path.exists() {
            return Err("The original path is occupied. The safety file was retained without overwriting it.".into());
        }
        storage::rename_verified(&file, &original.path)?;
    } else {
        let mut file=storage::locked_read(&original.path,false).map_err(|_|"The operation's final location is uncertain. Inspect the Windows Recycle Bin; no deletion was retried.".to_string())?;
        if storage::file_identity(&file)? != identity || storage::hash(&mut file)? != hash {
            return Err(
                "The original path contains a different file. Nothing was overwritten.".into(),
            );
        }
    }
    let tx = r.db.unchecked_transaction().map_err(error)?;
    tx.execute(
        "UPDATE drive_recycle SET state='restored',error=NULL WHERE id=?1",
        [id],
    )
    .map_err(error)?;
    tx.execute("UPDATE source_files SET status='available',disposition='superseded' WHERE id=?1 AND document_id=?2",params![cloud::s(&row,"source_id"),cloud::s(&row,"document_id")]).map_err(error)?;
    tx.commit().map_err(error)
}
pub fn recover(r: &Registry) -> Result<()> {
    recover_selected(r, None, false)
}
fn recover_selected(r: &Registry, project: Option<&str>, attention: bool) -> Result<()> {
    for row in cloud::rows(&r.db,"SELECT id,project_id FROM drive_recycle WHERE (?1 IS NULL OR project_id=?1) AND (state IN ('prepared','quarantined','bin_pending') OR (?2 AND state='attention'))",&[&project,&attention])? {
        let id=cloud::s(&row,"id");
        if let Err(message)=restore(r,id) {
            r.db.execute("UPDATE drive_recycle SET state='attention',error=?2 WHERE id=?1",params![id,message]).map_err(error)?;
            r.db.execute("INSERT INTO drive_cleanup(project_id,error) VALUES(?1,?2) ON CONFLICT(project_id) DO UPDATE SET error=excluded.error",params![cloud::s(&row,"project_id"),message]).map_err(error)?;
        }
    }
    Ok(())
}
pub fn recover_project(r: &Registry, project: &str) -> Result<()> {
    r.path(project)?;
    recover_selected(r, Some(project), true)?;
    let pending:bool=r.db.query_row("SELECT EXISTS(SELECT 1 FROM drive_recycle WHERE project_id=?1 AND state IN ('prepared','quarantined','bin_pending','attention'))",[project],|r|r.get(0)).map_err(error)?;
    if pending {
        return Err("Some older-file recovery still needs attention. Available safety files were kept; occupied original paths were not overwritten.".into());
    }
    r.db.execute("INSERT INTO drive_cleanup(project_id,error) VALUES(?1,NULL) ON CONFLICT(project_id) DO UPDATE SET error=NULL",[project]).map_err(error)?;
    Ok(())
}
