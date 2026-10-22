# ContextDock filename versions and save revisions

The user requested that edits to capitalization, Word image placement or metadata must not create a new counted version. They explicitly chose to keep syncing edited content under the same filename version.

## Behavior

- Routine saves of Report_v4.md or Report_v4.docx retain one v4 in Versions/history. The latest immutable save revision supplies cloud sync. There is no content-size or "minor change" heuristic.
- Report_v5 is a distinct version even when its bytes initially match v4. Later reverting v5 content to bytes previously seen in v4 retains v5 identity.
- Unnumbered files remain one working version. Removing a version suffix or renaming an unnumbered source does not itself increase the count. Decimal labels are recognized and equivalent numeric labels share a group.
- A backwards suffix goes to review instead of silently replacing the current version. Explicit manual association remains available.
- History and comparison choices show the latest revision of each filename version. Original immutable revisions remain retained locally with stable IDs/hashes for recovery and existing reports/chats. The schema-10 registry can already contain many captures; the new view groups them without deleting them.
- Current and history cloud policies use only the latest saved revision of each filename version. Existing verified replacement waits for indexing and preserves ownership checks. Older uploaded Files can remain retained for reuse; superseded index associations are removed through the existing workflow. Source files and local snapshots are retained.
- Snapshot revision sequences remain internal recovery/cloud identities. Filename labels are displayed in history, comparison, cloud jobs and chat citations; unnumbered evidence is labeled Saved revision rather than an invented vN.
- Existing separately enabled archival behavior, including snapshot-copy archives of in-place edits, remains intact. Those are safety copies, not additional counted filename versions.

## Migration and checks

Schema 011 preserves existing version IDs, snapshot paths, hashes, capture metadata, chat/report references, cloud jobs and installation identity. It permits identical content across distinct authored versions, persists new revision group keys and scopes deduplication by group. Migration runs transactionally with a foreign-key check before commit and restores enforcement afterward. Invalid old references roll back rather than silently dropping data. No application database or project directory was edited for implementation/testing. Installing the release does not reset projects.

Regression fixtures cover twelve repeated saves each of Markdown and Word files, stable logical counts and v4 labels, timestamp-only changes, latest content, unchanged first snapshots, restart, identical-byte v4-to-v5 export, v5 reverting to original v4 bytes, unnumbered edits/undo, equivalent decimal labels, backwards filename review, valid DOCX media-only changes syncing in both cloud policies, preservation of original cloud/snapshot identities on a schema-10 upgrade and rollback on invalid references. Frontend tests check v4 display for saved revision 97 and compare availability. Existing watcher, capture/crash recovery, archive/restore, folder selection, cloud ownership/cleanup, comparison and chat tests remain required release checks.

Manual acceptance after installation: edit and save a selected v4 Markdown or Word document, refresh or wait for tracking, verify the version count stays unchanged and cloud indexes the edit; reopen the app and repeat; save/export v5 and verify one additional counted version. Compare v4/v5 and check source filenames. Keep the existing backups. This automated validation does not claim installed upgrade or authenticated provider acceptance.

Release validation for 0.9.1: 206 native tests passed with one existing intentionally ignored child fixture; 138 frontend tests across 16 files passed. TypeScript, ESLint, all-target Clippy with warnings denied, rustfmt, diff whitespace checks and production frontend compilation passed. Tests include the schema-10 upgrade and migration rollback paths. Windows packaging is recorded separately in verification.md after the build completes.
