use serde::Serialize;
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComparedVersion {
    pub id: String,
    pub document_id: String,
    pub number: i64,
    pub source_name: String,
    pub captured_at: String,
    pub sha256: String,
    #[serde(skip)]
    pub relative: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    pub kind: String,
    pub old_line: Option<usize>,
    pub new_line: Option<usize>,
    pub text: String,
    pub terminated: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffBlock {
    pub id: String,
    pub kind: String,
    pub old_start: usize,
    pub old_count: usize,
    pub new_start: usize,
    pub new_count: usize,
    pub lines: Vec<DiffLine>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Comparison {
    pub format_version: u8,
    pub document_id: String,
    pub document_name: String,
    pub kind: String,
    pub earlier: ComparedVersion,
    pub later: ComparedVersion,
    pub bytes_identical: bool,
    pub text_equivalent: bool,
    pub added_lines: usize,
    pub removed_lines: usize,
    pub changed_blocks: usize,
    pub warnings: Vec<String>,
    pub blocks: Vec<DiffBlock>,
}
