use crate::{registry::Registry, storage};
use std::{collections::HashSet, fs, path::PathBuf};
use tempfile::{tempdir, TempDir};

struct Fixture {
    _temp: TempDir,
    root: PathBuf,
    r: Registry,
    project: String,
    document: String,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempdir().unwrap();
        let root = temp.path().join("project");
        let app = temp.path().join("app");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&app).unwrap();
        for (name, bytes) in [
            ("RAP_Master_Context_v4.md", "four"),
            ("RAP_Master_Context_v5.md", "five"),
            ("RAP_Master_Context_v6.md", "six"),
        ] {
            fs::write(root.join(name), bytes).unwrap();
        }
        let mut r = Registry::open(&app.join("registry.db")).unwrap();
        let project = r.add("Test", root.to_str().unwrap()).unwrap();
        let ids = r.source_ids(&project).unwrap();
        r.configure_tracking(&project, true, false, ids).unwrap();
        let document = r
            .tracking_view(&project)
            .unwrap()
            .families
            .into_iter()
            .find(|f| f.version_count > 0)
            .unwrap()
            .id;
        Self {
            _temp: temp,
            root,
            r,
            project,
            document,
        }
    }
    fn apply(&mut self, enabled: bool) {
        let review = self.r.archival_review(&self.project, enabled).unwrap();
        self.r
            .apply_archival(&self.project, enabled, &review.token)
            .unwrap();
    }
    fn archive(&self, name: &str) -> PathBuf {
        PathBuf::from(
            self.r
                .versions(&self.document)
                .unwrap()
                .into_iter()
                .find(|v| v.source_name == name)
                .unwrap()
                .archive_path
                .unwrap(),
        )
    }
    fn scan(&mut self) {
        self.r
            .reconcile(&self.project, "manual_refresh", &HashSet::new(), true)
            .unwrap();
    }
}

#[test]
fn archive_checkbox_round_trip_organizes_old_exports_and_restores_original_locations() {
    let mut f = Fixture::new();
    let versions =
        f.r.versions(&f.document)
            .unwrap()
            .into_iter()
            .map(|v| (v.id, v.number, v.sha256, v.snapshot_path))
            .collect::<Vec<_>>();
    let review = f.r.archival_review(&f.project, true).unwrap();
    assert_eq!(review.items.len(), 2);
    assert!(review.items.iter().all(|i| i.problem.is_none()));
    for item in &review.items {
        assert_eq!(
            item.to,
            format!("Old Context Files/RAP Master Context/{}", item.from)
        );
    }
    assert!(f.root.join("RAP_Master_Context_v4.md").exists());
    assert!(!f.root.join("Old Context Files").exists());
    for _ in 0..3 {
        f.apply(true);
        for (name, bytes) in [
            ("RAP_Master_Context_v4.md", "four"),
            ("RAP_Master_Context_v5.md", "five"),
        ] {
            assert!(!f.root.join(name).exists());
            assert_eq!(fs::read_to_string(f.archive(name)).unwrap(), bytes);
        }
        assert_eq!(
            fs::read_to_string(f.root.join("RAP_Master_Context_v6.md")).unwrap(),
            "six"
        );
        let review = f.r.archival_review(&f.project, false).unwrap();
        assert_eq!(review.items.len(), 2);
        assert!(review.items.iter().all(|i| i.problem.is_none()));
        f.apply(false);
        f.scan();
        for (name, bytes) in [
            ("RAP_Master_Context_v4.md", "four"),
            ("RAP_Master_Context_v5.md", "five"),
        ] {
            assert_eq!(fs::read_to_string(f.root.join(name)).unwrap(), bytes);
        }
        assert!(f.r.tracking_view(&f.project).unwrap().operations.is_empty());
        assert_eq!(
            f.r.versions(&f.document)
                .unwrap()
                .into_iter()
                .map(|v| (v.id, v.number, v.sha256, v.snapshot_path))
                .collect::<Vec<_>>(),
            versions
        );
    }
}

