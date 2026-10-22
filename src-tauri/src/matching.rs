use regex::Regex;
use std::{path::Path, sync::OnceLock};

#[derive(Debug, PartialEq, Eq)]
pub struct Stem {
    pub normalized: String,
    pub display: String,
    pub versioned: bool,
}
/// Only remove recognizable *trailing* version/date groups. Never match by shared words.
pub fn stem(name: &str) -> Stem {
    static SUFFIX: OnceLock<Regex> = OnceLock::new();
    let suffix=SUFFIX.get_or_init(|| Regex::new(r"(?i)(?:[\s_\-]*(?:\(?v(?:ersion)?[\s_\-]*\d+(?:\.\d+)*\)?|(?:19|20)\d{2}[-_]\d{2}[-_]\d{2}|(?:19|20)\d{6}))$").expect("constant filename pattern"));
    let raw = Path::new(name)
        .file_stem()
        .and_then(|x| x.to_str())
        .unwrap_or(name);
    let mut base = raw.trim().to_string();
    let mut versioned = false;
    loop {
        let trimmed = suffix
            .replace(&base, "")
            .trim_matches([' ', '_', '-'])
            .to_string();
        if trimmed == base || trimmed.is_empty() {
            break;
        }
        base = trimmed;
        versioned = true;
    }
    let display = base
        .replace(['_', '-'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    Stem {
        normalized: display.to_lowercase(),
        display,
        versioned,
    }
}
pub fn parent_key(relative: &str) -> String {
    crate::models::key(
        Path::new(relative)
            .parent()
            .unwrap_or(Path::new(""))
            .to_str()
            .unwrap_or(""),
    )
}
/// Explicit numeric export order; dates/ambiguous labels are not guessed.
pub fn version_order(name: &str) -> Option<Vec<u64>> {
    static VERSION: OnceLock<Regex> = OnceLock::new();
    let pattern = VERSION.get_or_init(|| {
        Regex::new(r"(?i)(?:^|[\s_-])v(?:ersion)?[\s_-]*(\d+(?:\.\d+)*)\)?$")
            .expect("constant version pattern")
    });
    let raw = Path::new(name).file_stem()?.to_str()?;
    let captures = pattern.captures(raw)?;
    let mut order = captures[1]
        .split('.')
        .map(str::parse)
        .collect::<std::result::Result<Vec<u64>, _>>()
        .ok()?;
    while order.len() > 1 && order.last() == Some(&0) {
        order.pop();
    }
    Some(order)
}
pub fn safe_name(value: &str) -> String {
    let value: String = value
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .take(60)
        .collect();
    let value = value.trim_matches([' ', '.']);
    if value.is_empty() {
        "Document".into()
    } else {
        value.into()
    }
}

/// Display the author's filename version separately from the snapshot sequence.
pub fn version_label(name: &str) -> Option<String> {
    static LABEL: OnceLock<Regex> = OnceLock::new();
    let pattern = LABEL.get_or_init(|| Regex::new(r"(?i)(?:^|[\s_(-])v(?:ersion)?[\s_-]*(\d+(?:\.\d+)*)\)?(?:[\s_-]+(?:(?:19|20)\d{2}[-_]\d{2}[-_]\d{2}|(?:19|20)\d{6}))*$").expect("constant version label pattern"));
    let raw = Path::new(name).file_stem()?.to_str()?;
    let captures = pattern.captures(raw)?;
    Some(format!("v{}", &captures[1]))
}
