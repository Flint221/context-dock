//! Bounded local extraction. No package entry is written to disk or fetched.
use crate::registry::Result;
use quick_xml::{events::Event, name::ResolveResult, NsReader};
use std::io::{Cursor, Read};
pub const MAX_SNAPSHOT: usize = 16 * 1024 * 1024;
pub const MAX_TEXT: usize = 4 * 1024 * 1024;
const MAX_XML: usize = 8 * 1024 * 1024;
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const STRICT_W: &str = "http://purl.oclc.org/ooxml/wordprocessingml/main";
#[derive(Debug)]
pub struct Extracted {
    pub text: String,
    pub warnings: Vec<String>,
}
pub fn extract(bytes: &[u8], kind: &str) -> Result<Extracted> {
    let (original, mut warnings) = match kind {
        "markdown" => (utf8(bytes)?.to_owned(), vec![]),
        "word" => (docx(bytes)?, vec!["DOCX body text only: paragraphs/runs, hyperlinks and displayed field results; tables use tabs between cells. Insertions/move-to text included; deletions/move-from text excluded (accepted view). Headers, footers, notes, comments, images/charts, text boxes/drawings, embedded content and field instructions are excluded. Formatting, numbering, hidden-text visibility and layout are not compared; merged cells are read in XML order and nested tables are unsupported. No external resources are loaded.".into()]),
        _ => return Err("This document type does not support local comparison.".into()),
    };
    if original.len() > MAX_TEXT {
        return Err("Extracted text exceeds the 4 MiB comparison limit.".into());
    }
    if original.starts_with('\u{feff}') {
        warnings.push("A UTF-8 BOM was excluded from textual comparison.".into());
    }
    if original.contains("\r\n") {
        warnings.push("CRLF line endings were normalized to LF for comparison.".into());
    }
    let text = original
        .strip_prefix('\u{feff}')
        .unwrap_or(&original)
        .replace("\r\n", "\n");
    if text.contains('\0') {
        return Err("NUL characters are unsupported; save text as UTF-8 (not UTF-16).".into());
    }
    Ok(Extracted { text, warnings })
}
fn utf8(bytes: &[u8]) -> Result<&str> {
    std::str::from_utf8(bytes)
        .map_err(|_| "Unsupported text encoding: comparison requires valid UTF-8.".into())
}
// Bound central-directory allocations before letting the ZIP parser build its index.
fn zip_preflight(bytes: &[u8]) -> Result<()> {
    if bytes.len() > MAX_SNAPSHOT {
        return Err("DOCX package exceeds the 16 MiB limit.".into());
    }
    let start = bytes.len().saturating_sub(65557);
    let pos = (start..bytes.len().saturating_sub(21))
        .rev()
        .find(|&i| {
            bytes.get(i..i + 4) == Some(b"PK\x05\x06")
                && i + 22 + u16::from_le_bytes([bytes[i + 20], bytes[i + 21]]) as usize
                    == bytes.len()
        })
        .ok_or("Corrupt DOCX ZIP: end directory is missing.")?;
    let u16_at = |n| u16::from_le_bytes([bytes[pos + n], bytes[pos + n + 1]]);
    if u16_at(4) != 0 || u16_at(6) != 0 || u16_at(8) != u16_at(10) || u16_at(10) > 2048 {
        return Err(
            "DOCX ZIP requires one volume, at most 2048 entries, and no ZIP64 directory.".into(),
        );
    }
    let u32_at = |n| u32::from_le_bytes(bytes[pos + n..pos + n + 4].try_into().unwrap()) as usize;
    let (mut cursor, size) = (u32_at(16), u32_at(12));
    if cursor.checked_add(size) != Some(pos) {
        return Err(
            "Unsupported DOCX ZIP central directory layout (ZIP64 is not supported).".into(),
        );
    }
    for _ in 0..u16_at(10) {
        if cursor.checked_add(46).is_none_or(|end| end > pos)
            || bytes.get(cursor..cursor + 4) != Some(b"PK\x01\x02")
        {
            return Err("Corrupt DOCX ZIP central directory.".into());
        }
        let len = |n| u16::from_le_bytes([bytes[cursor + n], bytes[cursor + n + 1]]) as usize;
        cursor += 46 + len(28) + len(30) + len(32);
        if cursor > pos {
            return Err("Corrupt DOCX ZIP directory lengths.".into());
        }
    }
    if cursor != pos {
        return Err("DOCX ZIP entry count does not match its directory.".into());
    }
    Ok(())
}
fn docx(bytes: &[u8]) -> Result<String> {
    zip_preflight(bytes)?;
    let mut zip =
        zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("Corrupt DOCX ZIP: {e}"))?;
    if zip.len() > 2048
        || zip
            .decompressed_size()
            .is_none_or(|n| n > 128 * 1024 * 1024)
    {
        return Err("DOCX ZIP exceeds entry or 128 MiB expanded-package limits.".into());
    }
    for i in 0..zip.len() {
        let f = zip
            .by_index(i)
            .map_err(|e| format!("Unsupported DOCX ZIP entry: {e}"))?;
        if f.size() > 32 * 1024 * 1024 || f.size() > f.compressed_size().max(1).saturating_mul(200)
        {
            return Err("DOCX ZIP entry exceeds size or 200:1 compression-ratio limits.".into());
        }
    }
    let f = zip
        .by_name("word/document.xml")
        .map_err(|_| "DOCX has no readable word/document.xml body.")?;
    if f.size() > MAX_XML as u64 {
        return Err("DOCX body XML exceeds the 8 MiB limit.".into());
    }
    let mut xml = Vec::new();
    f.take(MAX_XML as u64 + 1)
        .read_to_end(&mut xml)
        .map_err(|e| format!("Corrupt DOCX body: {e}"))?;
    if xml.len() > MAX_XML {
        return Err("DOCX body decompression exceeds the 8 MiB limit.".into());
    }
    body_xml(utf8(&xml)?)
}
pub(crate) fn body_xml(xml: &str) -> Result<String> {
    let mut reader = NsReader::from_str(xml);
    reader.config_mut().expand_empty_elements = true;
    reader.config_mut().check_comments = true;
    let mut stack: Vec<(String, bool)> = vec![];
    let mut body = false;
    let mut body_seen = false;
    let mut root_seen = false;
    let mut skip = 0usize;
    let mut table_depth = 0usize;
    let mut out = String::new();
    loop {
        let (ns, event) = reader
            .read_resolved_event()
            .map_err(|e| format!("Malformed DOCX XML: {e}"))?;
        let word =
            matches!(ns, ResolveResult::Bound(ref n) if n.as_ref() == W || n.as_ref() == STRICT_W);
        match event {
            Event::Start(e) => {
                let local = e.local_name().as_ref().to_owned();
                for a in e.attributes().with_checks(true) {
                    a.map_err(|e| format!("Malformed DOCX attribute: {e}"))?;
                }
                if stack.is_empty() {
                    if root_seen || !word || local != "document" {
                        return Err("DOCX XML must have one Word document root.".into());
                    }
                    root_seen = true;
                }
                if stack.len() >= 128 {
                    return Err("DOCX XML nesting exceeds 128 levels.".into());
                }
                if word && local == "body" {
                    if body_seen || stack.len() != 1 {
                        return Err("DOCX has an invalid document body.".into());
                    }
                    body_seen = true;
                    body = true;
                }
                if skip > 0 {
                    skip += 1;
                } else if body
                    && word
                    && matches!(
                        local.as_str(),
                        "del"
                            | "moveFrom"
                            | "txbxContent"
                            | "drawing"
                            | "pict"
                            | "object"
                            | "altChunk"
                    )
                {
                    skip = 1;
                }
                if body && skip == 0 && word {
                    match local.as_str() {
                        "tbl" => {
                            table_depth += 1;
                            if table_depth > 1 {
                                return Err(
                                    "Nested DOCX tables are not supported for comparison.".into()
                                );
                            }
                        }
                        "tab" => out.push('\t'),
                        "br" | "cr" => out.push('\n'),
                        "noBreakHyphen" => out.push('\u{2011}'),
                        "softHyphen" => out.push('\u{ad}'),
                        _ => {}
                    }
                }
                stack.push((local, word));
            }
            Event::End(_) => {
                let (local, word) = stack.pop().ok_or("Malformed DOCX closing element.")?;
                if skip > 0 {
                    skip -= 1;
                } else if body && word {
                    match local.as_str() {
                        "p" => out.push('\n'),
                        "tc" => {
                            if out.ends_with('\n') {
                                out.pop();
                            }
                            out.push('\t');
                        }
                        "tr" => {
                            if out.ends_with('\t') {
                                out.pop();
                            }
                            out.push('\n');
                        }
                        "tbl" => table_depth -= 1,
                        "body" => body = false,
                        _ => {}
                    }
                }
            }
            Event::Text(e)
                if body && skip == 0 && stack.last().is_some_and(|(n, w)| *w && n == "t") =>
            {
                out.push_str(e.as_ref());
            }
            Event::CData(e)
                if body && skip == 0 && stack.last().is_some_and(|(n, w)| *w && n == "t") =>
            {
                out.push_str(e.as_ref());
            }
            Event::Text(e) if stack.is_empty() && !e.as_ref().trim().is_empty() => {
                return Err("Malformed DOCX XML: text outside the document root.".into());
            }
            Event::CData(_) if stack.is_empty() => {
                return Err("Malformed DOCX XML: CDATA outside the document root.".into());
            }
            Event::GeneralRef(e) => {
                if stack.is_empty() {
                    return Err("Malformed DOCX XML: reference outside the document root.".into());
                }
                let name: &str = e.as_ref();
                let resolved = quick_xml::escape::unescape(&format!("&{name};"))
                    .map_err(|_| "Unsupported XML entity; custom entities are not allowed.")?
                    .into_owned();
                if body && skip == 0 && stack.last().is_some_and(|(n, w)| *w && n == "t") {
                    out.push_str(&resolved);
                }
            }
            Event::DocType(_) => {
                return Err("DOCX XML DTDs and external entities are not allowed.".into())
            }
            Event::Decl(e) => {
                if let Some(enc) = e.encoding() {
                    let enc = enc.map_err(|e| format!("Invalid XML encoding: {e}"))?;
                    if !enc.eq_ignore_ascii_case("utf-8") {
                        return Err("DOCX XML must use UTF-8 encoding.".into());
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
        if out.len() > MAX_TEXT {
            return Err("Extracted DOCX text exceeds the 4 MiB limit.".into());
        }
    }
    if !stack.is_empty() || !body_seen {
        return Err("DOCX document body is missing or incomplete.".into());
    }
    Ok(out)
}
