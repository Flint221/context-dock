use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub kind: String,
    pub versions: Vec<String>,
    pub baseline: Option<String>,
    pub model: String,
    pub input_budget: usize,
    pub output_tokens: usize,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Goals {
    pub purpose: String,
    pub goals: String,
    pub requirements: String,
    pub non_goals: String,
    pub constraints: String,
    pub deliverables: String,
    pub principles: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub document: String,
    pub version: String,
    pub number: i64,
    pub name: String,
    pub hash: String,
    pub captured_at: String,
    pub current: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    pub earlier: String,
    pub later: String,
    pub blocks: serde_json::Value,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    pub id: String,
    pub version: String,
    pub heading: String,
    pub start: usize,
    pub end: usize,
    pub text: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub project: String,
    pub request: Request,
    pub sources: Vec<Source>,
    pub baseline: Option<serde_json::Value>,
    pub evidence: Vec<Evidence>,
    pub transitions: Vec<Transition>,
    pub warnings: Vec<String>,
    pub scope: String,
    pub prompt_version: u8,
    pub schema_version: u8,
    pub input_units: usize,
    pub estimated_tokens: usize,
    pub estimate_usd: f64,
    pub conservative_usd: f64,
    pub pricing_date: String,
    pub cache_key: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Citation {
    pub evidence_id: String,
    pub quote: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Finding {
    pub category: String,
    pub title: String,
    pub detail: String,
    pub at_version: Option<String>,
    pub refs: Vec<Citation>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Report {
    pub summary: String,
    pub summary_refs: Vec<Citation>,
    pub findings: Vec<Finding>,
}
