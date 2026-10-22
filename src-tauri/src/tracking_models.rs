use serde::Serialize;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Family {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub state: String,
    pub active_source_id: Option<String>,
    pub active_filename: Option<String>,
    pub source_status: Option<String>,
    pub current_number: Option<i64>,
    pub version_count: i64,
    pub last_captured: Option<String>,
    pub needs_review: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceChoice {
    pub id: String,
    pub document_id: String,
    pub name: String,
    pub relative_path: String,
    pub status: String,
    pub disposition: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    pub source_id: String,
    pub name: String,
    pub relative_path: String,
    pub explanation: String,
    pub candidate_ids: Vec<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub id: String,
    pub kind: String,
    pub message: String,
    pub created_at: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingOperation {
    pub id: String,
    pub kind: String,
    pub state: String,
    pub error: Option<String>,
    pub created_at: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackingView {
    pub import_notice: Option<String>,
    pub enabled: bool,
    pub archive_enabled: bool,
    pub initialized: bool,
    pub watch_status: String,
    pub error: Option<String>,
    pub families: Vec<Family>,
    pub sources: Vec<SourceChoice>,
    pub reviews: Vec<Review>,
    pub activity: Vec<Activity>,
    pub operations: Vec<PendingOperation>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub id: String,
    pub number: i64,
    pub source_name: String,
    pub source_relative: String,
    pub captured_at: String,
    pub modified_at: Option<String>,
    pub size: i64,
    pub sha256: String,
    pub reason: String,
    pub snapshot_path: String,
    pub archive_path: Option<String>,
    pub snapshot_available: bool,
    pub archive_available: bool,
    pub current: bool,
}
