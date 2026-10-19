//! App-lifetime watcher ownership; no service and no watcher survives disabling a project.
use crate::{models, registry::Registry, scanner};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender, TrySendError},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
type Shared = Arc<Mutex<Registry>>;
enum Message {
    Dirty(String, Vec<PathBuf>),
    Error(String, String),
}
struct Pending {
    first: Instant,
    last: Instant,
    not_before: Instant,
    paths: HashSet<String>,
    full: bool,
    attempt: u8,
}
impl Pending {
    fn new(full: bool) -> Self {
        let now = Instant::now();
        Self {
            first: now,
            last: now,
            not_before: now,
            paths: HashSet::new(),
            full,
            attempt: 0,
        }
    }
}
pub struct Monitor {
    stop: Arc<AtomicBool>,
    _thread: Option<thread::JoinHandle<()>>,
}
impl Drop for Monitor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}
impl Monitor {
    pub fn start(
        registry: Shared,
        changed: impl Fn(&str) + Send + Sync + 'static,
    ) -> std::io::Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let changed = Arc::new(changed);
        let handle = thread::Builder::new()
            .name("contextdock-tracking".into())
            .spawn(move || run(registry, flag, changed))?;
        Ok(Self {
            stop,
            _thread: Some(handle),
        })
    }
    #[cfg(test)]
    pub fn shutdown(mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(t) = self._thread.take() {
            t.join().unwrap();
        }
    }
}
fn relevant(root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    !relative.components().any(|c| {
        c.as_os_str()
            .to_str()
            .is_some_and(scanner::ignored_directory)
    })
}
fn watcher(
    id: String,
    root: PathBuf,
    tx: SyncSender<Message>,
    overflow: Arc<AtomicBool>,
) -> notify::Result<RecommendedWatcher> {
    let filter = root.clone();
    let project = id.clone();
    let mut watch = RecommendedWatcher::new(
        move |result: notify::Result<Event>| {
            let message = match result {
                Ok(event) => {
                    if matches!(event.kind, EventKind::Access(_)) {
                        return;
                    }
                    let paths: Vec<_> = event
                        .paths
                        .into_iter()
                        .filter(|p| relevant(&filter, p))
                        .collect();
                    if paths.is_empty() {
                        return;
                    }
                    Message::Dirty(project.clone(), paths)
                }
                Err(e) => Message::Error(
                    project.clone(),
                    format!("Watcher error: {e}. Manual reconciliation remains available."),
                ),
            };
            if let Err(TrySendError::Full(_)) = tx.try_send(message) {
                overflow.store(true, Ordering::Release);
            }
        },
        Config::default().with_follow_symlinks(false),
    )?;
    watch.watch(&root, RecursiveMode::Recursive)?;
    Ok(watch)
}
fn run(registry: Shared, stop: Arc<AtomicBool>, changed: Arc<dyn Fn(&str) + Send + Sync>) {
    let (tx, rx) = mpsc::sync_channel(512);
    let overflow = Arc::new(AtomicBool::new(false));
    let mut watches: HashMap<String, RecommendedWatcher> = HashMap::new();
    let mut roots: HashMap<String, PathBuf> = HashMap::new();
    let mut pending: HashMap<String, Pending> = HashMap::new();
    let mut retry_watch: HashMap<String, Instant> = HashMap::new();
    let mut next_sync = Instant::now();
    while !stop.load(Ordering::Acquire) {
        if Instant::now() >= next_sync {
            if let Ok(mut r) = registry.try_lock() {
                if let Ok(configs) = r.enabled_projects() {
                    let enabled: HashSet<_> = configs.iter().map(|(id, _)| id.clone()).collect();
                    watches.retain(|id, _| enabled.contains(id));
                    roots.retain(|id, _| enabled.contains(id));
                    pending.retain(|id, _| enabled.contains(id));
                    retry_watch.retain(|id, _| enabled.contains(id));
                    for (id, path) in configs {
                        let root = PathBuf::from(path);
                        roots.insert(id.clone(), root.clone());
                        if watches.contains_key(&id) && !root.is_dir() {
                            watches.remove(&id);
                            retry_watch.remove(&id);
                        }
                        if !watches.contains_key(&id)
                            && retry_watch.get(&id).is_none_or(|t| Instant::now() >= *t)
                        {
                            match watcher(id.clone(), root, tx.clone(), overflow.clone()) {
                                Ok(w) => {
                                    watches.insert(id.clone(), w);
                                    retry_watch.remove(&id);
                                    let _=r.watch_status(&id,"watching",Some("Native monitoring active while ContextDock is running."));
                                    let mut p = Pending::new(true);
                                    p.first -= Duration::from_secs(4);
                                    p.last -= Duration::from_secs(2);
                                    pending.insert(id.clone(), p);
                                }
                                Err(e) => {
                                    let message=format!("Cannot watch folder: {e}. Reconnect the drive or use manual reconciliation. Retrying in 30 seconds.");
                                    let status = if roots[&id].is_dir() {
                                        "error"
                                    } else {
                                        "unavailable"
                                    };
                                    let _ = r.watch_status(&id, status, Some(&message));
                                    if status == "unavailable" {
                                        let _ = r.refresh(&id);
                                    }
                                    retry_watch.insert(
                                        id.clone(),
                                        Instant::now() + Duration::from_secs(30),
                                    );
                                }
                            }
                            changed(&id);
                        }
                    }
                }
            }
            next_sync = Instant::now() + Duration::from_millis(500);
        }
        if overflow.swap(false, Ordering::AcqRel) {
            for id in roots.keys() {
                pending.insert(id.clone(), Pending::new(true));
            }
        }
        let mut messages = vec![];
        if let Ok(m) = rx.recv_timeout(Duration::from_millis(200)) {
            messages.push(m);
        }
        messages.extend(rx.try_iter().take(511));
        for message in messages {
            match message {
                Message::Dirty(id, paths) => {
                    if let Some(root) = roots.get(&id) {
                        let p = pending.entry(id).or_insert_with(|| Pending::new(false));
                        p.last = Instant::now();
                        p.attempt = 0;
                        for path in paths {
                            if let Ok(relative) = path.strip_prefix(root) {
                                if p.paths.len() < 2048 {
                                    p.paths.insert(models::key(&relative.to_string_lossy()));
                                } else {
                                    p.full = true;
                                }
                            }
                        }
                    }
                }
                Message::Error(id, message) => {
                    watches.remove(&id);
                    retry_watch.insert(id.clone(), Instant::now() + Duration::from_secs(30));
                    if let Ok(r) = registry.try_lock() {
                        let _ = r.watch_status(&id, "error", Some(&message));
                    }
                    changed(&id);
                }
            }
        }
        let due: Vec<String> = pending
            .iter()
            .filter(|(_, p)| {
                Instant::now() >= p.not_before
                    && (p.last.elapsed() >= Duration::from_millis(950)
                        || p.first.elapsed() >= Duration::from_secs(4))
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in due {
            if stop.load(Ordering::Acquire) {
                break;
            }
            let Ok(mut r) = registry.try_lock() else {
                continue;
            };
            let Some(mut p) = pending.remove(&id) else {
                continue;
            };
            if !r.tracking_flags(&id).is_ok_and(|flags| flags.0) {
                continue;
            }
            let reason = if p.full {
                "startup_reconciliation"
            } else {
                "watcher"
            };
            let retry = match r.reconcile(&id, reason, &p.paths, p.full) {
                Ok(retry) => retry,
                Err(e) => {
                    let _ = r.activity(&id, "reconcile_failed", &e);
                    let _ = r.set_tracking_error(&id, &[e]);
                    true
                }
            };
            drop(r);
            changed(&id);
            if retry && p.attempt < 3 {
                p.attempt += 1;
                p.not_before =
                    Instant::now() + Duration::from_secs([0, 2, 5, 15][p.attempt as usize]);
                pending.insert(id, p);
            }
        }
    }
}
