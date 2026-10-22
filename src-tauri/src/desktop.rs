//! Native lifecycle and notifications remain active when the WebView is hidden.
use crate::{
    cloud,
    notifications::{self, Delivery, Queue},
    registry::{error, Result},
    review, Store,
};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

pub struct Runtime {
    stop: Arc<AtomicBool>,
    worker: Mutex<Option<thread::JoinHandle<()>>>,
    route: Mutex<Option<Value>>,
    pub tray_ready: AtomicBool,
    pub error: Arc<Mutex<Option<String>>>,
    notification_error: Arc<Mutex<Option<String>>>,
    retry_notifications: AtomicBool,
    ready: AtomicBool,
    pending_close: Mutex<Option<String>>,
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // Explicit Exit joins off the UI thread. OS shutdown cannot wait on
        // window queries whose event loop is already exiting.
    }
}
pub fn uploads_paused(db: &rusqlite::Connection) -> bool {
    review::preference(db, "desktop.uploads_paused", false)
}
pub fn parse_route(url: &str) -> Option<Value> {
    let route = url.strip_prefix("contextdock://")?;
    let (page, query) = route.split_once('?').unwrap_or((route, ""));
    let page = page.trim_end_matches('/');
    if !["review", "project"].contains(&page) {
        return None;
    }
    let project = if query.is_empty() {
        None
    } else {
        Some(query.strip_prefix("project=")?)
    };
    if let Some(id) = project {
        uuid::Uuid::parse_str(id).ok()?;
    }
    if page == "project" && project.is_none() {
        return None;
    }
    Some(json!({"page":page,"project":project}))
}
pub fn show(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}
pub fn navigate(app: &tauri::AppHandle, route: Value) {
    if let Some(r) = app.try_state::<Runtime>() {
        if let Ok(mut pending) = r.route.lock() {
            *pending = Some(route.clone());
        }
        if r.ready.load(Ordering::Relaxed) {
            let _ = app.emit("desktop-route", route);
        }
    }
    show(app);
}
pub fn launch(app: &tauri::AppHandle, args: &[String]) {
    if let Some(route) = args.iter().find_map(|a| parse_route(a)) {
        navigate(app, route);
    } else if !args.iter().any(|a| a == "--background") {
        show(app);
    }
}
pub fn request_close(app: &tauri::AppHandle, action: &str) {
    if app
        .try_state::<Runtime>()
        .is_some_and(|r| r.stop.load(Ordering::Relaxed))
    {
        return;
    }
    show(app);
    if let Some(runtime) = app.try_state::<Runtime>() {
        if !runtime.ready.load(Ordering::Relaxed) {
            if let Ok(mut pending) = runtime.pending_close.lock() {
                *pending = Some(action.into());
            }
            return;
        }
    }
    let _ = app.emit("desktop-close", action);
}
pub fn start(app: &tauri::AppHandle, path: PathBuf) -> std::io::Result<Runtime> {
    let stop = Arc::new(AtomicBool::new(false));
    let error = Arc::new(Mutex::new(None));
    let notification_error = Arc::new(Mutex::new(None));
    let worker_notification_error = notification_error.clone();
    let (worker_stop, worker_error, handle) = (stop.clone(), error.clone(), app.clone());
    let worker = thread::Builder::new()
        .name("contextdock-desktop".into())
        .spawn(move || {
            let Ok(db) = cloud::connect(&path) else {
                return;
            };
            let mut old_pending = String::new();
            let mut notifications = Queue::default();
            while !worker_stop.load(Ordering::Relaxed) {
                if handle
                    .try_state::<Runtime>()
                    .is_some_and(|r| r.retry_notifications.swap(false, Ordering::Relaxed))
                {
                    notifications = Queue::default();
                }
                let result = background_tick(&db, &handle, &mut old_pending, &mut notifications);
                if let Err(e) = result {
                    if let Ok(mut slot) = worker_error.lock() {
                        *slot = Some(e);
                    }
                }
                if let Ok(mut slot) = worker_notification_error.lock() {
                    let current = notifications.error();
                    if *slot != current {
                        *slot = current;
                        let _ = handle.emit("desktop-changed", ());
                    }
                }
                for _ in 0..20 {
                    if worker_stop.load(Ordering::Relaxed) {
                        break;
                    }
                    thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        })?;
    Ok(Runtime {
        stop,
        worker: Mutex::new(Some(worker)),
        route: Mutex::new(None),
        tray_ready: AtomicBool::new(false),
        error,
        notification_error,
        retry_notifications: AtomicBool::new(false),
        ready: AtomicBool::new(false),
        pending_close: Mutex::new(None),
    })
}
pub fn tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open ContextDock", true, None::<&str>)?;
    let pending = MenuItem::with_id(app, "review", "Pending approvals (0)", true, None::<&str>)?;
    let paused = {
        let state = app.state::<Store>();
        let r = state
            .0
            .lock()
            .map_err(|_| tauri::Error::AssetNotFound("Registry unavailable".into()))?;
        uploads_paused(&r.db)
    };
    let pause = MenuItem::with_id(
        app,
        "pause",
        if paused {
            "Resume uploads"
        } else {
            "Pause uploads"
        },
        true,
        None::<&str>,
    )?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let exit = MenuItem::with_id(app, "exit", "Exit ContextDock", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &pending, &pause, &settings, &exit])?;
    let mut builder = TrayIconBuilder::with_id("contextdock")
        .tooltip("ContextDock · Running in background")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show(app),
            "review" => navigate(app, json!({"page":"review","project":null})),
            "settings" => navigate(app, json!({"page":"settings","project":null})),
            "exit" => request_close(app, "exit"),
            "pause" => {
                let store = app.state::<Store>().inner().clone();
                let app = app.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    let result = (|| {
                        let r = store
                            .0
                            .lock()
                            .map_err(|_| "Registry unavailable".to_string())?;
                        review::set_preference(
                            &r.db,
                            "desktop.uploads_paused",
                            !uploads_paused(&r.db),
                        )
                    })();
                    if let Err(e) = result {
                        if let Ok(mut slot) = app.state::<Runtime>().error.lock() {
                            *slot = Some(e);
                        }
                        show(&app);
                    }
                    let _ = app.emit("desktop-changed", ());
                });
            }
            _ => {}
        })
        .on_tray_icon_event(|icon, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show(icon.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    app.manage(TrayItems { pending, pause });
    app.state::<Runtime>()
        .tray_ready
        .store(true, Ordering::Relaxed);
    Ok(())
}
struct TrayItems {
    pending: MenuItem<tauri::Wry>,
    pause: MenuItem<tauri::Wry>,
}
fn foreground(app: &tauri::AppHandle) -> bool {
    app.get_webview_window("main").is_some_and(|w| {
        w.is_visible().unwrap_or(false)
            && !w.is_minimized().unwrap_or(true)
            && w.is_focused().unwrap_or(false)
    })
}
fn background_tick(
    db: &rusqlite::Connection,
    app: &tauri::AppHandle,
    old: &mut String,
    notifications: &mut Queue,
) -> Result<()> {
    let entries = review::pending(db)?;
    let total = entries.len();
    let signature = serde_json::to_string(&entries).map_err(|e| e.to_string())?;
    if signature != *old {
        *old = signature;
        let _ = app.emit("desktop-changed", ());
    }
    let paused = uploads_paused(db);
    if let Some(items) = app.try_state::<TrayItems>() {
        let _ = items
            .pending
            .set_text(format!("Pending approvals ({total})"));
        let _ = items.pause.set_text(if paused {
            "Resume uploads"
        } else {
            "Pause uploads"
        });
    }
    let projects=cloud::rows(db,"SELECT p.id,p.name,p.scan_error,p.tracking_error,c.error cloud_error,c.status cloud_status,(SELECT count(*) FROM cloud_jobs j WHERE j.project_id=p.id AND j.desired=1 AND j.state IN ('failed','uncertain')) failed FROM projects p LEFT JOIN cloud_projects c ON c.project_id=p.id WHERE p.removed_at IS NULL",&[])?;
    let problems = projects
        .iter()
        .filter(|p| {
            !cloud::s(p, "scan_error").is_empty()
                || !cloud::s(p, "tracking_error").is_empty()
                || !cloud::s(p, "cloud_error").is_empty()
                || cloud::n(p, "failed") > 0
        })
        .count();
    if let Some(icon) = app.tray_by_id("contextdock") {
        let _ = icon.set_tooltip(Some(format!(
            "ContextDock · {total} pending · {problems} problems{}",
            if paused { " · Uploads paused" } else { "" }
        )));
    }
    let now = chrono::Utc::now().timestamp();
    let active = foreground(app);
    let mut notice_keys = std::collections::HashSet::new();
    for p in projects {
        let id = cloud::s(&p, "id");
        let files = entries
            .iter()
            .filter(|e| e.project_id == id)
            .collect::<Vec<_>>();
        let key = format!("approval:{id}");
        if files.is_empty() {
            db.execute("DELETE FROM desktop_notices WHERE key=?1", [&key])
                .map_err(error)?;
        } else {
            let signature = files
                .iter()
                .map(|f| f.path.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            let due = review::notice_due(db, &key, &signature, now, 30)?;
            let enabled = review::preference(db, "desktop.approval_notifications", true);
            if enabled {
                notice_keys.insert(key.clone());
            }
            if due
                && !active
                && enabled
                && notifications.attempt(&key, now, || {
                    notifications::send(
                        app,
                        &format!("{}: files need approval", cloud::s(&p, "name")),
                        &format!(
                            "{} new files are waiting. Open ContextDock to review them.",
                            files.len()
                        ),
                        &format!("contextdock://review?project={id}"),
                        &key,
                    )
                })
            {
                review::notice_sent(db, &key, now)?;
            }
        }
        let key = format!("problem:{id}");
        let signature = format!(
            "{}|{}|{}|{}",
            cloud::s(&p, "scan_error"),
            cloud::s(&p, "tracking_error"),
            cloud::s(&p, "cloud_error"),
            cloud::n(&p, "failed")
        );
        if signature == "|||0" {
            db.execute("DELETE FROM desktop_notices WHERE key=?1", [&key])
                .map_err(error)?;
        } else {
            let due = review::notice_due(db, &key, &signature, now, 30)?;
            let enabled = review::preference(db, "desktop.problem_notifications", true);
            if enabled {
                notice_keys.insert(key.clone());
            }
            if due
                && !active
                && enabled
                && notifications.attempt(&key, now, || {
                    notifications::send(
                        app,
                        &format!("{} needs attention", cloud::s(&p, "name")),
                        "Open ContextDock to review a tracking, folder or cloud problem.",
                        &format!("contextdock://project?project={id}"),
                        &key,
                    )
                })
            {
                review::notice_sent(db, &key, now)?;
            }
        }
    }
    notifications.retain(&notice_keys);
    Ok(())
}
#[tauri::command]
pub async fn desktop_test_notification(app: tauri::AppHandle) -> Result<String> {
    let handle = app.clone();
    let delivery = tauri::async_runtime::spawn_blocking(move || {
        notifications::send(
            &handle,
            "ContextDock notifications are ready",
            "This is a test. Click to open Pending approvals. No files were uploaded or approved.",
            "contextdock://review",
            "test",
        )
    })
    .await
    .map_err(|e| e.to_string())??;
    if delivery == Delivery::Disabled {
        return Err("Windows notifications are disabled. Enable ContextDock in Windows Settings > System > Notifications, then try again.".into());
    }
    let runtime = app.state::<Runtime>();
    if let Ok(mut error) = runtime.notification_error.lock() {
        *error = None;
    }
    runtime.retry_notifications.store(true, Ordering::Relaxed);
    let _ = app.emit("desktop-changed", ());
    Ok("Test notification submitted to Windows. Check the banner or Notification Center (Windows + N); Do not disturb may hide the banner.".into())
}
#[tauri::command]
pub async fn desktop_read(state: tauri::State<'_, Store>, app: tauri::AppHandle) -> Result<Value> {
    let startup = crate::startup::status()?;
    let native_error = app
        .state::<Runtime>()
        .error
        .lock()
        .ok()
        .and_then(|e| e.clone());
    let notification_error = app
        .state::<Runtime>()
        .notification_error
        .lock()
        .ok()
        .and_then(|e| e.clone());
    let native_error = native_error.or(notification_error);
    crate::work(state.inner().clone(),move|r|Ok(json!({"keepRunning":review::preference(&r.db,"desktop.keep_running",true),"approvalNotifications":review::preference(&r.db,"desktop.approval_notifications",true),"problemNotifications":review::preference(&r.db,"desktop.problem_notifications",true),"uploadsPaused":uploads_paused(&r.db),"startupRegistered":startup.registered,"startupEnabled":startup.enabled,"pending":review::pending(&r.db)?.len(),"error":native_error}))).await
}
#[tauri::command]
pub async fn desktop_setting(
    state: tauri::State<'_, Store>,
    app: tauri::AppHandle,
    key: String,
    value: bool,
) -> Result<()> {
    if key == "startup" {
        crate::startup::set(value)?;
    } else {
        let key = match key.as_str() {
            "keepRunning" => "desktop.keep_running",
            "approvalNotifications" => "desktop.approval_notifications",
            "problemNotifications" => "desktop.problem_notifications",
            "uploadsPaused" => "desktop.uploads_paused",
            _ => return Err("Unknown desktop setting.".into()),
        }
        .to_string();
        crate::work(state.inner().clone(), move |r| {
            review::set_preference(&r.db, &key, value)
        })
        .await?;
    }
    let _ = app.emit("desktop-changed", ());
    Ok(())
}
#[tauri::command]
pub fn desktop_ready(app: tauri::AppHandle) -> Result<()> {
    let runtime = app.state::<Runtime>();
    runtime.ready.store(true, Ordering::Relaxed);
    if let Some(action) = runtime.pending_close.lock().ok().and_then(|mut p| p.take()) {
        let _ = app.emit("desktop-close", action);
    }
    Ok(())
}
#[tauri::command]
pub fn desktop_route(app: tauri::AppHandle) -> Option<Value> {
    app.state::<Runtime>()
        .route
        .lock()
        .ok()
        .and_then(|mut r| r.take())
}
#[tauri::command]
pub async fn desktop_complete(
    state: tauri::State<'_, Store>,
    app: tauri::AppHandle,
    action: String,
) -> Result<()> {
    if !["close", "exit"].contains(&action.as_str()) {
        return Err("Invalid close action.".into());
    }
    let keep = if action == "exit" {
        false
    } else {
        crate::work(state.inner().clone(), |r| {
            Ok(review::preference(&r.db, "desktop.keep_running", true))
        })
        .await
        .unwrap_or(true)
    };
    if action == "close" && keep && app.state::<Runtime>().tray_ready.load(Ordering::Relaxed) {
        app.get_webview_window("main")
            .ok_or("Window unavailable")?
            .hide()
            .map_err(|e| e.to_string())?;
    } else {
        app.state::<crate::monitor::Monitor>().request_stop();
        let runtime = app.state::<crate::cloud_commands::CloudRuntime>();
        runtime.request_stop();
        let gate = runtime.gate.clone();
        let store = state.inner().clone();
        let desktop = app.state::<Runtime>();
        desktop.stop.store(true, Ordering::Relaxed);
        let worker = desktop
            .worker
            .lock()
            .map_err(|_| "Desktop worker unavailable")?
            .take();
        let _ = app.emit("desktop-stopping", ());
        // Wait off the event thread for the current journaled HTTP/local operation.
        // A stop guard prevents the worker from sending any subsequent request.
        tauri::async_runtime::spawn_blocking(move || {
            let _gate = gate.lock().unwrap_or_else(|e| e.into_inner());
            let _registry = store.0.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(worker) = worker {
                let _ = worker.join();
            }
            Ok::<(), String>(())
        })
        .await
        .map_err(|e| e.to_string())??;
        app.exit(0);
    }
    Ok(())
}
#[tauri::command]
pub async fn pending_read(state: tauri::State<'_, Store>) -> Result<Vec<review::PendingFile>> {
    crate::work(state.inner().clone(), |r| review::pending(&r.db)).await
}
#[tauri::command]
pub async fn pending_preview(
    state: tauri::State<'_, Store>,
    request: review::ReviewRequest,
) -> Result<Value> {
    crate::work(state.inner().clone(), move |r| review::preview(r, &request)).await
}
#[tauri::command]
pub async fn pending_apply(
    state: tauri::State<'_, Store>,
    runtime: tauri::State<'_, crate::cloud_commands::CloudRuntime>,
    app: tauri::AppHandle,
    request: review::ReviewRequest,
    token: String,
) -> Result<()> {
    let store = state.inner().clone();
    let gate = runtime.gate.clone();
    let project = request.project.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _gate = gate.lock().map_err(|_| "Cloud worker unavailable")?;
        let mut r = store.0.lock().map_err(|_| "Registry unavailable")?;
        review::apply(&mut r, &request, &token)
    })
    .await
    .map_err(|e| e.to_string())??;
    let _ = app.emit("registry-changed", &project);
    let _ = app.emit("cloud-changed", &project);
    let _ = app.emit("desktop-changed", ());
    Ok(())
}