#[test]
fn restore_conflicts_keep_both_files_and_retry_after_restart_without_reenabling_archival() {
    let mut f = Fixture::new();
    f.apply(true);
    let archive = f.archive("RAP_Master_Context_v4.md");
    fs::write(f.root.join("RAP_Master_Context_v4.md"), "user replacement").unwrap();
    let review = f.r.archival_review(&f.project, false).unwrap();
    assert!(review.items.iter().any(|i| i
        .problem
        .as_deref()
        .is_some_and(|p| p.contains("already contains"))));
    f.apply(false);
    assert!(!f.r.tracking_view(&f.project).unwrap().archive_enabled);
    assert_eq!(
        fs::read_to_string(f.root.join("RAP_Master_Context_v4.md")).unwrap(),
        "user replacement"
    );
    assert_eq!(fs::read_to_string(&archive).unwrap(), "four");
    assert!(f
        .r
        .archival_review(&f.project, true)
        .unwrap_err()
        .contains("pending file restores"));
    fs::rename(
        f.root.join("RAP_Master_Context_v4.md"),
        f.root.join("Retained_user_replacement.md"),
    )
    .unwrap();
    let db = f.r.store.join("registry.db");
    f.r = Registry::open(&db).unwrap();
    f.scan();
    assert_eq!(
        fs::read_to_string(f.root.join("RAP_Master_Context_v4.md")).unwrap(),
        "four"
    );
    assert!(!archive.exists());
    assert_eq!(
        fs::read_to_string(f.root.join("Retained_user_replacement.md")).unwrap(),
        "user replacement"
    );
    assert!(f.r.tracking_view(&f.project).unwrap().operations.is_empty());
    assert!(!f.r.tracking_view(&f.project).unwrap().archive_enabled);
}

#[test]
fn stale_archive_review_and_changed_archive_identity_never_start_unreviewed_moves() {
    let mut f = Fixture::new();
    let review = f.r.archival_review(&f.project, true).unwrap();
    fs::write(f.root.join("RAP_Master_Context_v4.md"), "changed").unwrap();
    assert!(f
        .r
        .apply_archival(&f.project, true, &review.token)
        .unwrap_err()
        .contains("review changed"));
    assert!(!f.root.join("Old Context Files").exists());
    fs::write(f.root.join("RAP_Master_Context_v4.md"), "four").unwrap();
    f.apply(true);
    let archive = f.archive("RAP_Master_Context_v4.md");
    let actual = archive.with_extension("retained");
    fs::rename(&archive, &actual).unwrap();
    fs::copy(&actual, &archive).unwrap();
    f.apply(false);
    assert!(archive.exists());
    assert!(!f.root.join("RAP_Master_Context_v4.md").exists());
    assert!(f.r.tracking_view(&f.project).unwrap().error.is_some());
    fs::remove_file(&archive).unwrap();
    fs::rename(&actual, &archive).unwrap();
    f.scan();
    assert_eq!(
        fs::read_to_string(f.root.join("RAP_Master_Context_v4.md")).unwrap(),
        "four"
    );
}

#[test]
fn restore_recovers_a_crash_after_rename_before_metadata_commit() {
    let mut f = Fixture::new();
    f.apply(true);
    f.r.db.execute_batch("CREATE TRIGGER fail_restore BEFORE UPDATE OF state ON operations WHEN NEW.kind='restore' AND NEW.state='done' BEGIN SELECT RAISE(FAIL,'injected restore failure'); END;").unwrap();
    f.apply(false);
    assert!(f.root.join("RAP_Master_Context_v4.md").exists());
    assert!(f.r.tracking_view(&f.project).unwrap().error.is_some());
    f.r.db.execute_batch("DROP TRIGGER fail_restore;").unwrap();
    let db = f.r.store.join("registry.db");
    f.r = Registry::open(&db).unwrap();
    f.scan();
    assert!(f.r.tracking_view(&f.project).unwrap().operations.is_empty());
    assert_eq!(f.r.versions(&f.document).unwrap().len(), 3);
    assert_eq!(
        fs::read_to_string(f.root.join("RAP_Master_Context_v4.md")).unwrap(),
        "four"
    );
}

#[test]
fn snapshot_copy_archives_remain_on_opt_out_without_overwriting_the_current_file() {
    let mut f = Fixture::new();
    f.apply(true);
    fs::write(f.root.join("RAP_Master_Context_v6.md"), "edited six").unwrap();
    f.scan();
    let copy =
        f.r.versions(&f.document)
            .unwrap()
            .into_iter()
            .find(|v| v.source_name == "RAP_Master_Context_v6.md" && !v.current)
            .unwrap()
            .archive_path
            .unwrap();
    let review = f.r.archival_review(&f.project, false).unwrap();
    assert!(review.warnings.iter().any(|w| w.contains("snapshot-copy")));
    f.apply(false);
    assert_eq!(fs::read_to_string(copy).unwrap(), "six");
    assert_eq!(
        fs::read_to_string(f.root.join("RAP_Master_Context_v6.md")).unwrap(),
        "edited six"
    );
    assert_eq!(f.r.versions(&f.document).unwrap().len(), 4);
}

