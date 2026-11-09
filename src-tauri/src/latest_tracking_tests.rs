use crate::{latest_tracking, registry::Registry};
use std::{collections::HashSet, fs};
use tempfile::tempdir;

fn rename_fixture(
    files: &[(&str, &str)],
) -> (tempfile::TempDir, std::path::PathBuf, Registry, String) {
    let t = tempdir().unwrap();
    let root = t.path().join("Project");
    let app = t.path().join("App");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&app).unwrap();
    for (name, contents) in files {
        let path = root.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
    let mut r = Registry::open(&app.join("registry.db")).unwrap();
    let project = r.add("Project", root.to_str().unwrap()).unwrap();
    r.configure_tracking(&project, true, false, vec![]).unwrap();
    (t, root, r, project)
}

#[test]
fn latest_tracking_excluded_moves_keep_word_and_markdown_identities_after_forget_and_restart() {
    for forgotten in [false, true] {
        let (_t, root, mut r, project) = rename_fixture(&[
            ("ContextDock_Questions_v1.docx", "questions"),
            ("ContextDock_Paper_v1.docx", "paper"),
            ("ContextDock_Design_v1.md", "design"),
            ("ContextDock_Keep_v1.md", "keep"),
        ]);
        let original = r
            .source_ids(&project)
            .unwrap()
            .iter()
            .map(|id| r.source(id).unwrap())
            .collect::<Vec<_>>();
        crate::drive_publish::configure(&r.db, &project, "account-A", true, true).unwrap();
        for source in &original {
            r.db.execute("INSERT INTO drive_documents(document_id,project_id,file_id,verified_hash,state) VALUES(?1,?2,?3,?4,'synced')", rusqlite::params![source.document_id,project,format!("remote-{}",source.id),source.last_hash]).unwrap();
        }
        let revision = crate::path_selection::revision(&r.db, &project).unwrap();
        r.save_path_selection(
            &project,
            revision,
            vec![crate::path_selection::Rule {
                path: "chat context".into(),
                kind: "folder".into(),
            }],
        )
        .unwrap();
        fs::create_dir(root.join("Chat Context")).unwrap();
        let moved = original
            .iter()
            .filter(|s| !s.name.contains("Keep"))
            .collect::<Vec<_>>();
        for source in &moved {
            fs::rename(
                root.join(&source.relative),
                root.join("Chat Context").join(source.name.to_lowercase()),
            )
            .unwrap();
        }
        r.refresh(&project).unwrap();
        if forgotten {
            for source in [moved[0], moved[2]] {
                let plan = r.preview_forget(&project, &source.relative).unwrap();
                r.forget_missing(&project, &plan.path, &plan.token).unwrap();
            }
        }
        r.reconcile(&project, "watcher", &HashSet::new(), true)
            .unwrap();
        let db = r.store.join("registry.db");
        drop(r);
        let mut r = Registry::open(&db).unwrap();
        r.reconcile(&project, "startup_reconciliation", &HashSet::new(), true)
            .unwrap();
        let view = r.path_selection(&project).unwrap();
        for source in &moved {
            let after = r.source(&source.id).unwrap();
            assert_eq!(after.document_id, source.document_id);
            assert_eq!(after.disposition, "ignored");
            assert_eq!(after.status, "available");
            assert!(after
                .relative
                .replace('\\', "/")
                .starts_with("Chat Context/"));
            assert_eq!(after.last_hash, source.last_hash);
            let entry = view
                .entries
                .iter()
                .find(|e| {
                    crate::path_selection::normalized(&e.path)
                        == crate::path_selection::normalized(&after.relative)
                })
                .unwrap();
            assert!(!entry.new);
            assert!(!view
                .entries
                .iter()
                .any(|e| crate::path_selection::normalized(&e.path)
                    == crate::path_selection::normalized(&source.relative)));
            assert!(!crate::path_selection::allowed(&r.db, &project, &after.relative).unwrap());
        }
        assert!(view.entries.iter().all(|e| e.status != "missing"));
        assert_eq!(
            crate::drive_publish::eligible_documents(&r.db, &project)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            r.db.query_row("SELECT COUNT(*) FROM document_versions", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            4
        );
        assert_eq!(
            r.db.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| r
                .get::<_, i64>(
                0
            ))
            .unwrap(),
            0
        );
        assert!(!r.store.join("snapshots").exists());
        let revision = view.revision;
        let rules = view
            .exclusions
            .into_iter()
            .filter(|rule| !rule.path.starts_with("chat context"))
            .collect();
        r.save_path_selection(&project, revision, rules).unwrap();
        r.reconcile(&project, "manual_refresh", &HashSet::new(), true)
            .unwrap();
        assert_eq!(
            crate::drive_publish::eligible_documents(&r.db, &project)
                .unwrap()
                .len(),
            4
        );
        for source in &original {
            assert_eq!(
                r.db.query_row(
                    "SELECT file_id FROM drive_documents WHERE document_id=?1",
                    [&source.document_id],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
                format!("remote-{}", source.id)
            );
        }
    }
}

#[test]
fn latest_tracking_excluded_move_detection_retains_changed_copies_ambiguity_and_pending_uploads() {
    for scenario in [
        "copy",
        "changed",
        "destinations",
        "sources",
        "pending",
        "renamed",
        "partial",
    ] {
        let files = if scenario == "sources" {
            vec![
                ("Old A/ContextDock_Context_v1.md", "same"),
                ("Old B/ContextDock_Context_v1.md", "same"),
            ]
        } else {
            vec![("ContextDock_Context_v1.md", "same")]
        };
        let (_t, root, mut r, project) = rename_fixture(&files);
        let before = r
            .source_ids(&project)
            .unwrap()
            .iter()
            .map(|id| r.source(id).unwrap())
            .collect::<Vec<_>>();
        let rev = crate::path_selection::revision(&r.db, &project).unwrap();
        r.save_path_selection(
            &project,
            rev,
            vec![crate::path_selection::Rule {
                path: "chat context".into(),
                kind: "folder".into(),
            }],
        )
        .unwrap();
        fs::create_dir_all(root.join("Chat Context/Other")).unwrap();
        let filename = if scenario == "renamed" {
            "ContextDock_Other_v1.md"
        } else {
            "ContextDock_Context_v1.md"
        };
        fs::write(
            root.join("Chat Context").join(filename),
            if scenario == "changed" {
                "diff"
            } else {
                "same"
            },
        )
        .unwrap();
        if scenario == "destinations" {
            fs::write(
                root.join("Chat Context/Other/ContextDock_Context_v1.md"),
                "same",
            )
            .unwrap();
        }
        if scenario != "copy" {
            for source in &before {
                fs::remove_file(root.join(&source.relative)).unwrap();
            }
        }
        if scenario == "pending" {
            crate::drive_publish::configure(&r.db, &project, "account-A", true, true).unwrap();
            r.db.execute("INSERT INTO drive_documents(document_id,project_id,file_id,pending_hash) VALUES(?1,?2,'retained-file','pending')", rusqlite::params![before[0].document_id,project]).unwrap();
        }
        r.refresh(&project).unwrap();
        if scenario == "partial" {
            r.db.execute(
                "UPDATE projects SET status='partial',scan_error='unreadable subtree' WHERE id=?1",
                [&project],
            )
            .unwrap();
            let rules = crate::path_selection::rules(&r.db, &project).unwrap();
            r.reconcile_excluded_relocations(&project, &root, &rules)
                .unwrap();
        } else {
            r.reconcile(&project, "watcher", &HashSet::new(), true)
                .unwrap();
        }
        for source in &before {
            assert_eq!(
                r.source(&source.id).unwrap().relative,
                source.relative,
                "{scenario}"
            );
        }
        let destination = format!("Chat Context/{filename}");
        let view = r.path_selection(&project).unwrap();
        assert!(
            view.entries
                .iter()
                .any(
                    |e| crate::path_selection::normalized(&e.path) == destination.to_lowercase()
                        && e.new
                ),
            "{scenario}"
        );
        assert!(!crate::path_selection::allowed(&r.db, &project, &destination).unwrap());
    }
}

#[test]
fn latest_tracking_version_rename_retires_old_path_but_a_real_missing_latest_still_warns() {
    let (_t, root, mut r, project) = rename_fixture(&[("ContextDock_Context_v8.md", "same")]);
    let family = crate::drive_publish::eligible_documents(&r.db, &project).unwrap()[0].clone();
    fs::rename(
        root.join("ContextDock_Context_v8.md"),
        root.join("ContextDock_Context_v9.md"),
    )
    .unwrap();
    r.reconcile(&project, "watcher", &HashSet::new(), true)
        .unwrap();
    let visible = r.snapshot().unwrap().documents;
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].name, "ContextDock_Context_v9.md");
    assert_eq!(visible[0].document_id, family);
    assert_eq!(visible[0].status, "available");
    assert!(r
        .path_selection(&project)
        .unwrap()
        .entries
        .iter()
        .all(|e| !e.path.ends_with("ContextDock_Context_v8.md")));
    assert_eq!(r.versions(&family).unwrap().len(), 2);
    fs::remove_file(root.join("ContextDock_Context_v9.md")).unwrap();
    r.reconcile(&project, "watcher", &HashSet::new(), true)
        .unwrap();
    let visible = r.snapshot().unwrap().documents;
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].status, "missing");
}

