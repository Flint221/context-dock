use crate::models::{key, now};
use std::{fs, path::Path};
use walkdir::{DirEntry, WalkDir};
pub struct Observed {
    pub name: String,
    pub relative: String,
    pub key: String,
    pub kind: String,
    pub size: i64,
    pub modified: Option<String>,
}
pub struct Scan {
    pub files: Vec<Observed>,
    pub errors: Vec<String>,
}
fn excluded(e: &DirEntry) -> bool {
    let name = e.file_name().to_string_lossy().to_lowercase();
    e.depth() > 0
        && (is_link(e.path())
            || (e.file_type().is_dir()
                && matches!(
                    name.as_str(),
                    ".git"
                        | "node_modules"
                        | "target"
                        | "dist"
                        | "build"
                        | ".next"
                        | ".contextdock"
                        | ".contextdock-archive"
                        | "contextdock-archive"
                )))
}
fn is_link(path: &Path) -> bool {
    let Ok(m) = fs::symlink_metadata(path) else {
        return false;
    };
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        m.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        m.file_type().is_symlink()
    }
}
pub fn canonical(path: &str) -> Result<String, String> {
    let p = Path::new(path);
    if !p.is_absolute() {
        return Err("Choose an absolute existing folder path.".into());
    }
    let p = fs::canonicalize(p)
        .map_err(|_| "The selected folder is missing or inaccessible.".to_string())?;
    fs::read_dir(&p).map_err(|_| "The selected folder cannot be read.".to_string())?;
    p.to_str()
        .map(String::from)
        .ok_or("Folder path must be valid Unicode.".into())
}
pub fn scan(root: &Path) -> Result<Scan, String> {
    fs::read_dir(root).map_err(|_| {
        "Folder is missing or inaccessible. Reconnect the drive or check permissions.".to_string()
    })?;
    let mut scan = Scan {
        files: vec![],
        errors: vec![],
    };
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !excluded(e))
    {
        let e = match entry {
            Ok(e) => e,
            Err(e) => {
                scan.errors.push(e.to_string());
                continue;
            }
        };
        if !e.file_type().is_file() {
            continue;
        }
        let ext = e
            .path()
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        let kind = match ext.as_str() {
            "md" | "markdown" => "markdown",
            "docx" => "word",
            _ => continue,
        };
        let m = match e.metadata() {
            Ok(m) => m,
            Err(e) => {
                scan.errors.push(e.to_string());
                continue;
            }
        };
        let Some(relative) = e.path().strip_prefix(root).ok().and_then(|p| p.to_str()) else {
            scan.errors
                .push("A document path is not valid Unicode.".into());
            continue;
        };
        scan.files.push(Observed {
            name: e.file_name().to_string_lossy().into(),
            relative: relative.into(),
            key: key(relative),
            kind: kind.into(),
            size: m.len().min(i64::MAX as u64) as i64,
            modified: m.modified().ok().map(|t| {
                chrono::DateTime::<chrono::Utc>::from(t)
                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
            }),
        });
    }
    Ok(scan)
}
pub fn observed_at() -> String {
    now()
}
