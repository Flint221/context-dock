mod ai;
mod ai_commands;
mod ai_inference;
mod ai_models;
#[cfg(test)]
mod ai_tests;
mod archival;
#[cfg(test)]
mod archival_tests;
mod chat;
mod chat_commands;
mod chat_stream;
#[cfg(test)]
mod chat_tests;
mod cloud;
mod cloud_api;
mod cloud_cleanup;
mod cloud_commands;
mod cloud_credentials;
mod cloud_models;
mod cloud_sync;
#[cfg(test)]
mod cloud_tests;
mod comparison;
mod comparison_models;
#[cfg(test)]
mod comparison_tests;
mod desktop;
#[cfg(test)]
mod desktop_tests;
mod extraction;
mod folder_recovery;
#[cfg(test)]
mod folder_recovery_tests;
mod forget;
#[cfg(test)]
mod forget_tests;
mod initial_import;
mod matching;
mod models;
mod monitor;
mod notifications;
mod operations;
mod path_selection;
#[cfg(test)]
mod path_selection_tests;
mod processes;
mod registry;
mod review;
mod scanner;
mod startup;
mod storage;
#[cfg(test)]
mod tests;
mod tracking;
mod tracking_models;
mod tracking_queries;
#[cfg(test)]
mod tracking_tests;
mod versioning;
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
async fn preview_project_folder(
    state: tauri::State<'_, Store>,
    id: String,
    path: String,
) -> Result<folder_recovery::FolderPlan, String> {
    work(state.inner().clone(), move |r| r.folder_plan(&id, &path)).await
}
#[tauri::command]
async fn reassign_project_folder(
    state: tauri::State<'_, Store>,
    id: String,
    path: String,
    token: String,
) -> Result<(), String> {
    work(state.inner().clone(), move |r| {
        r.reassign_folder(&id, &path, &token)
    })
    .await
}
#[tauri::command]
async fn remove_project(
    state: tauri::State<'_, Store>,
    id: String,
    retain_cloud: bool,
) -> Result<(), String> {
    work(state.inner().clone(), move |r| {
        r.remove_with_cloud_choice(&id, retain_cloud)
    })
    .await
}
#[tauri::command]
async fn refresh_project_folders(state: tauri::State<'_, Store>) -> Result<(), String> {
    work(state.inner().clone(), |r| r.refresh_folders()).await
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
    runtime: tauri::State<'_, cloud_commands::CloudRuntime>,
    project: String,
    enabled: bool,
    archive: bool,
    selected: Vec<String>,
) -> Result<(), String> {
    let store = state.inner().clone();
    let gate = runtime.gate.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _gate = gate.lock().map_err(|_| "Cloud worker is unavailable.")?;
        let mut registry = store.0.lock().map_err(|_| "Registry is unavailable.")?;
        registry.configure_tracking(&project, enabled, archive, selected)
    })
    .await
    .map_err(|e| format!("Tracking settings failed: {e}"))?
}
#[tauri::command]
async fn archival_review(
    state: tauri::State<'_, Store>,
    project: String,
    enabled: bool,
) -> Result<archival::Review, String> {
    work(state.inner().clone(), move |r| {
        r.archival_review(&project, enabled)
    })
    .await
}
#[tauri::command]
async fn apply_archival_review(
    state: tauri::State<'_, Store>,
    runtime: tauri::State<'_, cloud_commands::CloudRuntime>,
    app: tauri::AppHandle,
    project: String,
    enabled: bool,
    token: String,
) -> Result<(), String> {
    let store = state.inner().clone();
    let gate = runtime.gate.clone();
    let id = project.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _gate = gate.lock().map_err(|_| "Cloud worker is unavailable.")?;
        let mut r = store.0.lock().map_err(|_| "Registry is unavailable.")?;
        r.apply_archival(&id, enabled, &token)
    })
    .await
    .map_err(|e| format!("Archival action failed: {e}"))??;
    let _ = app.emit("registry-changed", &project);
    Ok(())
}
#[tauri::command]
async fn resolve_match(
    state: tauri::State<'_, Store>,
    runtime: tauri::State<'_, cloud_commands::CloudRuntime>,
    project: String,
    source: String,
    action: String,
    target: Option<String>,
) -> Result<(), String> {
    let store = state.inner().clone();
    let gate = runtime.gate.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _gate = gate.lock().map_err(|_| "Cloud worker is unavailable.")?;
        let mut registry = store.0.lock().map_err(|_| "Registry is unavailable.")?;
        registry.resolve_match(&project, &source, &action, target)
    })
    .await
    .map_err(|e| format!("Document selection failed: {e}"))?
}
#[tauri::command]
async fn document_versions(
    state: tauri::State<'_, Store>,
    document: String,
) -> Result<Vec<tracking_models::Version>, String> {
    work(state.inner().clone(), move |r| {
        r.published_versions(&document)
    })
    .await
}
#[tauri::command]
async fn compare_versions(
    state: tauri::State<'_, Store>,
    earlier: String,
    later: String,
) -> Result<comparison_models::Comparison, String> {
    use std::sync::atomic::{AtomicBool, Ordering};
    static RUNNING: AtomicBool = AtomicBool::new(false);
    if RUNNING.swap(true, Ordering::AcqRel) {
        return Err("A comparison is already running. Please retry when it finishes.".into());
    }
    struct Permit;
    impl Drop for Permit {
        fn drop(&mut self) {
            RUNNING.store(false, Ordering::Release);
        }
    }
    let permit = Permit;
    let job = work(state.inner().clone(), move |r| {
        r.comparison_job(&earlier, &later)
    })
    .await?;
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        comparison::compare(job)
    })
    .await
    .map_err(|e| format!("Comparison worker failed: {e}"))?
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
#[tauri::command]
async fn path_selection_view(
    state: tauri::State<'_, Store>,
    project: String,
) -> Result<path_selection::View, String> {
    work(state.inner().clone(), move |r| r.path_selection(&project)).await
}
#[tauri::command]
async fn save_path_selection(
    state: tauri::State<'_, Store>,
    runtime: tauri::State<'_, cloud_commands::CloudRuntime>,
    app: tauri::AppHandle,
    project: String,
    revision: i64,
    exclusions: Vec<path_selection::Rule>,
) -> Result<(), String> {
    let gate = runtime.gate.clone();
    let store = state.inner().clone();
    let id = project.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = gate.lock().map_err(|_| "Cloud worker is unavailable.")?;
        let mut r = store.0.lock().map_err(|_| "Registry is unavailable.")?;
        r.save_path_selection(&id, revision, exclusions)?;
        // Reconcile admitted sources only. An error after persistence is shown
        // by the tracking panel; it must not turn a saved draft into an unsaved one.
        if let Err(e) = r.reconcile(
            &id,
            "path_selection",
            &std::collections::HashSet::new(),
            true,
        ) {
            let _ = r.set_tracking_error(&id, &[e]);
        }
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| format!("Folder selection failed: {e}"))??;
    let _ = app.emit("registry-changed", &project);
    let _ = app.emit("cloud-changed", &project);
    Ok(())
}
#[tauri::command]
async fn preview_forget_missing(
    state: tauri::State<'_, Store>,
    project: String,
    path: String,
) -> Result<forget::Plan, String> {
    work(state.inner().clone(), move |r| {
        r.preview_forget(&project, &path)
    })
    .await
}
#[tauri::command]
async fn forget_missing_file(
    state: tauri::State<'_, Store>,
    runtime: tauri::State<'_, cloud_commands::CloudRuntime>,
    app: tauri::AppHandle,
    project: String,
    path: String,
    token: String,
) -> Result<Vec<String>, String> {
    let gate = runtime.gate.clone();
    let store = state.inner().clone();
    let id = project.clone();
    let warnings = tauri::async_runtime::spawn_blocking(move || {
        let _guard = gate.lock().map_err(|_| "Cloud worker is unavailable.")?;
        let mut r = store.0.lock().map_err(|_| "Registry is unavailable.")?;
        let mut warnings = r.forget_missing(&id, &path, &token)?;
        if let Err(e) = r.reconcile(
            &id,
            "forget_missing",
            &std::collections::HashSet::new(),
            true,
        ) {
            let _ = r.set_tracking_error(&id, std::slice::from_ref(&e));
            warnings.push(format!(
                "Removal saved, but tracking refresh needs attention: {e}"
            ));
        }
        Ok::<Vec<String>, String>(warnings)
    })
    .await
    .map_err(|e| e.to_string())??;
    let _ = app.emit("registry-changed", &project);
    let _ = app.emit("cloud-changed", &project);
    let _ = app.emit("desktop-changed", ());
    Ok(warnings)
}
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            desktop::launch(app, &args)
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                desktop::request_close(window.app_handle(), "close");
            }
            if matches!(event, tauri::WindowEvent::Focused(true)) {
                let _ = window.app_handle().emit("desktop-changed", ());
            }
        })
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
            app.manage(
                ai_commands::Runtime::new(dir.join("contextdock.db"))
                    .map_err(std::io::Error::other)?,
            );
            app.manage(cloud_commands::CloudRuntime::start(
                dir.join("contextdock.db"),
                app.handle().clone(),
            )?);
            let handle = app.handle().clone();
            let monitor = monitor::Monitor::start(registry, move |project| {
                let _ = handle.emit("registry-changed", project);
            })?;
            app.manage(monitor);
            app.manage(desktop::start(app.handle(), dir.join("contextdock.db"))?);
            if let Err(e) = desktop::tray(app.handle()) {
                if let Ok(mut slot) = app.state::<desktop::Runtime>().error.lock() {
                    *slot = Some(format!(
                        "System tray unavailable; the window will remain accessible. {e}"
                    ));
                }
                desktop::show(app.handle());
            }
            let args = std::env::args().collect::<Vec<_>>();
            desktop::launch(app.handle(), &args);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            desktop::desktop_read,
            desktop::desktop_setting,
            desktop::desktop_test_notification,
            desktop::desktop_ready,
            desktop::desktop_route,
            desktop::desktop_complete,
            desktop::pending_read,
            desktop::pending_preview,
            desktop::pending_apply,
            ai_commands::ai_action,
            chat_commands::chat_action,
            cloud_commands::cloud_read,
            cloud_commands::cloud_link,
            cloud_commands::cloud_key,
            cloud_commands::cloud_action,
            snapshot,
            path_selection_view,
            save_path_selection,
            preview_forget_missing,
            forget_missing_file,
            add_project,
            rename_project,
            preview_project_folder,
            reassign_project_folder,
            remove_project,
            refresh_project,
            refresh_project_folders,
            set_theme,
            open_project,
            tracking_view,
            configure_tracking,
            archival_review,
            apply_archival_review,
            resolve_match,
            document_versions,
            compare_versions,
            open_version_location,
            open_current_location,
            retry_operations,
            processes::process_inventory,
            processes::stop_processes,
            processes::preview_process,
            processes::process_startup_items,
            processes::open_startup_settings
        ])
        .run(tauri::generate_context!())
        .expect("ContextDock could not start");
}