#[test]
fn latest_tracking_repairs_existing_version_rename_metadata_and_restores_a_reappearing_path() {
    let (_t, root, mut r, project) = rename_fixture(&[
        ("ContextDock_Context_v6.md", "same"),
        ("ContextDock_Context_v7.md", "same"),
        ("ContextDock_Context_v8.md", "newer"),
    ]);
    let old = r
        .snapshot()
        .unwrap()
        .documents
        .into_iter()
        .find(|d| d.name.ends_with("v6.md"))
        .unwrap();
    let before = r.versions(&old.document_id).unwrap().len();
    fs::remove_file(root.join("ContextDock_Context_v6.md")).unwrap();
    r.reconcile(&project, "startup_reconciliation", &HashSet::new(), true)
        .unwrap();
    assert!(r
        .snapshot()
        .unwrap()
        .documents
        .iter()
        .all(|d| d.status != "missing"));
    assert_eq!(r.versions(&old.document_id).unwrap().len(), before);
    assert_eq!(r.source(&old.id).unwrap().disposition, "renamed");
    fs::write(root.join("ContextDock_Context_v6.md"), "same").unwrap();
    r.reconcile(&project, "watcher", &HashSet::new(), true)
        .unwrap();
    let restored = r
        .snapshot()
        .unwrap()
        .documents
        .into_iter()
        .find(|d| d.id == old.id)
        .unwrap();
    assert_eq!(restored.status, "available");
    assert_eq!(restored.document_id, old.document_id);
}

