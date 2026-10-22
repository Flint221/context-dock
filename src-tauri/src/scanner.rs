use crate::models::key;
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
pub fn ignored_directory(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        ".git"
            | "node_modules"
            | "target"
            | "dist"
            | "build"
            | ".next"
            | ".contextdock"
            | ".contextdock-archive"
            | "contextdock-archive"
            | "old context files"
    )
}
fn excluded(e: &DirEntry) -> bool {
    e.depth() > 0
        && (e.file_name().to_string_lossy().starts_with("~$")
            || is_link(e.path())
            || (e.file_type().is_dir() && ignored_directory(&e.file_name().to_string_lossy())))
}
pub fn is_link(path: &Path) -> bool {
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
                    .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
            }),
        });
    }
    Ok(scan)
}
pub fn folders(root: &Path) -> Result<(Vec<String>, Vec<String>), String> {
    fs::read_dir(root).map_err(|_| "The project folder cannot be read.")?;
    let mut folders = vec![String::new()];
    let mut warnings = Vec::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !excluded(e))
    {
        match entry {
            Ok(e) if e.depth() > 0 && e.file_type().is_dir() => {
                if folders.len() >= 20_000 {
                    return Err(
                        "Folder tree exceeds 20,000 directories. Choose a smaller project root."
                            .into(),
                    );
                }
                if let Some(p) = e.path().strip_prefix(root).ok().and_then(|p| p.to_str()) {
                    folders.push(p.replace('\\', "/"));
                }
            }
            Err(e) => warnings.push(e.to_string()),
            _ => {}
        }
    }
    Ok((folders, warnings))
}
#[cfg(windows)]
pub fn explorer_path(path: &str) -> String {
    if let Some(unc) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        path.strip_prefix(r"\\?\").unwrap_or(path).to_string()
    }
}
