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
    fn target(name: &str) -> Vec<u16> {
        name.encode_utf16().chain(Some(0)).collect()
    }
    pub fn read_named(name: &str) -> Result<Option<Zeroizing<String>>, String> {
        let mut credential = std::ptr::null_mut();
        unsafe {
            if CredReadW(target(name).as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) == 0 {
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
    pub fn save_named(target_name: &str, key: &str) -> Result<(), String> {
        if key.len() > CRED_MAX_CREDENTIAL_BLOB_SIZE as usize {
            return Err("The credential exceeds the Windows vault size limit.".into());
        }
        let mut name = target(target_name);
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
    pub fn delete_named(name: &str) -> Result<(), String> {
        if unsafe { CredDeleteW(target(name).as_ptr(), CRED_TYPE_GENERIC, 0) } == 0
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
pub fn read_named(_: &str) -> Result<Option<Zeroizing<String>>, String> {
    Ok(None)
}
#[cfg(not(windows))]
pub fn save_named(_: &str, _: &str) -> Result<(), String> {
    Err("Windows Credential Manager is required.".into())
}
#[cfg(not(windows))]
pub fn delete_named(_: &str) -> Result<(), String> {
    Ok(())
}
pub fn read() -> Result<Option<Zeroizing<String>>, String> {
    read_named("ContextDock.OpenAI")
}
pub fn save(key: &str) -> Result<(), String> {
    save_named("ContextDock.OpenAI", key)
}
pub fn delete() -> Result<(), String> {
    delete_named("ContextDock.OpenAI")
}
