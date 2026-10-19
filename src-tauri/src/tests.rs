use crate::registry::Registry;
use std::fs;
use tempfile::tempdir;
#[test]
fn migrations_persistence_rename_duplicate_and_safe_removal() {
    let temp = tempdir().unwrap();
    let folder = temp.path().join("Project");
    fs::create_dir(&folder).unwrap();
    fs::write(folder.join("a.md"), "valuable content").unwrap();
    let db = temp.path().join("registry.db");
    let id;
    {
        let mut r = Registry::open(&db).unwrap();
        id = r.add(" Custom name ", folder.to_str().unwrap()).unwrap();
        assert!(r
            .add("Duplicate", folder.join(".").to_str().unwrap())
            .is_err());
        r.rename(&id, "Renamed").unwrap();
        r.theme("dark").unwrap();
        assert!(r.rename(&id, " ").is_err());
    }
    let r = Registry::open(&db).unwrap();
    let s = r.snapshot().unwrap();
    assert_eq!(s.projects[0].name, "Renamed");
    assert_eq!(s.theme, "dark");
    assert_eq!(s.documents.len(), 1);
    assert_eq!(s.documents[0].project_id, id);
    assert_eq!(
        r.db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    r.remove(&id).unwrap();
    assert!(r.snapshot().unwrap().documents.is_empty());
    assert_eq!(
        fs::read_to_string(folder.join("a.md")).unwrap(),
        "valuable content"
    );
}
#[test]
fn recursive_discovery_identity_exclusions_and_read_only() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("root");
    fs::create_dir(&root).unwrap();
    for dir in [
        "nested",
        "other",
        ".git",
        "node_modules",
        "dist",
        "build",
        "target",
        ".contextdock",
        ".contextdock-archive",
        "contextdock-archive",
    ] {
        fs::create_dir(root.join(dir)).unwrap();
        fs::write(root.join(dir).join("same.md"), "unchanged").unwrap();
    }
    for file in ["one.md", "two.MARKDOWN", "nested/spec.DOCX", "ignored.txt"] {
        fs::write(root.join(file), "bytes").unwrap();
    }
    let before = fs::metadata(root.join("one.md"))
        .unwrap()
        .modified()
        .unwrap();
    let mut r = Registry::open(&temp.path().join("r.db")).unwrap();
    let id = r.add("Project", root.to_str().unwrap()).unwrap();
    let first = r.snapshot().unwrap();
    assert_eq!(first.documents.len(), 5);
    let ids: Vec<_> = first.documents.iter().map(|d| d.id.clone()).collect();
    r.refresh(&id).unwrap();
    let second = r.snapshot().unwrap();
    assert_eq!(
        ids,
        second
            .documents
            .iter()
            .map(|d| d.id.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        fs::metadata(root.join("one.md"))
            .unwrap()
            .modified()
            .unwrap(),
        before
    );
    assert_eq!(fs::read(root.join("one.md")).unwrap(), b"bytes");
    fs::remove_file(root.join("one.md")).unwrap();
    r.refresh(&id).unwrap();
    assert_eq!(
        r.snapshot()
            .unwrap()
            .documents
            .iter()
            .find(|d| d.name == "one.md")
            .unwrap()
            .status,
        "missing"
    );
    fs::rename(&root, temp.path().join("disconnected")).unwrap();
    r.refresh(&id).unwrap();
    let s = r.snapshot().unwrap();
    assert_eq!(s.projects[0].status, "unavailable");
    assert_eq!(s.documents.len(), 5);
    assert!(s.documents.iter().all(|d| d.status == "unavailable"));
    fs::rename(temp.path().join("disconnected"), &root).unwrap();
    r.refresh(&id).unwrap();
    assert_eq!(r.snapshot().unwrap().projects[0].status, "available");
}
#[test]
fn invalid_paths_and_names_are_rejected() {
    let t = tempdir().unwrap();
    let mut r = Registry::open(&t.path().join("r.db")).unwrap();
    assert!(r.add("Test", "relative").is_err());
    assert!(r
        .add("Test", t.path().join("missing").to_str().unwrap())
        .is_err());
    let file = t.path().join("file");
    fs::write(&file, "hi").unwrap();
    assert!(r.add("Test", file.to_str().unwrap()).is_err());
    assert!(r.add("", t.path().to_str().unwrap()).is_err());
    assert!(r.theme("bad").is_err());
    assert!(r.refresh("unknown").is_err());
}
#[cfg(windows)]
#[test]
fn windows_case_and_junction_safety() {
    let t = tempdir().unwrap();
    let root = t.path().join("Project");
    fs::create_dir(&root).unwrap();
    let mut r = Registry::open(&t.path().join("r.db")).unwrap();
    r.add("Project", root.to_str().unwrap()).unwrap();
    assert!(r
        .add("Duplicate", &root.to_str().unwrap().to_uppercase())
        .is_err());
    // Junction creation needs no symlink privilege. A junction into an unrelated folder must be skipped.
    let external = t.path().join("outside");
    fs::create_dir(&external).unwrap();
    fs::write(external.join("secret.md"), "private").unwrap();
    let status = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(root.join("junction"))
        .arg(&external)
        .status()
        .unwrap();
    assert!(status.success());
    let id = r.snapshot().unwrap().projects[0].id.clone();
    r.refresh(&id).unwrap();
    assert!(r.snapshot().unwrap().documents.is_empty());
    fs::remove_dir(root.join("junction")).unwrap();
}
