use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudDraft {
    pub enabled: bool,
    pub automatic: bool,
    pub mode: String,
    pub documents: Vec<String>,
    pub soft_limit: i64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudPlan {
    pub token: String,
    pub additions: usize,
    pub removals: usize,
    pub files: usize,
    pub bytes: i64,
    pub store_name: String,
}
#[derive(Clone)]
pub struct CloudVersion {
    pub id: String,
    pub document: String,
    pub number: i64,
    pub kind: String,
    pub hash: String,
    pub snapshot: String,
    pub size: i64,
    pub current: bool,
}
