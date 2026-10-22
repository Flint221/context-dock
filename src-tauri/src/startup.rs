//! Per-user registration only. Never write Windows' StartupApproved override.
use crate::registry::Result;
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub registered: bool,
    pub enabled: Option<bool>,
}
#[cfg(windows)]
const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(windows)]
pub(crate) const APPROVED: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
#[cfg(windows)]
pub(crate) fn status_for(name: &str, command: &str) -> Result<Status> {
    use windows_registry::CURRENT_USER;
    let entry = match CURRENT_USER.open(RUN).and_then(|k| k.get_string(name)) {
        Ok(s) => Some(s),
        Err(e) => {
            if e.code().0 as u32 == 0x80070002 {
                None
            } else {
                return Err(format!("Cannot read Windows startup registration: {e}"));
            }
        }
    };
    let registered = entry.as_deref() == Some(command);
    let enabled = if !registered {
        Some(false)
    } else {
        match CURRENT_USER.open(APPROVED).and_then(|k| k.get_value(name)) {
            Ok(v) => match v.first() {
                Some(2 | 6) => Some(true),
                Some(3 | 7) => Some(false),
                _ => None,
            },
            Err(e) if e.code().0 as u32 == 0x80070002 => Some(true),
            Err(_) => None,
        }
    };
    Ok(Status {
        registered,
        enabled,
    })
}
#[cfg(windows)]
pub(crate) fn set_for(name: &str, command: &str, value: bool) -> Result<()> {
    use windows_registry::CURRENT_USER;
    if value {
        CURRENT_USER
            .create(RUN)
            .and_then(|k| k.set_string(name, command))
            .map_err(|e| e.to_string())?;
    } else if let Err(e) = CURRENT_USER
        .options()
        .write()
        .open(RUN)
        .and_then(|k| k.remove_value(name))
    {
        if e.code().0 as u32 != 0x80070002 {
            return Err(e.to_string());
        }
    }
    if status_for(name, command)?.registered != value {
        return Err("Windows startup registration could not be verified.".into());
    }
    Ok(())
}
#[cfg(windows)]
fn command() -> Result<String> {
    Ok(format!(
        "\"{}\" --background",
        std::env::current_exe()
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("Executable path cannot be registered for startup.")?
    ))
}
#[cfg(windows)]
pub fn status() -> Result<Status> {
    status_for("ContextDock", &command()?)
}
#[cfg(windows)]
pub fn set(value: bool) -> Result<()> {
    set_for("ContextDock", &command()?, value)
}
#[cfg(not(windows))]
pub fn status() -> Result<Status> {
    Ok(Status {
        registered: false,
        enabled: None,
    })
}
#[cfg(not(windows))]
pub fn set(_: bool) -> Result<()> {
    Err("Windows startup is available on Windows.".into())
}
