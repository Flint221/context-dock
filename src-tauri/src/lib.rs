mod matching;
mod models;
mod monitor;
mod operations;
mod registry;
mod scanner;
mod storage;
#[cfg(test)]
mod tests;
mod tracking;
mod tracking_models;
mod tracking_queries;
use models::Snapshot;
use registry::Registry;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{Emitter, Manager};
#[derive(Clone)]
struct Store(Arc<Mutex<Registry>>);
async fn work<T: Send + 'static>(
    state: Store,
    f: impl FnOnce(&mut Registry) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut registry = state
            .0
            .lock()
            .map_err(|_| "Local database is unavailable. Restart ContextDock.".to_string())?;
        f(&mut registry)
    })
    .await
    .map_err(|e| format!("Operation failed: {e}"))?
}
#[tauri::command]
async fn snapshot(state: tauri::State<'_, Store>) -> Result<Snapshot, String> {
    work(state.inner().clone(), |r| r.snapshot()).await
}
#[tauri::command]
async fn add_project(
    state: tauri::State<'_, Store>,
    name: String,
    path: String,
) -> Result<String, String> {
    work(state.inner().clone(), move |r| r.add(&name, &path)).await
}
#[tauri::command]
async fn rename_project(
    state: tauri::State<'_, Store>,
    id: String,
    name: String,
) -> Result<(), String> {
    work(state.inner().clone(), move |r| r.rename(&id, &name)).await
}
#[tauri::command]
async fn remove_project(state: tauri::State<'_, Store>, id: String) -> Result<(), String> {
    work(state.inner().clone(), move |r| r.remove(&id)).await
}
#[tauri::command]
async fn refresh_project(state: tauri::State<'_, Store>, id: String) -> Result<(), String> {
    work(state.inner().clone(), move |r| {
        r.reconcile(
            &id,
            "manual_refresh",
            &std::collections::HashSet::new(),
            true,
        )
        .map(|_| ())
    })
    .await
}
#[tauri::command]
async fn set_theme(state: tauri::State<'_, Store>, theme: String) -> Result<(), String> {
    work(state.inner().clone(), move |r| r.theme(&theme)).await
}
#[tauri::command]
async fn open_project(state: tauri::State<'_, Store>, id: String) -> Result<(), String> {
    work(state.inner().clone(), move |r| {
        let path = r.path(&id)?;
        let path = scanner::canonical(&path)?;
        #[cfg(windows)]
        {
            std::process::Command::new("explorer.exe")
                .arg(scanner::explorer_path(&path))
                .spawn()
                .map_err(|e| format!("Cannot open File Explorer: {e}"))?;
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = path;
            Err("File Explorer is available on Windows.".into())
        }
    })
    .await
}
#[tauri::command]
async fn tracking_view(
    state: tauri::State<'_, Store>,
    project: String,
) -> Result<tracking_models::TrackingView, String> {
    work(state.inner().clone(), move |r| r.tracking_view(&project)).await
}
#[tauri::command]
async fn configure_tracking(
    state: tauri::State<'_, Store>,
    project: String,
    enabled: bool,
    archive: bool,
    selected: Vec<String>,
) -> Result<(), String> {
    work(state.inner().clone(), move |r| {
        r.configure_tracking(&project, enabled, archive, selected)
    })
    .await
}
#[tauri::command]
async fn resolve_match(
    state: tauri::State<'_, Store>,
    project: String,
    source: String,
    action: String,
    target: Option<String>,
) -> Result<(), String> {
    work(state.inner().clone(), move |r| {
        r.resolve_match(&project, &source, &action, target)
    })
    .await
}
#[tauri::command]
async fn document_versions(
    state: tauri::State<'_, Store>,
    document: String,
) -> Result<Vec<tracking_models::Version>, String> {
    work(state.inner().clone(), move |r| r.versions(&document)).await
}
#[tauri::command]
async fn open_version_location(
    state: tauri::State<'_, Store>,
    version: String,
    archive: bool,
) -> Result<(), String> {
    work(state.inner().clone(), move |r| {
        let path = r.version_location(&version, archive)?;
        open_location(&path)
    })
    .await
}
#[tauri::command]
async fn open_current_location(
    state: tauri::State<'_, Store>,
    document: String,
) -> Result<(), String> {
    work(state.inner().clone(), move |r| {
        let path = r.current_location(&document)?;
        open_location(&path)
    })
    .await
}
#[tauri::command]
async fn retry_operations(state: tauri::State<'_, Store>, project: String) -> Result<(), String> {
    work(state.inner().clone(), move |r| {
        let errors = r.recover_operations(&project, true)?;
        r.set_tracking_error(&project, &errors)
    })
    .await
}
fn open_location(path: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        let parent = std::path::Path::new(path)
            .parent()
            .ok_or("Location has no parent folder.")?;
        std::process::Command::new("explorer.exe")
            .arg(scanner::explorer_path(&parent.to_string_lossy()))
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err("File Explorer is available on Windows.".into())
    }
}
struct SessionLock {
    _file: std::fs::File,
}
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir: PathBuf = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let lock = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(dir.join("runtime.lock"))?;
            lock.try_lock().map_err(|e| {
                std::io::Error::other(format!(
                    "ContextDock is already running, or its session lock is unavailable: {e}"
                ))
            })?;
            app.manage(SessionLock { _file: lock });
            let registry = Arc::new(Mutex::new(
                Registry::open(&dir.join("contextdock.db")).map_err(std::io::Error::other)?,
            ));
            app.manage(Store(registry.clone()));
            let handle = app.handle().clone();
            let monitor = monitor::Monitor::start(registry, move |project| {
                let _ = handle.emit("registry-changed", project);
            })?;
            app.manage(monitor);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            add_project,
            rename_project,
            remove_project,
            refresh_project,
            set_theme,
            open_project,
            tracking_view,
            configure_tracking,
            resolve_match,
            document_versions,
            open_version_location,
            open_current_location,
            retry_operations
        ])
        .run(tauri::generate_context!())
        .expect("ContextDock could not start");
}
