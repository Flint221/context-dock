# ContextDock project removal v1

October 22, 2026. The user requested removal of the remaining Cattle Tool registration while preserving actual documents and backups.

## Findings

The running ContextDock 0.8.1 process initially displayed the reported database-malformed error. A copy of the database reached through the ordinary app-data path was healthy schema 10 with no projects. Windows file-handle inspection established that the running process held a different file object despite the same reported path: its registry was 573,440 bytes, while the ordinary-path registry was 327,680 bytes, with different Windows file identities. This explains why the previous empty-registry verification did not establish what this process was using. The reason for the different file objects is not established; no filesystem/virtualization cause is claimed.

ContextDock exited before the attempted per-handle copy. The prior application's exact database object remained accessible by its Windows file ID. That object was copied independently, without changing it, into `%LOCALAPPDATA%\ContextDock_Project_Removal_v1\Actual_Registry_Copy\contextdock.db`. Its SHA-256 was `463a7a566f40dc6888dd30c99d94daba7b7ffc21f69d1954d57f4b080b92d218`. The earlier attempted running-copy directory is empty and is not a backup claim.

The exact registry used by the application was schema 9 and still malformed. Its projects table was readable and showed six registrations, five already removed and Cattle Tool still active. This is distinct from the separate fresh schema-10 registry. No claim is made that the earlier reset had replaced this application's exact file object.

## Applied removal

A disposable candidate copy was updated first using the registration tombstone and cloud/tracking-disable statements corresponding to Registry::remove_with_cloud_choice. Cattle Tool was matched by its exact project UUID and name. The existing project-removal trigger also disabled its cloud configuration. No documents, versions, snapshots, chats or source files were deleted.

With ContextDock not running, the exact previously observed application database object was reopened by Windows file ID with exclusive access. Its bytes had to match the preserved backup before any write. The candidate retained the same file length. A write/flush was followed by complete byte readback, with rollback to the preserved original specified if verification failed. Verification passed. Intent, before/after hashes and changed-page inventory are retained in ContextDock_Removal_Intent_v1.json; the verified readback copy and result are retained alongside it.

Verified from the exact-object readback: zero active projects; Cattle Tool removed_at is set; tracking and archival are off; project cloud enabled, sync_requested, delete_requested and auto_sync are all zero, with paused one. Reopening ContextDock is needed to refresh the displayed project list. No GUI interaction or new application launch was performed by the agent, and no live UI acceptance is claimed.

Actual project files, archives, snapshots and all existing backups were not modified by this operation. The exact pre-removal database copy remains available in addition to earlier evidence. No cloud requests or uploads were made.

This is a scoped registration removal, not a full repair of the remaining malformed schema-9 database. Other database errors may remain; they must not be described as fixed by this tombstone. The separate healthy schema-10 registry still has zero projects. Any later complete reset must verify the database object actually opened by the application rather than relying only on a matching displayed pathname.

Only this operational report changes the repository. No application code, installer, schema migration or new automated-test result is included in this removal.
