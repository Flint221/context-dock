//! Win32 notification identity is separate from the window/shortcut identity.
use crate::registry::Result;

#[derive(Debug, PartialEq)]
pub enum Delivery {
    Submitted,
    Disabled,
}

#[derive(Default)]
pub struct Queue {
    attempts: std::collections::HashMap<String, (i64, Option<String>)>,
}
impl Queue {
    pub fn attempt(
        &mut self,
        key: &str,
        now: i64,
        send: impl FnOnce() -> Result<Delivery>,
    ) -> bool {
        if self
            .attempts
            .get(key)
            .is_some_and(|(last, _)| now - last < 60)
        {
            return false;
        }
        let result = send();
        let submitted = matches!(result, Ok(Delivery::Submitted));
        self.attempts.insert(key.into(), (now, result.err()));
        submitted
    }
    pub fn retain(&mut self, keys: &std::collections::HashSet<String>) {
        self.attempts.retain(|key, _| keys.contains(key));
    }
    pub fn error(&self) -> Option<String> {
        self.attempts.values().find_map(|(_, error)| error.clone())
    }
}

#[cfg(windows)]
const STUB_ACTIVATOR: &str = "{F2B6E09C-86C1-4A12-B15E-C8330596F08B}";

#[cfg(windows)]
struct Apartment(bool);
#[cfg(windows)]
impl Apartment {
    fn enter() -> Self {
        Self(
            unsafe {
                windows::Win32::System::WinRT::RoInitialize(
                    windows::Win32::System::WinRT::RO_INIT_MULTITHREADED,
                )
            }
            .is_ok(),
        )
    }
}
#[cfg(windows)]
impl Drop for Apartment {
    fn drop(&mut self) {
        if self.0 {
            unsafe { windows::Win32::System::WinRT::RoUninitialize() };
        }
    }
}

