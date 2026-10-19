//! Narrow, guarded filesystem primitives. No source deletion fallback is permitted.
use crate::{models, scanner};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Component, Path, PathBuf},
    time::Duration,
};
pub type Result<T> = std::result::Result<T, String>;
pub struct CheckedPath {
    pub path: PathBuf,
    _directories: Vec<File>,
}
fn io(e: std::io::Error) -> String {
    format!("Filesystem operation could not complete: {e}")
}
pub fn directory_guard(path: &Path) -> Result<File> {
    if scanner::is_link(path) {
        return Err("Refusing a symbolic link, junction, or reparse-point directory.".into());
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::*;
        options
            .access_mode(FILE_READ_ATTRIBUTES)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let handle = options.open(path).map_err(io)?;
    let m = handle.metadata().map_err(io)?;
    if !m.is_dir() {
        return Err("Expected a directory.".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if m.file_attributes() & 0x400 != 0 {
            return Err("Refusing a reparse-point directory.".into());
        }
    }
    Ok(handle)
}
/// Pins every traversed directory on Windows, rejecting traversal and reparse points.
pub fn checked(root: &Path, relative: &str, create_parents: bool) -> Result<CheckedPath> {
    let components = Path::new(relative)
        .components()
        .map(|c| match c {
            Component::Normal(n) => Ok(n.to_owned()),
            _ => Err("Unsafe relative path.".to_string()),
        })
        .collect::<Result<Vec<_>>>()?;
    if components.is_empty() {
        return Err("Expected a relative file path.".into());
    }
    let canonical = fs::canonicalize(root).map_err(io)?;
    if models::key(&canonical.to_string_lossy()) != models::key(&root.to_string_lossy()) {
        return Err("The registered root changed location or became a link.".into());
    }
    let mut guards = vec![directory_guard(root)?];
    let mut path = root.to_path_buf();
    for part in &components[..components.len() - 1] {
        path.push(part);
        if create_parents {
            match fs::create_dir(&path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(io(e)),
            }
        }
        guards.push(directory_guard(&path)?);
    }
    path.push(components.last().ok_or("Missing filename")?);
    if scanner::is_link(&path) {
        return Err("Refusing a symbolic link or reparse-point file.".into());
    }
    Ok(CheckedPath {
        path,
        _directories: guards,
    })
}
pub fn locked_read(path: &Path, for_move: bool) -> Result<File> {
    let mut opts = OpenOptions::new();
    opts.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::{Foundation::GENERIC_READ, Storage::FileSystem::*};
        opts.share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
        if for_move {
            opts.access_mode(GENERIC_READ | DELETE);
        }
    }
    #[cfg(not(windows))]
    let _ = for_move;
    let f = opts.open(path).map_err(io)?;
    let m = f.metadata().map_err(io)?;
    if !m.is_file() {
        return Err("Expected an ordinary document file.".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if m.file_attributes() & 0x400 != 0 {
            return Err("Refusing a reparse-point file.".into());
        }
    }
    Ok(f)
}
pub fn hash(file: &mut File) -> Result<String> {
    file.seek(SeekFrom::Start(0)).map_err(io)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf).map_err(io)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
pub fn verify(path: &Path, expected: &str) -> Result<()> {
    if hash(&mut locked_read(path, false)?)? != expected {
        return Err(
            "Integrity conflict: bytes do not match the preserved SHA-256 hash. Nothing was moved."
                .into(),
        );
    }
    Ok(())
}
pub fn stamp(m: &fs::Metadata) -> Option<String> {
    m.modified().ok().map(|t| {
        chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
    })
}
pub struct Staged {
    pub temporary: String,
    pub hash: String,
    pub size: i64,
    pub modified: Option<String>,
}
pub fn stage(root: &Path, relative: &str, store: &Path) -> Result<Staged> {
    let source = checked(root, relative, false)?;
    let first = fs::metadata(&source.path).map_err(io)?;
    std::thread::sleep(Duration::from_millis(if cfg!(test) { 5 } else { 650 }));
    let mut file = locked_read(&source.path, false)?;
    let before = file.metadata().map_err(io)?;
    if first.len() != before.len() || stamp(&first) != stamp(&before) {
        return Err("Document is still being written; capture will be retried.".into());
    }
    let temporary = format!("staging/{}.tmp", uuid::Uuid::new_v4());
    let target = checked(store, &temporary, true)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target.path)
        .map_err(io)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    let mut size = 0u64;
    loop {
        let n = file.read(&mut buf).map_err(io)?;
        if n == 0 {
            break;
        }
        output.write_all(&buf[..n]).map_err(io)?;
        hasher.update(&buf[..n]);
        size += n as u64;
    }
    output.sync_all().map_err(io)?;
    drop(output);
    let after = file.metadata().map_err(io)?;
    if size != before.len() || before.len() != after.len() || stamp(&before) != stamp(&after) {
        return Err("Document changed while being captured. Source was left intact.".into());
    }
    let hash = format!("{:x}", hasher.finalize());
    verify(&target.path, &hash)?;
    Ok(Staged {
        temporary,
        hash,
        size: i64::try_from(size).map_err(|_| "Document is too large.")?,
        modified: stamp(&after),
    })
}
pub fn snapshot_finalize(
    store: &Path,
    temp: &str,
    final_relative: &str,
    expected: &str,
    already_finalized: bool,
) -> Result<()> {
    let final_path = checked(store, final_relative, true)?;
    let temp_path = checked(store, temp, true)?;
    if final_path.path.exists() {
        let same = same_file::is_same_file(&temp_path.path, &final_path.path).unwrap_or(false);
        if !already_finalized && !same {
            return Err(
                "Snapshot destination collision; existing bytes were not overwritten.".into(),
            );
        }
    } else {
        verify(&temp_path.path, expected)?;
        fs::hard_link(&temp_path.path, &final_path.path).map_err(io)?;
    }
    verify(&final_path.path, expected)?;
    locked_read(&final_path.path, false)?
        .sync_all()
        .map_err(io)?;
    Ok(())
}
pub fn snapshot_seal(store: &Path, temp: &str, final_relative: &str) -> Result<()> {
    let final_path = checked(store, final_relative, false)?;
    let temp_path = checked(store, temp, true)?;
    if same_file::is_same_file(&temp_path.path, &final_path.path).unwrap_or(false) {
        fs::remove_file(&temp_path.path).map_err(io)?;
    }
    let f = locked_read(&final_path.path, false)?;
    let mut p = f.metadata().map_err(io)?.permissions();
    p.set_readonly(true);
    f.set_permissions(p).map_err(io)?;
    f.sync_all().map_err(io)?;
    Ok(())
}
pub fn discard_stage(store: &Path, temp: &str) -> Result<()> {
    let p = checked(store, temp, false)?;
    fs::remove_file(&p.path).map_err(io)
}
/// Windows renames the verified, write/delete-denied handle itself, never a re-resolved filename.
#[cfg(windows)]
pub fn rename_verified(file: &File, destination: &Path) -> Result<()> {
    use std::{
        mem::{offset_of, size_of},
        os::windows::{ffi::OsStrExt, io::AsRawHandle},
    };
    use windows_sys::Win32::Storage::FileSystem::{
        FileRenameInfo, SetFileInformationByHandle, FILE_RENAME_INFO,
    };
    let name: Vec<u16> = destination.as_os_str().encode_wide().collect();
    let size = offset_of!(FILE_RENAME_INFO, FileName) + name.len() * size_of::<u16>();
    let mut buffer = vec![0u64; size.div_ceil(size_of::<u64>())];
    // Buffer is aligned for FILE_RENAME_INFO, sized for its trailing UTF-16 filename.
    let ok = unsafe {
        let info = buffer.as_mut_ptr().cast::<FILE_RENAME_INFO>();
        (*info).Anonymous.ReplaceIfExists = 0;
        (*info).RootDirectory = std::ptr::null_mut();
        (*info).FileNameLength = (name.len() * 2) as u32;
        std::ptr::copy_nonoverlapping(
            name.as_ptr(),
            std::ptr::addr_of_mut!((*info).FileName).cast::<u16>(),
            name.len(),
        );
        SetFileInformationByHandle(
            file.as_raw_handle(),
            FileRenameInfo,
            info.cast(),
            size as u32,
        )
    };
    if ok == 0 {
        return Err(format!("Safe archive rename failed (including cross-volume or sharing conflicts): {}. Source was not deleted.",std::io::Error::last_os_error()));
    }
    file.sync_all().map_err(io)?;
    Ok(())
}
#[cfg(not(windows))]
pub fn rename_verified(_file: &File, _destination: &Path) -> Result<()> {
    Err("Source archival moves currently require Windows handle-based rename. Source was left intact.".into())
}
pub fn archive_copy(
    snapshot: &Path,
    root: &Path,
    temp_relative: &str,
    dest_relative: &str,
    expected: &str,
    finalized: bool,
) -> Result<()> {
    let dest = checked(root, dest_relative, true)?;
    let temp = checked(root, temp_relative, true)?;
    if dest.path.exists() {
        if !finalized && !same_file::is_same_file(&temp.path, &dest.path).unwrap_or(false) {
            return Err(
                "Archive destination collision. Existing archive files were not overwritten."
                    .into(),
            );
        }
        return verify(&dest.path, expected);
    }
    if !temp.path.exists() {
        let mut source = locked_read(snapshot, false)?;
        if hash(&mut source)? != expected {
            return Err("Snapshot integrity check failed.".into());
        }
        source.rewind().map_err(io)?;
        let mut out = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp.path)
            .map_err(io)?;
        std::io::copy(&mut source, &mut out).map_err(io)?;
        out.sync_all().map_err(io)?;
    }
    verify(&temp.path, expected)?;
    fs::hard_link(&temp.path, &dest.path).map_err(io)?;
    verify(&dest.path, expected)?;
    locked_read(&dest.path, false)?.sync_all().map_err(io)?;
    Ok(())
}
pub fn cleanup_archive_temp(root: &Path, temp_relative: &str, dest_relative: &str) {
    if let (Ok(temp), Ok(dest)) = (
        checked(root, temp_relative, false),
        checked(root, dest_relative, false),
    ) {
        if same_file::is_same_file(&temp.path, &dest.path).unwrap_or(false) {
            let _ = fs::remove_file(temp.path);
        }
    }
}
