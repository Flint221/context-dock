use crate::{
    comparison_models::*,
    extraction,
    registry::{error, Registry, Result},
    storage,
};
use rusqlite::OptionalExtension;
use sha2::{Digest, Sha256};
use similar::{Algorithm, ChangeTag, TextDiff};
use std::{
    io::Read,
    path::PathBuf,
    time::{Duration, Instant},
};
pub struct ComparisonJob {
    pub store: PathBuf,
    pub name: String,
    pub kind: String,
    pub earlier: ComparedVersion,
    pub later: ComparedVersion,
}
impl Registry {
    /// Metadata only. No snapshot I/O while the registry mutex is held.
    pub fn comparison_job(&self, left: &str, right: &str) -> Result<ComparisonJob> {
        let find = |id: &str| -> Result<ComparedVersion> {
            self.db.query_row("SELECT id,document_id,number,source_name,captured_at,sha256,snapshot_relative FROM document_versions WHERE id=?1", [id], |r| Ok(ComparedVersion {
                id:r.get(0)?, document_id:r.get(1)?, number:r.get(2)?, source_name:r.get(3)?, captured_at:r.get(4)?, sha256:r.get(5)?, relative:r.get(6)?,
            })).optional().map_err(error)?.ok_or("Captured version was not found. Reopen version history.".into())
        };
        let (mut earlier, mut later) = (find(left)?, find(right)?);
        if earlier.document_id != later.document_id {
            return Err("Only versions of the same logical document can be compared.".into());
        }
        if earlier.number > later.number {
            std::mem::swap(&mut earlier, &mut later);
        }
        let (name, kind) = self
            .db
            .query_row(
                "SELECT family_name,kind FROM documents WHERE id=?1",
                [&earlier.document_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(error)?;
        Ok(ComparisonJob {
            store: self.store.clone(),
            name,
            kind,
            earlier,
            later,
        })
    }
}
fn verified_bytes(store: &std::path::Path, v: &ComparedVersion) -> Result<Vec<u8>> {
    let checked = storage::checked(store, &v.relative, false)
        .map_err(|e| format!("v{} snapshot unavailable or unsafe: {e}", v.number))?;
    let file = storage::locked_read(&checked.path, false)
        .map_err(|e| format!("v{} snapshot missing, locked or unreadable: {e}", v.number))?;
    if file.metadata().map_err(|e| e.to_string())?.len() > extraction::MAX_SNAPSHOT as u64 {
        return Err(format!(
            "v{} snapshot exceeds the 16 MiB comparison limit.",
            v.number
        ));
    }
    // Hash exactly the bounded buffer used by extraction, while retaining guarded directories.
    let mut bytes = vec![];
    file.take(extraction::MAX_SNAPSHOT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Cannot read v{} snapshot: {e}", v.number))?;
    if bytes.len() > extraction::MAX_SNAPSHOT {
        return Err("Snapshot grew beyond the 16 MiB comparison limit.".into());
    }
    if format!("{:x}", Sha256::digest(&bytes)) != v.sha256 {
        return Err(format!(
            "v{} snapshot SHA-256 integrity mismatch. Comparison stopped; no files were changed.",
            v.number
        ));
    }
    Ok(bytes)
}
pub fn compare(job: ComparisonJob) -> Result<Comparison> {
    let left = verified_bytes(&job.store, &job.earlier)?;
    let right = verified_bytes(&job.store, &job.later)?;
    let a = extraction::extract(&left, &job.kind)
        .map_err(|e| format!("v{}: {e}", job.earlier.number))?;
    let b = extraction::extract(&right, &job.kind)
        .map_err(|e| format!("v{}: {e}", job.later.number))?;
    let mut warnings: Vec<String> = a
        .warnings
        .into_iter()
        .map(|s| format!("Earlier: {s}"))
        .collect();
    warnings.extend(b.warnings.into_iter().map(|s| format!("Later: {s}")));
    let (blocks, added_lines, removed_lines, changed_blocks) = diff(&a.text, &b.text)?;
    Ok(Comparison {
        format_version: 1,
        document_id: job.earlier.document_id.clone(),
        document_name: job.name,
        kind: job.kind,
        bytes_identical: left == right,
        text_equivalent: a.text == b.text,
        earlier: job.earlier,
        later: job.later,
        added_lines,
        removed_lines,
        changed_blocks,
        warnings,
        blocks,
    })
}
pub(crate) fn diff(a: &str, b: &str) -> Result<(Vec<DiffBlock>, usize, usize, usize)> {
    for text in [a, b] {
        if text.len() > extraction::MAX_TEXT || text.split_inclusive('\n').count() > 20_000 {
            return Err(
                "Comparison supports at most 4 MiB of text and 20,000 lines per version.".into(),
            );
        }
        if text.split_inclusive('\n').any(|s| s.len() > 65536) {
            return Err("A line exceeds the 64 KiB comparison limit. Split very long paragraphs before capturing future versions.".into());
        }
    }
    let start = Instant::now();
    let budget = Duration::from_secs(2);
    let d = TextDiff::configure()
        .algorithm(Algorithm::Myers)
        .timeout(budget)
        .diff_lines(a, b);
    // similar may return an approximate script at its deadline. Never present that as exact.
    if start.elapsed() >= budget {
        return Err(
            "Comparison exceeds the 2-second diff complexity limit. Select a closer version pair."
                .into(),
        );
    }
    let mut blocks: Vec<DiffBlock> = vec![];
    let (mut added, mut removed, mut changed) = (0, 0, 0);
    let (mut old, mut new) = (1, 1);
    for c in d.iter_all_changes() {
        let equal = c.tag() == ChangeTag::Equal;
        if blocks
            .last()
            .is_none_or(|last| (last.kind == "equal") != equal)
        {
            if !equal {
                changed += 1;
            }
            blocks.push(DiffBlock {
                id: format!("block-{old}-{new}"),
                kind: if equal { "equal" } else { "change" }.into(),
                old_start: old,
                old_count: 0,
                new_start: new,
                new_count: 0,
                lines: vec![],
            });
        }
        let block = blocks
            .last_mut()
            .ok_or("Diff block could not be created.")?;
        let (old_line, new_line, kind) = match c.tag() {
            ChangeTag::Equal => {
                block.old_count += 1;
                block.new_count += 1;
                let p = (Some(old), Some(new), "equal");
                old += 1;
                new += 1;
                p
            }
            ChangeTag::Delete => {
                removed += 1;
                block.old_count += 1;
                let p = (Some(old), None, "remove");
                old += 1;
                p
            }
            ChangeTag::Insert => {
                added += 1;
                block.new_count += 1;
                let p = (None, Some(new), "add");
                new += 1;
                p
            }
        };
        let value = c.value();
        block.lines.push(DiffLine {
            kind: kind.into(),
            old_line,
            new_line,
            text: value.strip_suffix('\n').unwrap_or(value).to_owned(),
            terminated: value.ends_with('\n'),
        });
    }
    Ok((blocks, added, removed, changed))
}