#[test]
fn latest_tracking_missing_path_is_not_retired_for_changed_ambiguous_or_excluded_successors() {
    for (contents, ambiguous, excluded) in [
        ("different", false, false),
        ("same", true, false),
        ("same", false, true),
    ] {
        let (_t, root, mut r, project) = rename_fixture(&[
            ("ContextDock_Context_v1.md", "same"),
            ("ContextDock_Context_v2.md", contents),
        ]);
        if ambiguous {
            fs::write(root.join("ContextDock_Context_v3.md"), "same").unwrap();
            r.reconcile(&project, "watcher", &HashSet::new(), true)
                .unwrap();
        }
        if excluded {
            let revision = crate::path_selection::revision(&r.db, &project).unwrap();
            r.save_path_selection(
                &project,
                revision,
                vec![crate::path_selection::Rule {
                    path: "ContextDock_Context_v2.md".into(),
                    kind: "file".into(),
                }],
            )
            .unwrap();
        }
        fs::remove_file(root.join("ContextDock_Context_v1.md")).unwrap();
        r.reconcile(&project, "watcher", &HashSet::new(), true)
            .unwrap();
        assert!(r
            .snapshot()
            .unwrap()
            .documents
            .iter()
            .any(|d| d.name.ends_with("v1.md") && d.status == "missing"));
    }
}

#[test]
fn latest_tracking_does_not_create_archives_or_snapshot_bytes_and_reuses_save_metadata() {
    let t = tempdir().unwrap();
    let root = t.path().join("Project");
    let app = t.path().join("App");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&app).unwrap();
    fs::write(root.join("ContextDock_Context_v1.md"), "one").unwrap();
    let mut r = Registry::open(&app.join("registry.db")).unwrap();
    assert!(latest_tracking::enabled(&r.db));
    let project = r.add("Project", root.to_str().unwrap()).unwrap();
    r.configure_tracking(&project, true, true, vec![]).unwrap();
    assert!(!r.tracking_flags(&project).unwrap().1);
    for n in 0..8 {
        fs::write(root.join("ContextDock_Context_v1.md"), format!("save {n}")).unwrap();
        r.reconcile(&project, "watcher", &HashSet::new(), true)
            .unwrap();
    }
    assert_eq!(
        r.db.query_row("SELECT COUNT(*) FROM document_versions", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert!(!app.join("snapshots").exists());
    assert!(!app.join("staging").exists());
    assert!(!root.join("Old Context Files").exists());
    assert_eq!(
        fs::read_to_string(root.join("ContextDock_Context_v1.md")).unwrap(),
        "save 7"
    );
}

#[test]
fn latest_tracking_groups_numbered_files_and_keeps_old_local_sources_in_place() {
    let t = tempdir().unwrap();
    let root = t.path().join("Project");
    let app = t.path().join("App");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&app).unwrap();
    fs::write(root.join("ContextDock_Context_v1.md"), "one").unwrap();
    fs::write(root.join("ContextDock_Context_v2.md"), "two").unwrap();
    let mut r = Registry::open(&app.join("registry.db")).unwrap();
    let project = r.add("Project", root.to_str().unwrap()).unwrap();
    r.configure_tracking(&project, true, false, vec![]).unwrap();
    let families = crate::drive_publish::eligible_documents(&r.db, &project).unwrap();
    assert_eq!(families.len(), 1);
    assert_eq!(
        crate::drive_publish::current(&r.db, &project, &families[0])
            .unwrap()
            .name,
        "ContextDock_Context_v2.md"
    );
    assert!(root.join("ContextDock_Context_v1.md").exists());
    assert!(root.join("ContextDock_Context_v2.md").exists());
    assert!(!app.join("snapshots").exists());
}

