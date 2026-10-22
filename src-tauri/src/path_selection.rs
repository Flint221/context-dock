//! Saved path exclusions are an overlay: discovery and immutable history remain intact.
use crate::{
    cloud, models,
    registry::{error, Registry, Result},
    scanner,
};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    path::Path,
};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub path: String,
    pub kind: String,
}
#[derive(Serialize)]
pub struct Entry {
    pub path: String,
    pub kind: String,
    pub status: String,
    pub new: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub revision: i64,
    pub exclusions: Vec<Rule>,
    pub entries: Vec<Entry>,
    pub warnings: Vec<String>,
    pub cloud_pending: usize,
}

pub fn normalized(path: &str) -> String {
    path.replace('\\', "/").to_lowercase()
}
pub fn permits(rules: &[Rule], path: &str) -> bool {
    let path = normalized(path);
    !rules.iter().any(|r| {
        path == r.path
            || (r.kind == "folder"
                && (r.path.is_empty() || path.starts_with(&format!("{}/", r.path))))
    })
}
pub fn rules(db: &Connection, project: &str) -> Result<Vec<Rule>> {
    let mut q = db
        .prepare("SELECT path,kind FROM path_exclusions WHERE project_id=?1 ORDER BY path,kind")
        .map_err(error)?;
    let result = q
        .query_map([project], |r| {
            Ok(Rule {
                path: r.get(0)?,
                kind: r.get(1)?,
            })
        })
        .map_err(error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(error);
    result
}
pub fn allowed(db: &Connection, project: &str, path: &str) -> Result<bool> {
    Ok(permits(&rules(db, project)?, path))
}
pub fn revision(db: &Connection, project: &str) -> Result<i64> {
    db.query_row(
        "SELECT COALESCE((SELECT revision FROM path_selection WHERE project_id=?1),0)",
        [project],
        |r| r.get(0),
    )
    .map_err(error)
}
pub fn blocked_documents(db: &Connection, project: &str) -> Result<HashSet<String>> {
    let rules = rules(db, project)?;
    let mut blocked = HashSet::new();
    for d in cloud::rows(db,"SELECT d.id,s.relative_path FROM documents d JOIN source_files s ON s.id=d.active_source_id WHERE d.project_id=?1",&[&project])? {
        if !permits(&rules,cloud::s(&d,"relative_path")) {blocked.insert(cloud::s(&d,"id").to_string());}
    }
    Ok(blocked)
}
pub fn blocked_versions(db: &Connection, project: &str) -> Result<HashSet<String>> {
    let rules = rules(db, project)?;
    let mut blocked = HashSet::new();
    if rules.is_empty() {
        return Ok(blocked);
    }
    let mut q=db.prepare("SELECT v.id,v.source_relative,s.relative_path FROM document_versions v JOIN documents d ON d.id=v.document_id LEFT JOIN source_files s ON s.id=d.active_source_id WHERE d.project_id=?1").map_err(error)?;
    let rows = q
        .query_map([project], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(error)?;
    for row in rows {
        let (id, path, current) = row.map_err(error)?;
        if !permits(&rules, &path) || current.is_some_and(|p| !permits(&rules, &p)) {
            blocked.insert(id);
        }
    }
    Ok(blocked)
}
fn validate(input: Vec<Rule>) -> Result<Vec<Rule>> {
    if input.len() > 10_000 {
        return Err("Select at most 10,000 path exclusions.".into());
    }
    let mut result = Vec::new();
    for rule in input {
        let path = normalized(&rule.path);
        if !["file", "folder"].contains(&rule.kind.as_str())
            || path.len() > 4096
            || path.contains(':')
            || path.contains('\0')
            || path.starts_with('/')
            || (path.is_empty() && rule.kind != "folder")
            || (!path.is_empty()
                && path.split('/').any(|p| {
                    p.is_empty()
                        || p == "."
                        || p == ".."
                        || p.ends_with([' ', '.'])
                        || p.chars().any(|c| c.is_control() || "*?<>|\"".contains(c))
                }))
        {
            return Err("Choose valid paths relative to this project; absolute paths and traversal are not allowed.".into());
        }
        result.push(Rule {
            path,
            kind: rule.kind,
        });
    }
    result.sort();
    result.dedup();
    if result.windows(2).any(|pair| pair[0].path == pair[1].path) {
        return Err("A path cannot be both a file and folder.".into());
    }
    Ok(result)
}
impl Registry {
    pub fn path_selection(&mut self, project: &str) -> Result<View> {
        let root = self.path(project)?;
        crate::folder_recovery::require_ready(&self.db, project)?;
        self.refresh(project)?;
        let exclusions = rules(&self.db, project)?;
        let (folders, warnings) = scanner::folders(Path::new(&root))?;
        let mut entries = BTreeMap::new();
        for path in folders {
            entries.insert(
                normalized(&path),
                Entry {
                    path,
                    kind: "folder".into(),
                    status: "available".into(),
                    new: false,
                },
            );
        }
        let sources=cloud::rows(&self.db,"SELECT s.relative_path,s.status,EXISTS(SELECT 1 FROM new_paths n WHERE n.project_id=s.project_id AND n.path=lower(replace(s.relative_path,char(92),'/'))) is_new FROM source_files s WHERE s.project_id=?1 AND s.status!='archived'",&[&project])?;
        for s in sources {
            let path = cloud::s(&s, "relative_path").replace('\\', "/");
            entries.insert(
                normalized(&path),
                Entry {
                    path,
                    kind: "file".into(),
                    status: cloud::s(&s, "status").into(),
                    new: cloud::n(&s, "is_new") == 1,
                },
            );
        }
        for rule in &exclusions {
            entries.entry(rule.path.clone()).or_insert_with(|| Entry {
                path: rule.path.clone(),
                kind: rule.kind.clone(),
                status: "missing".into(),
                new: false,
            });
        }
        Ok(View {
            revision: revision(&self.db, project)?,
            exclusions,
            entries: entries.into_values().collect(),
            warnings,
            cloud_pending: cloud::selection_removals(&self.db, project)?.len(),
        })
    }
    pub fn save_path_selection(
        &mut self,
        project: &str,
        expected: i64,
        input: Vec<Rule>,
    ) -> Result<()> {
        self.path(project)?;
        crate::folder_recovery::require_ready(&self.db, project)?;
        let next = validate(input)?;
        let previous = rules(&self.db, project)?;
        let tx = self.db.transaction().map_err(error)?;
        if revision(&tx, project)? != expected {
            return Err(
                "Folder selection changed elsewhere. Reload and review your changes before saving."
                    .into(),
            );
        }
        let p = cloud::project(&tx, project)?;
        if ["cleanup_pending", "cleanup_failed"].contains(&cloud::s(&p, "status"))
            || cloud::n(&p, "delete_requested") == 1
        {
            return Err("Finish cloud cleanup before changing folder selection.".into());
        }
        tx.execute("DELETE FROM path_exclusions WHERE project_id=?1", [project])
            .map_err(error)?;
        for r in &next {
            tx.execute(
                "INSERT INTO path_exclusions VALUES(?1,?2,?3)",
                params![project, r.path, r.kind],
            )
            .map_err(error)?;
        }
        tx.execute("INSERT INTO path_selection VALUES(?1,1) ON CONFLICT(project_id) DO UPDATE SET revision=revision+1",[project]).map_err(error)?;
        if rules(&tx, project)? != next || revision(&tx, project)? != expected + 1 {
            return Err("Folder selection could not be verified. No changes saved.".into());
        }
        let sources = cloud::rows(
            &tx,
            "SELECT id,document_id,relative_path,disposition FROM source_files WHERE project_id=?1",
            &[&project],
        )?;
        for source in sources {
            let path = cloud::s(&source, "relative_path");
            if !permits(&next, path) {
                tx.execute(
                    "DELETE FROM pending_matches WHERE source_id=?1",
                    [cloud::s(&source, "id")],
                )
                .map_err(error)?;
            } else if !permits(&previous, path) && cloud::s(&source, "disposition") == "ignored" {
                tx.execute("UPDATE source_files SET disposition=CASE WHEN last_hash IS NULL THEN 'discovered' ELSE 'tracked' END WHERE id=?1",[cloud::s(&source,"id")]).map_err(error)?;
                tx.execute("UPDATE documents SET tracking_state=CASE WHEN current_version_id IS NULL THEN 'discovered' ELSE 'tracked' END WHERE id=?1",[cloud::s(&source,"document_id")]).map_err(error)?;
            }
        }
        let mut pending = tx
            .prepare("SELECT path FROM new_paths WHERE project_id=?1")
            .map_err(error)?;
        let paths = pending
            .query_map([project], |r| r.get::<_, String>(0))
            .map_err(error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(error)?;
        drop(pending);
        for path in paths {
            if permits(&next, &path) {
                tx.execute(
                    "DELETE FROM new_paths WHERE project_id=?1 AND path=?2",
                    params![project, path],
                )
                .map_err(error)?;
            }
        }
        let blocked = blocked_versions(&tx, project)?;
        for j in cloud::jobs(&tx, project)? {
            if blocked.contains(cloud::s(&j, "version_id")) {
                tx.execute("UPDATE cloud_jobs SET desired=0,phase=CASE WHEN file_id IS NOT NULL AND state NOT IN ('removed','purged') THEN 'path_remove' ELSE phase END,state=CASE WHEN file_id IS NOT NULL AND state NOT IN ('removed','purged') THEN 'removing' ELSE state END,next_at=0,error=NULL WHERE id=?1",[cloud::s(&j,"id")]).map_err(error)?;
            }
        }
        tx.execute("UPDATE cloud_projects SET revision=revision+1,sync_requested=CASE WHEN enabled=1 AND started=1 THEN 1 ELSE sync_requested END,next_at=0,error=NULL,attempts=0 WHERE project_id=?1",[project]).map_err(error)?;
        for job in cloud::jobs(&tx, project)? {
            if blocked.contains(cloud::s(&job, "version_id"))
                && (cloud::n(&job, "desired") != 0
                    || (!cloud::s(&job, "file_id").is_empty()
                        && !["removed", "purged"].contains(&cloud::s(&job, "state"))
                        && (cloud::s(&job, "phase") != "path_remove"
                            || cloud::s(&job, "state") != "removing")))
            {
                return Err(
                    "Cloud exclusion intent could not be verified. No folder changes saved.".into(),
                );
            }
        }
        if cloud::n(&cloud::project(&tx, project)?, "revision") != cloud::n(&p, "revision") + 1 {
            return Err("Cloud selection revision could not be verified. No changes saved.".into());
        }
        tx.execute("INSERT INTO tracking_activity(id,project_id,kind,message,created_at) VALUES(?1,?2,'path_selection','Folder/file selection saved. Excluded paths stop capture and leave the cloud index; local history is retained.',?3)",params![uuid::Uuid::new_v4().to_string(),project,models::now()]).map_err(error)?;
        tx.commit().map_err(error)
    }
}
