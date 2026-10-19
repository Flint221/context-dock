use crate::{
    comparison::{compare, diff},
    extraction::{body_xml, extract},
    registry::Registry,
};
use std::{
    collections::HashSet,
    fs,
    io::{Cursor, Write},
    path::PathBuf,
};
use tempfile::{tempdir, TempDir};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};
fn xml(body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{body}</w:body></w:document>"#
    )
}
fn package(body: &str, method: CompressionMethod) -> Vec<u8> {
    let mut z = ZipWriter::new(Cursor::new(vec![]));
    z.start_file(
        "word/document.xml",
        SimpleFileOptions::default().compression_method(method),
    )
    .unwrap();
    z.write_all(body.as_bytes()).unwrap();
    z.finish().unwrap().into_inner()
}
#[test]
fn identical_empty_and_unicode_text() {
    for s in ["", "# Heading\n\n- café Ω 🐄\n", "no final newline"] {
        let (blocks, a, r, c) = diff(s, s).unwrap();
        assert_eq!((a, r, c), (0, 0, 0));
        assert!(blocks.iter().all(|b| b.kind == "equal"));
    }
    assert_eq!(diff("", "added\n").unwrap().1, 1);
    assert_eq!(diff("removed\n", "").unwrap().2, 1);
}
#[test]
fn additions_removals_number_mapping_and_multiple_blocks() {
    let (blocks, a, r, c) = diff("one\nold\nkeep\nend\n", "one\nnew\nextra\nkeep\n").unwrap();
    assert_eq!((a, r, c), (2, 2, 2));
    let changes: Vec<_> = blocks.iter().filter(|b| b.kind == "change").collect();
    assert_eq!(
        (
            changes[0].old_start,
            changes[0].old_count,
            changes[0].new_start,
            changes[0].new_count
        ),
        (2, 1, 2, 2)
    );
    assert_eq!(changes[0].lines[0].old_line, Some(2));
    assert_eq!(changes[0].lines[1].new_line, Some(2));
    assert_eq!((changes[1].old_start, changes[1].new_start), (4, 5));
    assert!(blocks
        .iter()
        .any(|b| b.kind == "equal" && b.lines.iter().any(|l| l.text == "keep")));
}
#[test]
fn newline_bom_whitespace_and_encoding_policy() {
    assert_eq!(
        extract(b"\xef\xbb\xbfhello\r\n\r\n", "markdown")
            .unwrap()
            .text,
        "hello\n\n"
    );
    assert!(extract(b"hello\r\n", "markdown").unwrap().warnings[0].contains("CRLF"));
    assert!(extract(b"\xff\xfeh\0", "markdown")
        .unwrap_err()
        .contains("UTF-8"));
    assert!(extract(b"h\0", "markdown").unwrap_err().contains("UTF-16"));
    assert_eq!(diff("a\n", "a").unwrap().3, 1);
    assert!(!diff("a \n", "a\n")
        .unwrap()
        .0
        .iter()
        .all(|b| b.kind == "equal"));
    let (blocks, _, _, _) = diff("a", "b\n").unwrap();
    assert!(!blocks[0].lines[0].terminated);
    assert!(blocks[0].lines[1].terminated);
}
#[test]
fn realistic_large_inputs_are_bounded_without_truncation() {
    let a = (0..10_000)
        .map(|i| format!("line {i}\n"))
        .collect::<String>();
    let b = a.replace("line 5000\n", "changed 5000\n");
    let (_, added, removed, changed) = diff(&a, &b).unwrap();
    assert_eq!((added, removed, changed), (1, 1, 1));
    assert!(diff(&"x\n".repeat(20_001), "")
        .unwrap_err()
        .contains("20,000"));
    assert!(diff(&"x".repeat(65537), "").unwrap_err().contains("64 KiB"));
}
#[test]
fn docx_runs_paragraphs_tables_unicode_and_formatting_only() {
    let plain = xml("<w:p><w:r><w:t>Hello Ω &amp; café</w:t></w:r></w:p><w:p><w:r><w:t>Next</w:t><w:br/><w:t>line</w:t></w:r></w:p>");
    let formatted = xml("<w:p><w:r><w:rPr><w:b/></w:rPr><w:t>Hello </w:t></w:r><w:r><w:t>Ω &amp; café</w:t></w:r></w:p><w:p><w:r><w:t>Next</w:t><w:br/><w:t>line</w:t></w:r></w:p>");
    let a = package(&plain, CompressionMethod::Stored);
    let b = package(&formatted, CompressionMethod::Deflated);
    assert_ne!(a, b);
    let extracted = extract(&a, "word").unwrap();
    assert_eq!(extracted.text, "Hello Ω & café\nNext\nline\n");
    assert_eq!(extracted.text, extract(&b, "word").unwrap().text);
    let table=xml("<w:tbl><w:tr><w:tc><w:p><w:r><w:t>A</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>B</w:t></w:r></w:p></w:tc></w:tr></w:tbl>");
    assert_eq!(body_xml(&table).unwrap(), "A\tB\n");
}
#[test]
fn docx_policy_accepts_insertions_skips_deletions_drawings_and_instructions() {
    let s=xml("<w:p><w:del><w:r><w:delText>gone</w:delText></w:r></w:del><w:ins><w:r><w:t>kept</w:t></w:r></w:ins><w:r><w:instrText>INSTRUCTION</w:instrText></w:r><w:hyperlink><w:r><w:t>link</w:t></w:r></w:hyperlink><w:drawing><w:txbxContent><w:p><w:r><w:t>box</w:t></w:r></w:p></w:txbxContent></w:drawing></w:p>");
    assert_eq!(body_xml(&s).unwrap(), "keptlink\n");
    assert_eq!(body_xml(&xml("")).unwrap(), "");
}
#[test]
fn docx_corruption_xml_and_decompression_guards() {
    assert!(extract(b"not a zip", "word").unwrap_err().contains("ZIP"));
    assert!(body_xml("<document/>")
        .unwrap_err()
        .contains("Word document"));
    assert!(body_xml(&xml("<w:p>")).is_err());
    assert!(body_xml(&xml("<w:tbl><w:tbl/></w:tbl>"))
        .unwrap_err()
        .contains("Nested"));
    assert!(
        body_xml(&format!("<!DOCTYPE x [<!ENTITY e 'bomb'>]>{}", xml("")))
            .unwrap_err()
            .contains("DTDs")
    );
    assert!(body_xml(&xml("<w:p><w:r><w:t>&unknown;</w:t></w:r></w:p>")).is_err());
    let huge = package(
        &xml(&"a".repeat(9 * 1024 * 1024)),
        CompressionMethod::Stored,
    );
    assert!(extract(&huge, "word").unwrap_err().contains("8 MiB"));
    let bomb = package(&xml(&"a".repeat(1024 * 1024)), CompressionMethod::Deflated);
    assert!(extract(&bomb, "word")
        .unwrap_err()
        .contains("compression-ratio"));
    let missing = {
        let z = ZipWriter::new(Cursor::new(vec![]));
        z.finish().unwrap().into_inner()
    };
    assert!(extract(&missing, "word")
        .unwrap_err()
        .contains("document.xml"));
}
struct Fixture {
    _temp: TempDir,
    root: PathBuf,
    r: Registry,
    project: String,
    doc: String,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempdir().unwrap();
        let root = temp.path().join("source");
        fs::create_dir(&root).unwrap();
        let store = temp.path().join("store");
        fs::create_dir(&store).unwrap();
        fs::write(root.join("Master.md"), "one\n").unwrap();
        fs::write(root.join("Other.md"), "other\n").unwrap();
        let mut r = Registry::open(&store.join("registry.db")).unwrap();
        let project = r.add("Test", root.to_str().unwrap()).unwrap();
        let ids = r.source_ids(&project).unwrap();
        r.configure_tracking(&project, true, false, ids).unwrap();
        let doc = r
            .snapshot()
            .unwrap()
            .documents
            .into_iter()
            .find(|d| d.name == "Master.md")
            .unwrap()
            .document_id;
        Self {
            _temp: temp,
            root,
            r,
            project,
            doc,
        }
    }
    fn edit(&mut self, text: &str) {
        fs::write(self.root.join("Master.md"), text).unwrap();
        self.r
            .reconcile(&self.project, "manual_refresh", &HashSet::new(), true)
            .unwrap();
    }
}
#[test]
fn historical_nonconsecutive_reverse_and_same_version_are_read_only() {
    let mut f = Fixture::new();
    f.edit("two\n");
    f.edit("three\n");
    let vs = f.r.versions(&f.doc).unwrap();
    let before = f.r.db.total_changes();
    let job = f.r.comparison_job(&vs[0].id, &vs[2].id).unwrap();
    // Current source can disappear: only immutable snapshots are used.
    fs::remove_file(f.root.join("Master.md")).unwrap();
    let result = compare(job).unwrap();
    assert_eq!((result.earlier.number, result.later.number), (1, 3));
    assert_eq!((result.added_lines, result.removed_lines), (1, 1));
    assert_eq!(f.r.db.total_changes(), before);
    assert_eq!(f.r.versions(&f.doc).unwrap().len(), 3);
    let same = compare(f.r.comparison_job(&vs[1].id, &vs[1].id).unwrap()).unwrap();
    assert!(same.bytes_identical && same.text_equivalent);
}
#[test]
fn byte_changes_and_normalized_text_equivalence_are_distinct() {
    let mut f = Fixture::new();
    f.edit("\u{feff}one\r\n");
    let vs = f.r.versions(&f.doc).unwrap();
    let result = compare(f.r.comparison_job(&vs[0].id, &vs[1].id).unwrap()).unwrap();
    assert!(!result.bytes_identical);
    assert!(result.text_equivalent);
    assert_eq!(result.changed_blocks, 0);
    assert!(!result.warnings.is_empty());
}
#[test]
fn versions_snapshot_identity_and_integrity_fail_closed() {
    let f = Fixture::new();
    let vs = f.r.versions(&f.doc).unwrap();
    let other: String =
        f.r.db
            .query_row(
                "SELECT id FROM document_versions WHERE document_id!=?1",
                [&f.doc],
                |r| r.get(0),
            )
            .unwrap();
    assert!(f
        .r
        .comparison_job(&vs[0].id, &other)
        .err()
        .unwrap()
        .contains("same logical"));
    assert!(f.r.comparison_job("missing", &vs[0].id).is_err());
    let mut job = f.r.comparison_job(&vs[0].id, &vs[0].id).unwrap();
    job.earlier.relative = "snapshots/missing.md".into();
    assert!(compare(job).err().unwrap().contains("missing"));
    let mut job = f.r.comparison_job(&vs[0].id, &vs[0].id).unwrap();
    job.earlier.sha256 = "0".repeat(64);
    assert!(compare(job).err().unwrap().contains("integrity mismatch"));
    let mut job = f.r.comparison_job(&vs[0].id, &vs[0].id).unwrap();
    job.earlier.relative = "../outside.md".into();
    assert!(compare(job).err().unwrap().contains("Unsafe relative"));
}
#[test]
fn manual_markdown_samples_have_documented_counts() {
    let guide = include_str!("../../docs/manual-testing-milestone-3.md").replace("\r\n", "\n");
    let samples: Vec<_> = guide
        .split("```markdown\n")
        .skip(1)
        .take(3)
        .map(|s| s.split("```").next().unwrap().replace("\r\n", "\n"))
        .collect();
    assert_eq!(samples.len(), 3);
    for (a, b, added, removed) in [(0, 1, 4, 2), (1, 2, 7, 2), (0, 2, 10, 3)] {
        let (_, actual_added, actual_removed, _) = diff(&samples[a], &samples[b]).unwrap();
        assert_eq!((actual_added, actual_removed), (added, removed));
    }
}
#[test]
fn zip_directory_and_xml_nesting_claims_are_bounded() {
    let mut bytes = package(&xml(""), CompressionMethod::Stored);
    let eocd = bytes.len() - 22;
    bytes[eocd + 8..eocd + 12].fill(0);
    assert!(extract(&bytes, "word").unwrap_err().contains("entry count"));
    let too_deep = xml(&format!("{}{}", "<w:p>".repeat(130), "</w:p>".repeat(130)));
    assert!(body_xml(&too_deep).unwrap_err().contains("128"));
    assert!(body_xml(&format!("{}corrupt trailing text", xml("")))
        .unwrap_err()
        .contains("outside"));
}
#[cfg(windows)]
#[test]
fn snapshot_locks_reparse_escape_and_oversize_are_rejected() {
    use std::os::windows::fs::OpenOptionsExt;
    let f = Fixture::new();
    let vs = f.r.versions(&f.doc).unwrap();
    let exclusive = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&vs[0].snapshot_path)
        .unwrap();
    assert!(compare(f.r.comparison_job(&vs[0].id, &vs[0].id).unwrap())
        .err()
        .unwrap()
        .contains("unreadable"));
    drop(exclusive);
    let outside = f.root.join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("file.md"), "one\n").unwrap();
    let link = f.r.store.join("escape");
    assert!(std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&link)
        .arg(&outside)
        .status()
        .unwrap()
        .success());
    let mut job = f.r.comparison_job(&vs[0].id, &vs[0].id).unwrap();
    job.earlier.relative = "escape/file.md".into();
    assert!(compare(job).err().unwrap().contains("reparse"));
    let large = f.r.store.join("large.md");
    let file = fs::File::create(large).unwrap();
    file.set_len(crate::extraction::MAX_SNAPSHOT as u64 + 1)
        .unwrap();
    drop(file);
    let mut job = f.r.comparison_job(&vs[0].id, &vs[0].id).unwrap();
    job.earlier.relative = "large.md".into();
    assert!(compare(job).err().unwrap().contains("16 MiB"));
}
