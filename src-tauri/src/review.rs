//! Review uses the same saved path rules as the folder tree. No filesystem mutations.
use crate::{
    cloud::{self, n, s},
    path_selection::{self, Rule},
    registry::{error, Registry, Result},
};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingFile {
    pub project_id: String,
    pub project_name: String,
    pub path: String,
    pub kind: String,
    pub size: i64,
    pub status: String,
    pub modified: Option<String>,
}
pub fn pending(db: &Connection) -> Result<Vec<PendingFile>> {
    let rows=cloud::rows(db,"SELECT p.id project_id,p.name project_name,s.relative_path path,s.kind,s.size,s.status,s.modified_at modified FROM new_paths n JOIN projects p ON p.id=n.project_id JOIN source_files s ON s.project_id=n.project_id AND lower(replace(s.relative_path,char(92),'/'))=n.path WHERE p.removed_at IS NULL AND s.status!='archived' ORDER BY p.name,s.relative_path",&[])?;
    let mut entries = vec![];
    let mut folder_rules = std::collections::HashMap::<String, Vec<Rule>>::new();
    for row in rows {
        let project = s(&row, "project_id");
        if !folder_rules.contains_key(project) {
            folder_rules.insert(
                project.into(),
                path_selection::rules(db, project)?
                    .into_iter()
                    .filter(|r| r.kind == "folder")
                    .collect::<Vec<_>>(),
            );
        }
        if path_selection::permits(&folder_rules[project], s(&row, "path")) {
            entries.push(PendingFile {
                project_id: project.into(),
                project_name: s(&row, "project_name").into(),
                path: s(&row, "path").replace('\\', "/"),
                kind: s(&row, "kind").into(),
                size: n(&row, "size"),
                status: s(&row, "status").into(),
                modified: row["modified"].as_str().map(String::from),
            });
        }
    }
    Ok(entries)
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewRequest {
    pub project: String,
    pub paths: Vec<String>,
    pub action: String,
}
pub fn preview(r: &Registry, request: &ReviewRequest) -> Result<Value> {
    if !["approve", "ignore"].contains(&request.action.as_str())
        || request.paths.is_empty()
        || request.paths.len() > 1000
    {
        return Err("Choose between 1 and 1000 pending files to approve or ignore.".into());
    }
    let rows = pending(&r.db)?
        .into_iter()
        .filter(|e| e.project_id == request.project)
        .collect::<Vec<_>>();
    let mut paths = request
        .paths
        .iter()
        .map(|p| path_selection::normalized(p))
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    for path in &paths {
        let row = rows
            .iter()
            .find(|e| path_selection::normalized(&e.path) == *path)
            .ok_or("The pending files changed. Reload the inbox and review again.")?;
        if request.action == "approve" && row.status != "available" {
            return Err("Restore missing or inaccessible files before approving them.".into());
        }
    }
    let project=cloud::rows(&r.db,"SELECT name,folder_revision,folder_state,tracking_enabled FROM projects WHERE id=?1 AND removed_at IS NULL",&[&request.project])?.pop().ok_or("Project is no longer registered.")?;
    crate::folder_recovery::require_ready(&r.db, &request.project)?;
    let revision = path_selection::revision(&r.db, &request.project)?;
    let policy = cloud::project(&r.db, &request.project)?;
    let token = format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&json!([
                request.action,
                paths,
                rows,
                project,
                revision,
                policy["revision"]
            ]))
            .map_err(|e| e.to_string())?
        )
    );
    Ok(
        json!({"token":token,"project":request.project,"projectName":project["name"],"paths":paths,"action":request.action,"revision":revision,"trackingEnabled":project["tracking_enabled"],"cloudEnabled":policy["enabled"],"automatic":policy["auto_sync"]}),
    )
}
pub fn apply(r: &mut Registry, request: &ReviewRequest, token: &str) -> Result<()> {
    let plan = preview(r, request)?;
    if s(&plan, "token") != token {
        return Err(
            "Files, folder selection or project settings changed. Reload and confirm a new review."
                .into(),
        );
    }
    let paths = plan["paths"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    let mut rules = path_selection::rules(&r.db, &request.project)?;
    if request.action == "approve" {
        rules.retain(|r| r.kind != "file" || !paths.contains(&r.path));
    } else {
        for path in &paths {
            if !rules.iter().any(|r| r.kind == "file" && r.path == *path) {
                rules.push(Rule {
                    path: path.clone(),
                    kind: "file".into(),
                });
            }
        }
    }
    r.save_selection_review(
        &request.project,
        n(&plan, "revision"),
        rules,
        Some((&request.action, &paths)),
    )?;
    // Selection is already committed. Capture failures must not be reported as an
    // unsaved approval, and remain actionable in the tracking view.
    if request.action == "approve" {
        if let Err(e) = r.reconcile(
            &request.project,
            "review_approval",
            &std::collections::HashSet::new(),
            true,
        ) {
            let _ = r.set_tracking_error(&request.project, &[e]);
        }
    }
    Ok(())
}
pub fn preference(db: &Connection, key: &str, default: bool) -> bool {
    db.query_row("SELECT value FROM preferences WHERE key=?1", [key], |r| {
        r.get::<_, String>(0)
    })
    .map(|v| v == "true")
    .unwrap_or(default)
}
pub fn set_preference(db: &Connection, key: &str, value: bool) -> Result<()> {
    db.execute("INSERT INTO preferences(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![key,value.to_string()]).map_err(error)?;
    if preference(db, key, !value) != value {
        return Err("Desktop setting could not be saved and verified.".into());
    }
    Ok(())
}
// Signature changes accumulate for 30 seconds. Sending is acknowledged only after
// Windows accepts the toast, and its watermark survives a process restart.
pub fn notice_due(
    db: &Connection,
    key: &str,
    signature: &str,
    now: i64,
    delay: i64,
) -> Result<bool> {
    db.execute("INSERT INTO desktop_notices(key,signature,changed_at) VALUES(?1,?2,?3) ON CONFLICT(key) DO UPDATE SET signature=excluded.signature,changed_at=CASE WHEN desktop_notices.signature=desktop_notices.last_signature AND desktop_notices.signature!=excluded.signature THEN excluded.changed_at ELSE desktop_notices.changed_at END",params![key,signature,now]).map_err(error)?;
    db.query_row("SELECT signature!=last_signature AND ?2-changed_at>=?3 AND (last_sent=0 OR ?2-last_sent>=300) FROM desktop_notices WHERE key=?1",params![key,now,delay],|r|r.get(0)).map_err(error)
}
pub fn notice_sent(db: &Connection, key: &str, now: i64) -> Result<()> {
    db.execute(
        "UPDATE desktop_notices SET last_signature=signature,last_sent=?2 WHERE key=?1",
        params![key, now],
    )
    .map_err(error)?;
    Ok(())
}
