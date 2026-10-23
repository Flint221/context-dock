# ContextDock missing-file cleanup

Version 0.10.2 adds **Forget…** beside a confirmed Missing file in the saved folder tree. Ignore and unchecking remain reversible selection controls that retain discovery and history. Forget is a separate explicit permanent action; it never runs automatically when a source disappears.

Save or discard checkbox drafts before using Forget. The confirmation displays the exact relative path and number of captured save revisions affected. A never-approved test file normally has zero saved revisions. Cancel is initially focused and removes nothing. Confirmation removes that path's discovery record, pending approval, review decision and exact file exclusion. It deletes only snapshots captured from that exact path, preserving other filename versions in the family. An empty family is removed when no retained references prevent it. The missing warning, folder tree and tracking counts refresh after persistence. A minimal project audit event records the action and count without retaining the forgotten filename.

The backend requires a healthy project root and complete scan. It rechecks absence, selection revision, folder identity/revision, family/history state and cloud revision/jobs against a preview token while holding the registry and cloud-operation gates. A returned file, unavailable drive, incomplete scan or changed review cannot authorize deletion. Unfinished capture/archive operations, unresolved candidate matches, referenced grouped families, approved AI baselines and recorded archive copies block deletion with an explanation. Uploaded or uncertain cloud jobs retain their identities and block permanent local-history removal. This action does not delete remote data or silently lose cleanup evidence.

Snapshot deletion is recorded in migration 013's `forgotten_snapshots` journal in the same transaction as removing local history. Only unreferenced paths under the project's application-owned `snapshots/` directory are eligible. File bytes must match the saved hash; links, traversal and conflicting content are refused. On Windows, deletion uses a verified file handle, including the snapshot's read-only attribute. Failed physical cleanup remains queued with an error and appears as a folder-tree warning. Reloading the tree or reopening the application retries it. A committed removal closes its confirmation even if physical cleanup needs a retry.

Original source files, archive directories, backups, saved chats and reports are preserved. Forget is not a secure-erasure operation and does not change independent backup copies. If the same path is recreated, discovery sees it again and applies saved approval rules rather than restoring the erased history.

## Acceptance

1. Discover a new unrelated test Markdown file, delete it and reload the tree. Open Forget and verify zero captured revisions. Cancel leaves the row; confirm removes it and the missing-file warning. Restart and verify it stays gone.
2. Capture edits to a disposable tracked document, delete the source and confirm the reported revision count. Forget should remove only those snapshot files and history rows. Other documents and archive/backup files must remain intact.
3. With two filename versions grouped, delete the older source and forget only it. The newer source, snapshot and current-family label must remain.
4. Recreate the source after opening confirmation, change the saved selection or move the root. Confirmation must fail without deleting history. Resolve the state and open a fresh preview.
5. Verify cloud-linked or retained-baseline/archive histories show a blocker and cannot be confirmed. Excluding or ignoring a file must continue to preserve history.
6. Lock a disposable snapshot during removal. The entry should disappear after committed removal, with physical cleanup reported as pending. Release the lock and restart/reload; cleanup should finish. Corrupt or redirected snapshots must remain untouched and queued for inspection.

Tests use disposable project directories and databases. Actual installed projects, snapshots, credentials, backups and cloud resources are not modified during implementation or test execution.
