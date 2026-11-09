use crate::drive_auth::{self, NativeVault, OAuthHttp, Vault};
use serde::Serialize;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

#[derive(Default)]
pub struct DriveRuntime {
    pub gate: Arc<Mutex<()>>,
    pending: Arc<Mutex<Option<Arc<AtomicBool>>>>,
    error: Arc<Mutex<Option<String>>>,
}
impl Drop for DriveRuntime {
    fn drop(&mut self) {
        if let Ok(pending) = self.pending.lock() {
            if let Some(cancel) = pending.as_ref() {
                cancel.store(true, Ordering::SeqCst);
            }
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    configured: bool,
    connected: bool,
    connecting: bool,
    account: Option<drive_auth::Account>,
    error: Option<String>,
}
#[tauri::command]
pub async fn drive_status(state: tauri::State<'_, DriveRuntime>) -> Result<Connection, String> {
    let connecting = state
        .pending
        .lock()
        .map_err(|_| "Drive connection unavailable.")?
        .is_some();
    let error = state
        .error
        .lock()
        .map_err(|_| "Drive connection unavailable.")?
        .clone();
    tauri::async_runtime::spawn_blocking(move || {
        let saved = NativeVault.load()?;
        let account = saved
            .as_ref()
            .and_then(|s| s.session.as_ref())
            .map(|s| s.account.clone());
        Ok(Connection {
            configured: saved.is_some(),
            connected: account.is_some(),
            connecting,
            account,
            error,
        })
    })
    .await
    .map_err(|_| "Could not read the Drive connection.".to_string())?
}

struct Pending(Arc<Mutex<Option<Arc<AtomicBool>>>>);
impl Drop for Pending {
    fn drop(&mut self) {
        if let Ok(mut pending) = self.0.lock() {
            *pending = None;
        }
    }
}
#[tauri::command]
pub fn drive_cancel(state: tauri::State<'_, DriveRuntime>) -> Result<(), String> {
    if let Some(cancel) = state
        .pending
        .lock()
        .map_err(|_| "Drive connection unavailable.")?
        .as_ref()
    {
        cancel.store(true, Ordering::SeqCst);
    }
    Ok(())
}
#[tauri::command]
pub async fn drive_action(
    state: tauri::State<'_, DriveRuntime>,
    action: String,
    path: Option<String>,
) -> Result<(), String> {
    let gate = state.gate.clone();
    let pending = state.pending.clone();
    let error = state.error.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = gate
            .try_lock()
            .map_err(|_| "A Google Drive operation is already running.")?;
        if let Ok(mut slot) = error.lock() {
            *slot = None;
        }
        let result = action_inner(&action, path.as_deref(), pending);
        if let Ok(mut slot) = error.lock() {
            *slot = result.as_ref().err().cloned();
        }
        result
    })
    .await
    .map_err(|_| "The Google Drive operation could not finish.".to_string())?
}
fn action_inner(
    action: &str,
    path: Option<&str>,
    pending: Arc<Mutex<Option<Arc<AtomicBool>>>>,
) -> Result<(), String> {
    match action {
        "import" => {
            if NativeVault.load()?.is_some_and(|s| s.session.is_some()) {
                return Err("Disconnect Google Drive before replacing its OAuth client.".into());
            }
            let path = path.ok_or("Choose a Desktop OAuth client JSON.")?;
            let file = std::fs::File::open(path)
                .map_err(|_| "The OAuth client JSON could not be opened.")?;
            let mut bytes = zeroize::Zeroizing::new(Vec::new());
            std::io::Read::take(file, 65_537)
                .read_to_end(&mut bytes)
                .map_err(|_| "The OAuth client JSON could not be read.")?;
            drive_auth::configure(&NativeVault, &bytes)
        }
        "connect" => {
            let mut saved = NativeVault
                .load()?
                .ok_or("Import the ContextDock Desktop OAuth client first.")?;
            let cancel = Arc::new(AtomicBool::new(false));
            *pending
                .lock()
                .map_err(|_| "Drive connection unavailable.")? = Some(cancel.clone());
            let _pending = Pending(pending);
            let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
                .map_err(|_| "Could not open the local Google sign-in callback.")?;
            let flow = drive_auth::Flow::new(
                &saved.config,
                listener
                    .local_addr()
                    .map_err(|_| "Google sign-in callback unavailable.")?
                    .port(),
            );
            open_browser(&flow.url)?;
            let code = flow.wait(listener, &cancel, Duration::from_secs(180))?;
            let session = drive_auth::finish(
                &saved.config,
                &flow,
                &code,
                &drive_auth::GoogleHttp::new()?,
                chrono::Utc::now().timestamp(),
            )?;
            if cancel.load(Ordering::SeqCst) {
                return Err("Google sign-in cancelled. No new connection was saved.".into());
            }
            saved.session = Some(session);
            NativeVault.save(&saved)
        }
        "test" => {
            let http = drive_auth::GoogleHttp::new()?;
            let access = drive_auth::access(&NativeVault, &http, chrono::Utc::now().timestamp())?;
            let actual = http.account(&access)?;
            let saved = NativeVault.load()?.ok_or("Google Drive disconnected.")?;
            if saved
                .session
                .as_ref()
                .is_none_or(|s| s.account.id != actual.id)
            {
                return Err(
                    "Google Drive returned a different account. Disconnect and connect again."
                        .into(),
                );
            }
            Ok(())
        }
        "disconnect" => drive_auth::disconnect(&NativeVault, &drive_auth::GoogleHttp::new()?),
        "forget_client" => {
            if NativeVault.load()?.is_some_and(|s| s.session.is_some()) {
                return Err("Disconnect Google Drive before removing its OAuth client.".into());
            }
            NativeVault.clear()
        }
        _ => Err("Unknown Google Drive action.".into()),
    }
}
use std::io::Read;
fn open_browser(url: &str) -> Result<(), String> {
    if !url.starts_with("https://accounts.google.com/o/oauth2/v2/auth?") || url.contains('\0') {
        return Err("Invalid Google authorization URL.".into());
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};
        let operation = "open\0".encode_utf16().collect::<Vec<_>>();
        let target = url.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                operation.as_ptr(),
                target.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        if result as isize <= 32 {
            return Err("Could not open your default browser for Google sign-in.".into());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err("Google Drive sign-in currently requires Windows.".into())
    }
}