#[test]
fn initial_archive_consent_finishes_import_before_organizing_and_exclusions_stay_in_place() {
    let mut f = Fixture::new();
    let revision = crate::path_selection::revision(&f.r.db, &f.project).unwrap();
    f.r.save_path_selection(
        &f.project,
        revision,
        vec![crate::path_selection::Rule {
            path: "RAP_Master_Context_v4.md".into(),
            kind: "file".into(),
        }],
    )
    .unwrap();
    let review = f.r.archival_review(&f.project, true).unwrap();
    assert_eq!(review.items.len(), 1);
    assert!(review.warnings.iter().any(|w| w.contains("excluded")));
    f.apply(true);
    assert!(f.root.join("RAP_Master_Context_v4.md").exists());
    assert!(!f.root.join("RAP_Master_Context_v5.md").exists());
    f.apply(false);
    assert!(f.root.join("RAP_Master_Context_v5.md").exists());
    // Also exercise initial capture consent rather than only a later checkbox action.
    let mut other = Fixture::new();
    other
        .r
        .remove_with_cloud_choice(&other.project, false)
        .unwrap();
    // A retained registered project is resumed with explicit archival consent.
    other
        .r
        .add("Restored", other.root.to_str().unwrap())
        .unwrap();
    other
        .r
        .configure_tracking(&other.project, true, true, vec![])
        .unwrap();
    assert!(!other.root.join("RAP_Master_Context_v4.md").exists());
    assert!(other.root.join("RAP_Master_Context_v6.md").exists());
    for version in other.r.versions(&other.document).unwrap() {
        storage::verify(&PathBuf::from(version.snapshot_path), &version.sha256).unwrap();
    }
}

#[test]
fn foreign_archive_destination_is_preserved_and_opt_out_does_not_require_deleting_it() {
    let mut f = Fixture::new();
    let review = f.r.archival_review(&f.project, true).unwrap();
    let destination = f.root.join(&review.items[0].to);
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    fs::write(&destination, "unrelated archive file").unwrap();
    f.apply(true);
    assert_eq!(
        fs::read_to_string(&destination).unwrap(),
        "unrelated archive file"
    );
    assert!(!f.root.join("RAP_Master_Context_v4.md").exists());
    assert!(!f.root.join("RAP_Master_Context_v5.md").exists());
    assert_eq!(
        fs::read_to_string(
            f.root
                .join("Old Context Files/RAP Master Context/Archive 2/RAP_Master_Context_v4.md")
        )
        .unwrap(),
        "four"
    );
    f.apply(false);
    assert_eq!(
        fs::read_to_string(&destination).unwrap(),
        "unrelated archive file"
    );
    assert!(f.root.join("RAP_Master_Context_v4.md").exists());
    assert!(f.root.join("RAP_Master_Context_v5.md").exists());
    assert!(f.r.tracking_view(&f.project).unwrap().operations.is_empty());
}

#[test]
fn turning_archival_off_reverses_a_move_that_failed_before_metadata_commit() {
    let mut f = Fixture::new();
    f.apply(true);
    f.r.db.execute_batch("CREATE TRIGGER fail_archive BEFORE UPDATE OF state ON operations WHEN NEW.kind='archive' AND NEW.state='done' BEGIN SELECT RAISE(FAIL,'injected archive commit failure'); END;").unwrap();
    fs::write(f.root.join("RAP_Master_Context_v7.md"), "seven").unwrap();
    f.scan();
    assert!(!f.root.join("RAP_Master_Context_v6.md").exists());
    assert!(f.r.tracking_view(&f.project).unwrap().error.is_some());
    f.apply(false);
    assert_eq!(
        fs::read_to_string(f.root.join("RAP_Master_Context_v6.md")).unwrap(),
        "six"
    );
    assert_eq!(
        fs::read_to_string(f.root.join("RAP_Master_Context_v7.md")).unwrap(),
        "seven"
    );
    assert!(f.r.tracking_view(&f.project).unwrap().operations.is_empty());
    assert_eq!(f.r.versions(&f.document).unwrap().len(), 4);
}

