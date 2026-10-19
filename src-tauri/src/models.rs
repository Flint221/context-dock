use serde::Serialize;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project { pub id: String, pub name: String, pub path: String, pub created_at: String, pub status: String, pub last_scan: Option<String>, pub scan_error: Option<String>, pub document_count: i64, pub latest_modified: Option<String> }
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Document { pub id: String, pub project_id: String, pub name: String, pub relative_path: String, pub absolute_path: String, pub kind: String, pub size: i64, pub modified_at: Option<String>, pub last_observed: String, pub status: String }
#[derive(Serialize)]
pub struct Snapshot { pub projects: Vec<Project>, pub documents: Vec<Document>, pub theme: String }
pub fn now() -> String { chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true) }
pub fn key(path: &str) -> String { if cfg!(windows) { path.replace('/', "\\").to_lowercase() } else { path.to_string() } }
