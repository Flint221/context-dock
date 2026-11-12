//! Recycling is an explicit Shell transfer, never DeleteFile/RemoveItem.
use std::path::Path;

pub enum Volume {
    Ntfs,
    Exfat,
}

pub trait Bin {
    fn volume(&self, path: &Path) -> Result<Volume, String>;
    fn recycle(&self, path: &Path, hash: &str) -> Result<String, String>;
}
pub struct NativeBin;
impl Bin for NativeBin {
    fn volume(&self, path: &Path) -> Result<Volume, String> {
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows_sys::Win32::Storage::FileSystem::{
                GetDriveTypeW, GetVolumeInformationW, GetVolumePathNameW,
            };
            const DRIVE_FIXED: u32 = 3;
            let name = path
                .as_os_str()
                .encode_wide()
                .chain(Some(0))
                .collect::<Vec<_>>();
            let mut volume = [0u16; 32768];
            let mut filesystem = [0u16; 32];
            unsafe {
                if GetVolumePathNameW(name.as_ptr(), volume.as_mut_ptr(), volume.len() as u32) == 0
                    || GetDriveTypeW(volume.as_ptr()) != DRIVE_FIXED
                    || GetVolumeInformationW(
                        volume.as_ptr(),
                        std::ptr::null_mut(),
                        0,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        filesystem.as_mut_ptr(),
                        filesystem.len() as u32,
                    ) == 0
                {
                    return Err("Automatic recycling requires a local fixed NTFS or exFAT drive. Older files stay in place on this drive.".into());
                }
            }
            let filesystem = String::from_utf16_lossy(
                &filesystem[..filesystem
                    .iter()
                    .position(|c| *c == 0)
                    .unwrap_or(filesystem.len())],
            );
            match filesystem.as_str() {
                "NTFS" => Ok(Volume::Ntfs),
                "exFAT" => Ok(Volume::Exfat),
                _ => Err("Automatic recycling supports NTFS and exFAT. Older files stay in place on this drive.".into()),
            }
        }
        #[cfg(not(windows))]
        {
            let _ = path;
            Err("Automatic recycling currently requires Windows.".into())
        }
    }
    fn recycle(&self, path: &Path, hash: &str) -> Result<String, String> {
        self.volume(path)?;
        #[cfg(windows)]
        {
            let path = path.to_path_buf();
            let hash = hash.to_owned();
            std::thread::spawn(move || shell_recycle(&path, &hash))
                .join()
                .map_err(|_| {
                    "Windows recycling could not finish. Inspect the safety folder and Recycle Bin."
                        .to_string()
                })?
        }
        #[cfg(not(windows))]
        {
            let _ = hash;
            Err("Automatic recycling currently requires Windows.".into())
        }
    }
}
#[cfg(windows)]
fn shell_recycle(path: &Path, hash: &str) -> Result<String, String> {
    use windows::{
        core::HSTRING,
        Win32::{
            System::Com::{
                CoInitializeEx, CoTaskMemFree, CoUninitialize, IBindCtx, COINIT_APARTMENTTHREADED,
            },
            UI::Shell::{
                BHID_Transfer, FOLDERID_RecycleBinFolder, IShellItem, ITransferSource,
                SHCreateItemFromParsingName, SHGetKnownFolderItem, KF_FLAG_DEFAULT,
                SIGDN_DESKTOPABSOLUTEPARSING, SIGDN_FILESYSPATH,
            },
        },
    };
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe {
                CoUninitialize();
            }
        }
    }
    let err = |e: windows::core::Error| {
        format!(
            "Windows could not confirm recycling ({}). No permanent-delete fallback was attempted.",
            e.code()
        )
    };
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)
            .ok()
            .map_err(err)?;
        let _apartment = Apartment;
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE, FILE_SHARE_READ,
        };
        // Shell needs delete sharing for its move, but all writes stay denied.
        let mut pinned = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_DELETE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)
            .map_err(|e| e.to_string())?;
        if crate::scanner::is_link(path) || crate::storage::hash(&mut pinned)? != hash {
            return Err("The file changed before recycling. It was retained.".into());
        }
        let name = HSTRING::from(crate::scanner::explorer_path(&path.to_string_lossy()));
        let item: IShellItem =
            SHCreateItemFromParsingName(&name, None::<&IBindCtx>).map_err(err)?;
        let parent = item.GetParent().map_err(err)?;
        let source: ITransferSource = parent
            .BindToHandler(None::<&IBindCtx>, &BHID_Transfer)
            .map_err(err)?;
        let bin: IShellItem =
            SHGetKnownFolderItem(&FOLDERID_RecycleBinFolder, KF_FLAG_DEFAULT, None).map_err(err)?;
        // This method only recycles. Unsupported volumes/operations fail here;
        // there is deliberately no DeleteItem or RemoveItem fallback.
        let recycled = source.RecycleItem(&item, &bin, 0).map_err(err)?;
        let location = recycled
            .GetDisplayName(SIGDN_DESKTOPABSOLUTEPARSING)
            .map_err(err)?;
        let receipt = location
            .to_string()
            .map_err(|_| "The Recycle Bin returned an invalid receipt.".to_string());
        CoTaskMemFree(Some(location.0.cast()));
        let physical = recycled.GetDisplayName(SIGDN_FILESYSPATH).map_err(err)?;
        let physical_path = physical
            .to_string()
            .map_err(|_| "The Recycle Bin returned an invalid physical location.".to_string());
        CoTaskMemFree(Some(physical.0.cast()));
        let physical_path = physical_path?;
        let mut actual = crate::storage::locked_read(Path::new(&physical_path), false)?;
        if crate::storage::hash(&mut actual)? != hash
            || crate::storage::file_identity(&actual)? != crate::storage::file_identity(&pinned)?
        {
            return Err("The Recycle Bin receipt did not match the guarded file. Inspect it before further cleanup.".into());
        }
        receipt
    }
}

#[cfg(all(test, windows))]
#[test]
#[ignore = "Explicit Windows Recycle Bin acceptance; only a disposable unique Markdown fixture"]
fn windows_recycles_one_disposable_file_without_permanent_delete_fallback() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join(format!(
        "ContextDock_Recycle_Test_{}_v1.md",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::write(&path, "Disposable ContextDock recycle acceptance marker.").unwrap();
    let hash =
        crate::storage::hash(&mut crate::storage::locked_read(&path, false).unwrap()).unwrap();
    NativeBin.volume(&path).unwrap();
    let receipt = NativeBin.recycle(&path, &hash).unwrap();
    assert!(!path.exists());
    assert!(!receipt.is_empty());
    println!("Disposable file reached the Windows Recycle Bin with matching content.");
}