#[test]
fn nested_sources_restore_to_their_original_folder_and_silent_setting_rejection_moves_nothing() {
    let mut f = Fixture::new();
    fs::create_dir(f.root.join("nested")).unwrap();
    fs::write(f.root.join("nested/Notes_v1.md"), "notes one").unwrap();
    f.scan();
    let revision = crate::path_selection::revision(&f.r.db, &f.project).unwrap();
    f.r.save_path_selection(&f.project, revision, vec![])
        .unwrap();
    f.scan();
    fs::write(f.root.join("nested/Notes_v2.md"), "notes two").unwrap();
    f.scan();
    f.r.db.execute_batch("CREATE TRIGGER reject_setting BEFORE UPDATE OF archive_enabled ON projects BEGIN SELECT RAISE(IGNORE); END;").unwrap();
    let review = f.r.archival_review(&f.project, true).unwrap();
    assert!(f
        .r
        .apply_archival(&f.project, true, &review.token)
        .unwrap_err()
        .contains("verification failed"));
    assert!(!f.root.join("Old Context Files").exists());
    assert!(f.root.join("nested/Notes_v1.md").exists());
    f.r.db
        .execute_batch("DROP TRIGGER reject_setting;")
        .unwrap();
    f.apply(true);
    assert!(!f.root.join("nested/Notes_v1.md").exists());
    f.apply(false);
    let view = f.r.tracking_view(&f.project).unwrap();
    assert!(view.error.is_none(), "{:?}", view.error);
    assert_eq!(
        fs::read_to_string(f.root.join("nested/Notes_v1.md")).unwrap(),
        "notes one"
    );
    assert!(!f.root.join("Notes_v1.md").exists());
}

#[cfg(windows)]
#[test]
fn verified_rename_preserves_exact_utf16_names_across_buffer_alignments() {
    let f = Fixture::new();
    let root = PathBuf::from(f.r.path(&f.project).unwrap());
    for length in 1..=24 {
        fs::write(root.join("rename-source.bin"), "unchanged bytes").unwrap();
        let from = storage::checked(&root, "rename-source.bin", false).unwrap();
        let target = format!("alignment/{}Ω.bin", "x".repeat(length));
        let to = storage::checked(&root, &target, true).unwrap();
        let file = storage::locked_read(&from.path, true).unwrap();
        storage::rename_verified(&file, &to.path).unwrap();
        drop(file);
        assert!(!from.path.exists());
        assert_eq!(fs::read_to_string(to.path).unwrap(), "unchanged bytes");
    }
}

#[test]
fn archival_opt_out_restores_sources_without_resuming_paused_tracking() {
    let mut f = Fixture::new();
    f.apply(true);
    let versions = f.r.versions(&f.document).unwrap();
    f.r.configure_tracking(&f.project, false, true, vec![])
        .unwrap();
    f.apply(false);
    let view = f.r.tracking_view(&f.project).unwrap();
    assert!(!view.enabled);
    assert!(!view.archive_enabled);
    assert!(view.operations.is_empty());
    assert_eq!(f.r.versions(&f.document).unwrap().len(), versions.len());
    for (name, content) in [
        ("RAP_Master_Context_v4.md", "four"),
        ("RAP_Master_Context_v5.md", "five"),
        ("RAP_Master_Context_v6.md", "six"),
    ] {
        assert_eq!(fs::read_to_string(f.root.join(name)).unwrap(), content);
    }
}

#[test]
fn readable_archive_paths_preserve_nested_names_decimal_labels_and_reservations() {
    let f = Fixture::new();
    let path = |family: &str, relative: &str, capture, reserved: &[String]| {
        crate::operations::archive_destination(
            &f.r.db, &f.project, family, relative, capture, reserved,
        )
        .unwrap()
    };
    let original = "plans/RAP_Master_Context_v7.2.md";
    let base = path("RAP Master Context", original, None, &[]);
    assert_eq!(
        base,
        "Old Context Files/plans/RAP Master Context/RAP_Master_Context_v7.2.md"
    );
    assert_eq!(
        path("RAP Master Context", original, None, &[base.to_uppercase()]),
        "Old Context Files/plans/RAP Master Context/Archive 2/RAP_Master_Context_v7.2.md"
    );
    assert_eq!(
        path("RAP Master Context", original, Some(3), &[]),
        "Old Context Files/plans/RAP Master Context/Capture 3/RAP_Master_Context_v7.2.md"
    );
    let long_name = format!("RAP_{}_v12.3.docx", "Technical_".repeat(10));
    assert_eq!(
        PathBuf::from(path("RAP Technical", &long_name, None, &[]))
            .file_name()
            .unwrap()
            .to_str()
            .unwrap(),
        long_name
    );
    assert_eq!(
        path("CON", "RAP_Notes_v1.md", None, &[]),
        "Old Context Files/CON documents/RAP_Notes_v1.md"
    );
    assert!(crate::operations::archive_destination(
        &f.r.db,
        &f.project,
        "RAP",
        "../RAP_Notes_v1.md",
        None,
        &[]
    )
    .is_err());
    // Even a temporarily missing archived file must not have its journal path reused.
    let id = uuid::Uuid::new_v4().to_string();
    f.r.db.execute("INSERT INTO operations(id,project_id,document_id,dedupe_key,kind,state,payload,created_at,updated_at) VALUES(?1,?2,?3,?1,'archive','done',?4,'now','now')", rusqlite::params![id,f.project,f.document,serde_json::json!({"destination":base}).to_string()]).unwrap();
    assert!(path("RAP Master Context", original, None, &[]).contains("/Archive 2/"));
    f.r.db
        .execute("UPDATE operations SET state='restored' WHERE id=?1", [&id])
        .unwrap();
    assert_eq!(path("RAP Master Context", original, None, &[]), base);
}