#[test]
fn latest_tracking_upgrade_preserves_existing_snapshot_and_archived_source_bytes() {
    let t = tempdir().unwrap();
    let root = t.path().join("Project");
    let app = t.path().join("App");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&app).unwrap();
    fs::write(root.join("ContextDock_Context_v1.md"), "legacy content").unwrap();
    let db = app.join("registry.db");
    let mut old = Registry::open_legacy_fixture(&db).unwrap();
    let project = old.add("Project", root.to_str().unwrap()).unwrap();
    old.configure_tracking(&project, true, true, vec![])
        .unwrap();
    let snapshot: String = old
        .db
        .query_row(
            "SELECT snapshot_relative FROM document_versions LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    old.db.execute("INSERT INTO project_chats(id,project_id,title,created_at,updated_at) VALUES('preserved-chat',?1,'Saved chat','sample','sample')",[&project]).unwrap();
    fs::create_dir(root.join("Old Context Files")).unwrap();
    fs::write(
        root.join("Old Context Files/ContextDock_Archive_v1.md"),
        "preserved archive",
    )
    .unwrap();
    old.db
        .execute_batch(
            "DELETE FROM preferences WHERE key='workflow.latest_only'; PRAGMA user_version=15;",
        )
        .unwrap();
    drop(old);
    let mut r = Registry::open(&db).unwrap();
    assert!(latest_tracking::enabled(&r.db));
    assert!(!r.tracking_flags(&project).unwrap().1);
    fs::write(root.join("ContextDock_Context_v1.md"), "current content").unwrap();
    r.reconcile(&project, "watcher", &HashSet::new(), true)
        .unwrap();
    assert_eq!(
        fs::read_to_string(app.join(snapshot)).unwrap(),
        "legacy content"
    );
    assert_eq!(
        fs::read_to_string(root.join("Old Context Files/ContextDock_Archive_v1.md")).unwrap(),
        "preserved archive"
    );
    assert_eq!(fs::read_dir(app.join("snapshots")).unwrap().count(), 1);
    assert_eq!(
        r.db.query_row(
            "SELECT title FROM project_chats WHERE id='preserved-chat'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "Saved chat"
    );
}

#[test]
fn latest_tracking_forgetting_a_missing_path_keeps_document_identity_and_saved_metadata() {
    let t = tempdir().unwrap();
    let root = t.path().join("Project");
    let app = t.path().join("App");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&app).unwrap();
    let source = root.join("ContextDock_Context_v1.md");
    fs::write(&source, "one").unwrap();
    let mut r = Registry::open(&app.join("registry.db")).unwrap();
    let project = r.add("Project", root.to_str().unwrap()).unwrap();
    r.configure_tracking(&project, true, false, vec![]).unwrap();
    let before: (String, String) =
        r.db.query_row("SELECT id,document_id FROM source_files LIMIT 1", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    fs::remove_file(&source).unwrap();
    r.refresh(&project).unwrap();
    let plan = r
        .preview_forget(&project, "ContextDock_Context_v1.md")
        .unwrap();
    assert_eq!(plan.revision_count, 0);
    r.forget_missing(&project, &plan.path, &plan.token).unwrap();
    assert!(r.snapshot().unwrap().documents.is_empty());
    assert!(r
        .path_selection(&project)
        .unwrap()
        .entries
        .iter()
        .all(|entry| entry.kind != "file"));
    assert_eq!(
        r.db.query_row("SELECT COUNT(*) FROM document_versions", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert!(crate::drive_publish::eligible_documents(&r.db, &project)
        .unwrap()
        .is_empty());
    fs::write(&source, "one").unwrap();
    r.refresh(&project).unwrap();
    let after: (String, String) =
        r.db.query_row("SELECT id,document_id FROM source_files LIMIT 1", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!(before, after);
}
