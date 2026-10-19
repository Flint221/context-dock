mod matching;
mod models;
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
use tauri::Manager;
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
    work(state.inner().clone(), move |r| r.refresh(&id)).await
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
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir: PathBuf = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let registry =
                Registry::open(&dir.join("contextdock.db")).map_err(std::io::Error::other)?;
            app.manage(Store(Arc::new(Mutex::new(registry))));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            add_project,
            rename_project,
            remove_project,
            refresh_project,
            set_theme,
            open_project
        ])
        .run(tauri::generate_context!())
        .expect("ContextDock could not start");
}