#[test]
fn readable_snapshot_copy_names_keep_author_version_separate_from_capture_sequence() {
    let mut f = Fixture::new();
    f.apply(true);
    for contents in ["six edited once", "six edited twice"] {
        fs::write(f.root.join("RAP_Master_Context_v6.md"), contents).unwrap();
        f.scan();
    }
    for (capture, contents) in [(3, "six"), (4, "six edited once")] {
        let archive = f.root.join(format!(
            "Old Context Files/RAP Master Context/Capture {capture}/RAP_Master_Context_v6.md"
        ));
        assert_eq!(fs::read_to_string(archive).unwrap(), contents);
    }
    f.apply(false);
    assert_eq!(
        fs::read_to_string(f.root.join("RAP_Master_Context_v6.md")).unwrap(),
        "six edited twice"
    );
    assert_eq!(f.r.versions(&f.document).unwrap().len(), 5);
    assert!(f
        .root
        .join("Old Context Files/RAP Master Context/Capture 3/RAP_Master_Context_v6.md")
        .exists());
}

#[test]
fn legacy_uuid_archive_receipt_replays_and_restores_without_rewriting_its_path() {
    let mut f = Fixture::new();
    let versions = f.r.versions(&f.document).unwrap();
    let older = versions
        .iter()
        .find(|v| v.source_name == "RAP_Master_Context_v4.md")
        .unwrap();
    let current = versions
        .iter()
        .find(|v| v.source_name == "RAP_Master_Context_v6.md")
        .unwrap();
    let source = |name: &str| -> String {
        f.r.db
            .query_row(
                "SELECT id FROM source_files WHERE project_id=?1 AND name=?2",
                rusqlite::params![f.project, name],
                |r| r.get(0),
            )
            .unwrap()
    };
    let id = uuid::Uuid::new_v4().to_string();
    let destination = format!(
        "Old Context Files/RAP Master Context - {}/{} - RAP_Master_Context_v4.md",
        f.document, id
    );
    let plan = crate::operations::ArchivePlan {
        project: f.project.clone(),
        document: f.document.clone(),
        old_version: older.id.clone(),
        new_version: current.id.clone(),
        old_source: source("RAP_Master_Context_v4.md"),
        new_source: source("RAP_Master_Context_v6.md"),
        destination: destination.clone(),
        temporary: "Old Context Files/.unused.tmp".into(),
        mode: "move".into(),
        old_identity: None,
    };
    f.r.db.execute("INSERT INTO operations(id,project_id,document_id,dedupe_key,kind,payload,created_at,updated_at) VALUES(?1,?2,?3,?1,'archive',?4,'now','now')",rusqlite::params![id,f.project,f.document,serde_json::to_string(&plan).unwrap()]).unwrap();
    f.r.db
        .execute(
            "UPDATE projects SET archive_enabled=1 WHERE id=?1",
            [&f.project],
        )
        .unwrap();
    assert!(f.r.recover_operations(&f.project, true).unwrap().is_empty());
    assert_eq!(
        fs::read_to_string(f.root.join(&destination)).unwrap(),
        "four"
    );
    f.apply(false);
    assert_eq!(
        fs::read_to_string(f.root.join("RAP_Master_Context_v4.md")).unwrap(),
        "four"
    );
    assert!(!f.root.join(destination).exists());
    assert_eq!(f.r.versions(&f.document).unwrap().len(), 3);
}
