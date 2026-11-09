//! A separate compiled pilot cannot open the installed profile or its Drive vault.
use std::path::PathBuf;
#[cfg(not(feature = "pilot"))]
use tauri::Manager;

pub fn data_dir(app: &tauri::App) -> std::io::Result<PathBuf> {
    #[cfg(feature = "pilot")]
    {
        if app.config().identifier != "com.contextdock.pilot" {
            return Err(std::io::Error::other(
                "Build the pilot with tauri.pilot.conf.json. The installed profile was not opened.",
            ));
        }
        pilot_data_dir()
    }
    #[cfg(not(feature = "pilot"))]
    {
        app.path()
            .app_local_data_dir()
            .map_err(std::io::Error::other)
    }
}

#[cfg(all(test, feature = "pilot"))]
#[test]
#[ignore = "Explicit isolated native-app pilot seeding; requires a marked pilot directory"]
fn seed_isolated_pilot() {
    let data = pilot_data_dir().unwrap();
    assert!(
        !data.exists(),
        "Use a new pilot directory; existing data is never overwritten."
    );
    let root = data.parent().unwrap().join("Project");
    std::fs::create_dir_all(&data).unwrap();
    std::fs::create_dir(&root).unwrap();
    for n in 1..=2 {
        std::fs::write(
            root.join(format!("ContextDock_Context_v{n}.md")),
            format!("# ContextDock pilot\n\nDisposable Markdown marker {n}.\n"),
        )
        .unwrap();
        std::fs::write(
            root.join(format!("ContextDock_Notes_v{n}.docx")),
            crate::drive_publish_tests::sample_docx(&format!("Disposable Word marker {n}.")),
        )
        .unwrap();
    }
    let mut r = crate::registry::Registry::open(&data.join("contextdock.db")).unwrap();
    let id = r.add("ContextDock Pilot", root.to_str().unwrap()).unwrap();
    r.configure_tracking(&id, true, false, vec![]).unwrap();
    for key in [
        "desktop.approval_notifications",
        "desktop.problem_notifications",
    ] {
        crate::review::set_preference(&r.db, key, false).unwrap();
    }
    assert_eq!(
        crate::drive_publish::eligible_documents(&r.db, &id)
            .unwrap()
            .len(),
        2
    );
    println!("Pilot seeded at {}", data.display());
}

#[cfg(feature = "pilot")]
pub(crate) fn pilot_data_dir() -> std::io::Result<PathBuf> {
    let path = PathBuf::from(std::env::var_os("CONTEXTDOCK_PILOT_DIR").ok_or_else(|| {
        std::io::Error::other("Set CONTEXTDOCK_PILOT_DIR to an isolated absolute directory.")
    })?);
    if !path.is_absolute() {
        return Err(std::io::Error::other(
            "The pilot directory must be absolute.",
        ));
    }
    // A marker prevents an accidentally supplied existing app-data folder.
    if !path.join("ContextDock_Pilot_v1.txt").is_file() {
        return Err(std::io::Error::other(
            "The isolated pilot directory needs its ContextDock_Pilot_v1.txt marker.",
        ));
    }
    Ok(path.join("Data"))
}
