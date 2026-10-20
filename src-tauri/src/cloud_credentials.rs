//! Secrets exist only in transient native memory and the user's Windows vault.
use zeroize::Zeroizing;
pub fn validate(key: String) -> Result<Zeroizing<String>, String> {
    let key = Zeroizing::new(key);
    if !(20..=4096).contains(&key.len()) || !key.bytes().all(|b| (33..127).contains(&b)) {
        return Err("Enter a valid API key without whitespace.".into());
    }
    Ok(key)
}
#[cfg(windows)]
mod windows {
    use super::*;
    use windows_sys::Win32::Security::Credentials::*;
    fn target() -> Vec<u16> {
        "ContextDock.OpenAI\0".encode_utf16().collect()
    }
    pub fn read() -> Result<Option<Zeroizing<String>>, String> {
        let mut credential = std::ptr::null_mut();
        unsafe {
            if CredReadW(target().as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) == 0 {
                if windows_sys::Win32::Foundation::GetLastError() == 1168 {
                    return Ok(None);
                }
                return Err("Windows Credential Manager could not be read.".into());
            }
            let c = &*credential;
            let bytes =
                std::slice::from_raw_parts_mut(c.CredentialBlob, c.CredentialBlobSize as usize);
            let result = String::from_utf8(bytes.to_vec())
                .map(Zeroizing::new)
                .map_err(|_| "Saved credential is invalid.".into());
            zeroize::Zeroize::zeroize(bytes);
            CredFree(credential.cast());
            result.map(Some)
        }
    }
    pub fn save(key: &str) -> Result<(), String> {
        let mut name = target();
        let c = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: name.as_mut_ptr(),
            CredentialBlobSize: key.len() as u32,
            CredentialBlob: key.as_ptr() as *mut u8,
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            ..Default::default()
        };
        if unsafe { CredWriteW(&c, 0) } == 0 {
            return Err("Windows Credential Manager could not save the key.".into());
        }
        Ok(())
    }
    pub fn delete() -> Result<(), String> {
        if unsafe { CredDeleteW(target().as_ptr(), CRED_TYPE_GENERIC, 0) } == 0
            && unsafe { windows_sys::Win32::Foundation::GetLastError() } != 1168
        {
            return Err("Windows Credential Manager could not remove the key.".into());
        }
        Ok(())
    }
}
#[cfg(windows)]
pub use windows::*;
#[cfg(not(windows))]
pub fn read() -> Result<Option<Zeroizing<String>>, String> {
    Ok(None)
}
#[cfg(not(windows))]
pub fn save(_: &str) -> Result<(), String> {
    Err("Windows Credential Manager is required.".into())
}
#[cfg(not(windows))]
pub fn delete() -> Result<(), String> {
    Ok(())
}