#[cfg(windows)]
fn register(id: &str, icon: Option<&std::path::Path>) -> Result<()> {
    let key = windows_registry::CURRENT_USER
        .create(format!(r"Software\Classes\AppUserModelId\{id}"))
        .map_err(|e| format!("Cannot register Windows notification identity: {e}"))?;
    // Protocol activation only: a stable stub CLSID retains alerts in Notification
    // Center without a COM server. Never modify Windows notification permissions.
    for (name, value) in [
        ("DisplayName", "ContextDock"),
        ("CustomActivator", STUB_ACTIVATOR),
    ] {
        if key.get_string(name).ok().as_deref() != Some(value) {
            key.set_string(name, value).map_err(|e| e.to_string())?;
        }
    }
    if let Some(icon) = icon.filter(|p| p.is_file()).and_then(|p| p.to_str()) {
        if key.get_string("IconUri").ok().as_deref() != Some(icon) {
            key.set_string("IconUri", icon).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub fn send(
    app: &tauri::AppHandle,
    title: &str,
    body: &str,
    route: &str,
    key: &str,
) -> Result<Delivery> {
    use tauri::Manager;
    let icon = app
        .path()
        .resolve("ContextDock.ico", tauri::path::BaseDirectory::Resource)
        .ok();
    send_for(
        &app.config().identifier,
        icon.as_deref(),
        title,
        body,
        route,
        key,
        false,
    )
}

#[cfg(windows)]
fn send_for(
    id: &str,
    icon: Option<&std::path::Path>,
    title: &str,
    body: &str,
    route: &str,
    key: &str,
    suppress: bool,
) -> Result<Delivery> {
    register(id, icon)?;
    submit(id, title, body, route, key, suppress)
}

#[cfg(windows)]
fn submit(
    id: &str,
    title: &str,
    body: &str,
    route: &str,
    key: &str,
    suppress: bool,
) -> Result<Delivery> {
    use sha2::{Digest, Sha256};
    use windows::{
        core::HSTRING,
        Data::Xml::Dom::XmlDocument,
        UI::Notifications::{NotificationSetting, ToastNotification, ToastNotificationManager},
    };
    let _apartment = Apartment::enter();
    let stage =
        |label: &str, e: windows::core::Error| format!("Windows notification {label} failed: {e}");
    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(id))
        .map_err(|e| stage("identity lookup", e))?;
    match notifier.Setting() {
        Ok(setting) if setting != NotificationSetting::Enabled => return Ok(Delivery::Disabled),
        Ok(_) => (),
        // Windows may not know a newly registered sender until its first Show.
        // Show still enforces OS permissions; never treat any other failure as enabled.
        Err(e) if e.code().0 as u32 == 0x80070490 => (),
        Err(e) => return Err(stage("permission check", e)),
    }
    let xml = XmlDocument::new().map_err(|e| stage("creation", e))?;
    xml.LoadXml(&HSTRING::from(format!("<toast activationType=\"protocol\" launch=\"{}\"><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual></toast>", escape(route), escape(title), escape(body))))
        .map_err(|e| stage("content preparation", e))?;
    let toast =
        ToastNotification::CreateToastNotification(&xml).map_err(|e| stage("creation", e))?;
    toast
        .SetTag(&HSTRING::from(
            &format!("{:x}", Sha256::digest(key.as_bytes()))[..16],
        ))
        .map_err(|e| stage("tag", e))?;
    toast
        .SetGroup(&HSTRING::from("ContextDock"))
        .map_err(|e| stage("group", e))?;
    toast
        .SetSuppressPopup(suppress)
        .map_err(|e| stage("popup configuration", e))?;
    notifier.Show(&toast).map_err(|e| stage("submission", e))?;
    let setting = notifier
        .Setting()
        .map_err(|e| stage("permission check after submission", e))?;
    if setting != NotificationSetting::Enabled {
        return Ok(Delivery::Disabled);
    }
    Ok(Delivery::Submitted)
}

#[cfg(not(windows))]
fn send_for(
    _: &str,
    _: Option<&std::path::Path>,
    _: &str,
    _: &str,
    _: &str,
    _: &str,
    _: bool,
) -> Result<Delivery> {
    Ok(Delivery::Disabled)
}

#[cfg(windows)]
fn escape(v: &str) -> String {
    v.chars()
        .filter(|c| *c >= ' ' || *c == '\n')
        .collect::<String>()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Explicit Windows notification test; disposable identity, no popup, no project data"]
    fn windows_notification_identity_reproduces_and_repairs_missing_element() {
        let _apartment = Apartment::enter();
        let id = format!(
            "com.contextdock.verification.{}",
            uuid::Uuid::new_v4().simple()
        );
        struct Cleanup(String);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = windows::UI::Notifications::ToastNotificationManager::History()
                    .and_then(|h| h.ClearWithId(&windows::core::HSTRING::from(&self.0)));
                let _ = windows_registry::CURRENT_USER
                    .remove_tree(format!(r"Software\Classes\AppUserModelId\{}", self.0));
                let _ = windows_registry::CURRENT_USER.remove_tree(format!(
                    r"Software\Microsoft\Windows\CurrentVersion\Notifications\Settings\{}",
                    self.0
                ));
            }
        }
        let _cleanup = Cleanup(id.clone());
        use windows::{core::HSTRING, UI::Notifications::ToastNotificationManager};
        let previous = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(&id))
            .unwrap()
            .Setting();
        eprintln!("First-send permission lookup: {previous:?}");
        assert_eq!(previous.unwrap_err().code().0 as u32, 0x80070490);
        assert_eq!(
            send_for(
                &id,
                None,
                "ContextDock verification",
                "Disposable notification",
                "contextdock://review",
                "test",
                true
            )
            .unwrap(),
            Delivery::Submitted
        );
        // Repeated registration is idempotent.
        register(&id, None).unwrap();
        assert_eq!(
            windows_registry::CURRENT_USER
                .open(format!(r"Software\Classes\AppUserModelId\{id}"))
                .unwrap()
                .get_string("CustomActivator")
                .unwrap(),
            STUB_ACTIVATOR
        );
        let history = ToastNotificationManager::History()
            .unwrap()
            .GetHistoryWithId(&HSTRING::from(&id))
            .unwrap();
        assert_eq!(history.Size().unwrap(), 1);
        assert_eq!(history.GetAt(0).unwrap().Group().unwrap(), "ContextDock");
    }
}

#[cfg(test)]
mod queue_tests {
    use super::*;
    #[test]
    fn failed_alerts_back_off_without_blocking_other_projects_and_recover() {
        let mut queue = Queue::default();
        assert!(!queue.attempt("approval:one", 1000, || Err("Windows unavailable".into())));
        assert_eq!(queue.error().as_deref(), Some("Windows unavailable"));
        assert!(!queue.attempt("approval:one", 1002, || panic!("Must wait before retry")));
        assert!(queue.attempt("approval:two", 1002, || Ok(Delivery::Submitted)));
        assert!(queue.attempt("approval:one", 1060, || Ok(Delivery::Submitted)));
        assert_eq!(queue.error(), None);
    }
    #[test]
    fn windows_suppression_is_not_delivery_and_resolved_errors_are_retired() {
        let mut queue = Queue::default();
        assert!(!queue.attempt("approval:one", 1000, || Ok(Delivery::Disabled)));
        assert!(!queue.attempt("approval:one", 1002, || panic!(
            "Suppressed alerts also back off"
        )));
        assert_eq!(queue.error(), None);
        assert!(!queue.attempt("problem:two", 1000, || Err("Windows unavailable".into())));
        queue.retain(&["approval:one".into()].into_iter().collect());
        assert_eq!(queue.error(), None);
        assert!(queue.attempt("approval:one", 1060, || Ok(Delivery::Submitted)));
    }
}
