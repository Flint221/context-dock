# Architecture — Milestone 2

ContextDock 0.2.0 extends the Milestone 1 Tauri/React application without changing `com.contextdock.desktop`, existing entity UUIDs, or the supported per-user application data location.

## Module boundaries

| Module | Responsibility |
| --- | --- |
| `src/App.tsx` | Existing shell, navigation, feedback and registry-change subscription |
| `src/components/TrackingPanel.tsx` | Explicit tracking/archival consent, initialization selection, pending match decisions, family summaries and persisted activity |
| `DocumentHistory.tsx` | Independent version history, metadata, verified locations and path copying |
| `src/services/registry.ts` / `types.ts` | Typed, narrow Tauri IPC facade and payload contracts |
| `src-tauri/src/lib.rs` | Supported data paths, one-process store lock, blocking-task IPC commands and app-lifetime monitor |
| `registry.rs` / `models.rs` | Versioned migrations, project management, read-only metadata discovery/reconciliation and shell snapshots |
| `tracking.rs` | Initialization, deterministic matching/review, shared change-processing pipeline and user associations |
| `matching.rs` | Trailing date/version normalization, directory-scoped identity keys and safe display/archive names |
| `storage.rs` | Streaming hashes, stability checks, pinned directories, staged no-overwrite finalization and verified Windows handle renames |
| `operations.rs` | Durable capture/archive journal and crash replay; commit version and archive intent together |
| `tracking_queries.rs` / `tracking_models.rs` | Source/family/history queries, persisted activity, narrow location validation and UI DTOs |
| `monitor.rs` | Native watcher ownership, event coalescing, retry bounds, startup reconciliation and directory recovery |
| `migrations/001_registry.sql`, `002_tracking.sql` | Transactional in-place evolution from Milestone 1 |

Expensive metadata traversal, hashing, snapshot work and archival run on blocking worker threads, outside Tauri's UI thread. A shared mutex serializes mutations across IPC and the monitor. A process-level store lock prevents simultaneous 0.2.0 processes from processing the same journal. SQLite uses WAL, `synchronous=FULL`, foreign keys, a busy timeout, versioned transactional migrations and uniqueness constraints.

The frontend has no unrestricted filesystem, shell or SQL API. It selects a folder through the official dialog plugin and uses registered project/source/document/version IDs. Its additional permissions allow listening/unlistening to the native `registry-changed` event only; packaged local content remains protected by the existing CSP. Snapshot/archive opening validates a stored relative path and its hash, then opens its parent directory in Explorer. Paths are passed as process arguments, never interpolated shell commands.

## Database evolution and identity

```text
Project (existing UUID)
  ├── Logical document (existing documents UUID retained)
  │     ├── Source file locations (many)
  │     └── Immutable captured versions (many; independent sequence)
  ├── Remembered filename/directory associations
  ├── Pending document matches
  ├── Recoverable capture/archive journal
  └── Persisted tracking activity
```

Migration 002 rebuilds `documents` as logical families and transfers every original observation into `source_files`, retaining both the original logical document UUID and the initial source UUID. It captures **no** versions, performs **no** folder organization, and defaults both tracking and archival to off. Existing projects/preferences are unchanged. A newer schema is rejected rather than downgraded.

- `projects`: original registration plus tracking/archival flags, initialization state, watcher status, actionable tracking errors and a removal tombstone.
- `documents`: logical family identity/name/type, tracking state, active source ID and current captured version ID. A provisional observation associated with another family retains a hidden `merged_into` tombstone; identities are not recycled.
- `source_files`: project/document association, relative-path identity, metadata/availability, capture disposition and last-hashed fingerprint/hash. Multiple paths can belong to one family. Superseded sources are not repeatedly reactivated merely because they remain on disk with archival off.
- `document_versions`: version UUID, family-scoped sequential number, original filename/relative path, UTC capture/source times, byte count, SHA-256, snapshot path, optional archive path and capture reason. Unique `(document_id, number)` and `(document_id, sha256)` constraints prevent duplicate sequences/content. The active pointer can reuse an older version if identical historical bytes return; no duplicate content version or rollback action is invented.
- `document_aliases`: user-established family association scoped by project, parent directory, normalized stem and document type. Conflicting associations require review.
- `pending_matches`: source, candidate family IDs, explanation, observed hash and creation time. No automatic move occurs for a pending match.
- `operations`: unique intent/deduplication key, project/family, operation kind, typed serialized plan, state, error and timestamps.
- `tracking_activity`: durable meaningful capture/archive/match/watcher/failure/recovery events; the UI loads the latest 50.

Unregistering tombstones the registration and stops tracking. Sources, archives and snapshot bytes are untouched, and retained family/version metadata can be recovered by registering the same canonical root again with tracking/archival off. There is no automatic retention deletion. A project is never a single global version chain.

## Matching and initialization

Initial selection deliberately creates a separate v1 for each selected existing observation, even if filenames look related. Unselected existing files become ignored for tracking. New files added later enter the shared pipeline.

Matching precedence:

1. An exact associated source path retains its family.
2. An explicit remembered directory/name/type association takes priority; conflicting aliases require review.
3. An identical SHA-256 at a new path is a relocation only if one matching family has a missing active source. A live duplicate or multiple hash matches requires review. Identical bytes never create another version within a family.
4. Normalized stems must agree completely. Only recognizable **trailing** groups such as `v2.1`, `version_3`, `2026-10-19`, and `20261019` are removed. A unique family in the same parent directory/type, with recognizable version/date evidence, is high confidence.
5. Multiple existing candidates, multiple same-stem incoming files, another workstream directory, or a previous pending decision requires confirmation. Matching never uses common-word overlap or AI/content similarity.
6. An unrelated file starts its own family at v1.

Assign/new/ignore decisions are explicit. Changes to a superseded source require review rather than silently replacing the current active document. An assignment of unchanged duplicate bytes may add a location and update the active pointer, but does not move another source or invent a version. Match explanations and high-confidence decisions are persisted in activity.

## Watcher lifecycle and performance

`notify` recommended native recursive watchers exist only for opted-in registered projects while this application process runs. Enabling starts a watcher; disabling/unregistering drops its ownership and processing checks the persisted opt-in again. Startup installs watchers before reconciliation so a new event cannot silently fall into a scan/watch gap. Projects that cannot be watched show a visible status and retain manual reconciliation. Missing/unwatchable roots are retried every 30 seconds.

Native access events and excluded directory components are ignored. Event delivery uses a bounded channel; overflow requests a full reconciliation. Events coalesce for approximately 950 ms of quiet with a 4-second batching bound. Changed paths force hash checks even when size/mtime were preserved. Metadata gates unchanged files during normal watcher work. Startup/manual reconciliation intentionally performs a full integrity/change check. Transient capture failures retry after bounded 2/5/15-second delays; persistent errors stay visible for manual action. Native events and IPC refresh use the same serialized, hash-idempotent pipeline.

Files are streamed in 64 KiB chunks. Recent writes receive an additional 650 ms stability sample; size/high-resolution modification metadata is compared before/after reading. Windows read handles deny writes/deletion while bytes are captured; Word sharing locks therefore cause a retry. No event alone proves a completed save. A writer that closes a file for an extended pause can still present a stable intermediate state; no general filesystem API provides a universal application-level save-complete signal. Missing paths never authorize deletion or archival.

The current implementation scans project metadata per coalesced batch and keeps results in memory. Hashing is selective for normal events, UI document rendering is paged, and family/review lists load in bounded visible groups. Very large-scale DB pagination, scan cancellation and parallel project throughput remain future hardening work. Native state snapshots may wait for the current serialized operation; the UI thread remains responsive.

## Snapshot and journal protocol

Application-owned storage is outside source and installation directories:

```text
<local app data>/com.contextdock.desktop/
  contextdock.db
  staging/<random UUID>.tmp
  snapshots/<project UUID>/<document UUID>/<version UUID>.<extension>
```

A capture streams a stable source into a newly created temporary file, flushes it, hashes it and verifies the staged bytes. A durable capture plan reserves a version UUID/sequence and records source metadata, hash, staging/final locations, prior active pointers and whether archival is authorized. The snapshot finalizes by a no-overwrite hard link on supported local storage. A pending collision is accepted only when staging/final refer to the same file, or when a prior durable `finalized` marker proves ownership. Snapshot bytes are reverified, staging is unlinked, and the snapshot is sealed read-only. The version insert, active pointer, capture event, archive intent and journal completion commit in one SQLite transaction.

Committed snapshots are never overwritten or automatically removed. Read-only attributes are protection against accidental edits, not an administrator-proof security boundary; hashes detect external tampering when a saved location is opened. A crash before a capture plan exists can leave an unreferenced staging file; it is not a committed version and source bytes remain intact. Orphan staging/snapshots are retained conservatively rather than deleted automatically. Disk usage/retention tooling is future work.

Journal states are `pending`, `finalized`, `done`, or `blocked` (with an error). A finalized ownership marker is retained if a later metadata step fails. Startup and explicit retry can finish partial captures and archives without creating duplicate versions. Disabling tracking pauses recovery; disabling archival separately pauses archive operations. Re-enabling permission resumes safe recovery. No automatic rollback or destructive cleanup is performed.

## Archive protocol

Archive destinations use `Old Context Files/<sanitized family> - <document UUID>/vN - <sanitized original stem> - <operation UUID>.<extension>`. Original filenames remain in metadata. All components are validated and parents are guarded against traversal and Windows reparse points. Existing directories may be used as containers, but unrelated files are never claimed, overwritten or cleaned up.

Both old/new snapshots must pass SHA-256 verification before an archive operation proceeds:

- **Same source path:** stream the previous immutable snapshot to a newly created archive temporary, flush and verify, then finalize without overwrite. The new active source is never moved. The archive copy has independent bytes from the immutable snapshot.
- **Different source paths:** pin source/destination directories; open and verify the replacement; open the previous source with read/delete access and deny other writers/deleters; require its expected hash; record its Windows volume/file identity durably; then rename that exact handle using `SetFileInformationByHandle` with replacement disabled. No generic copy/delete or overwrite fallback exists.
- **Previously missing old source:** a safe snapshot copy can provide the archived old version without interpreting the missing path as permission to delete anything.

After a crash between rename and DB commit, recovery verifies destination hash **and persisted file identity**, requires the previous source to be absent, and finishes metadata only. A different existing file, even with identical bytes, is a conflict without ownership proof. Snapshot/archive collisions, unexpected source changes, locked files and unsupported/cross-volume moves remain pending; originals and immutable copies are retained. Network/removable filesystems that do not support these primitives fail safely instead of weakening the protocol.

`Old Context Files` is excluded case-insensitively from discovery and event processing, including manually edited archived files. Opening an archive verifies its hash and reports external edits; its independent snapshot is unaffected. User archives and captured snapshots are never automatically purged when sources disappear or projects are removed.

## Validation and future work

Safety tests use temporary databases/roots and real Windows ACL, sharing-lock, junction and native-watcher fixtures. Crash tests interrupt capture before/after finalization and archive after handle rename but before the DB update. Frontend tests exercise consent separation, selection, matching decisions and history actions without building a GUI automation harness. See [verification](verification.md) and the mandatory [manual testing guide](manual-testing-milestone-2.md).

Milestone 3 can add local content extraction/diffs and history comparison without changing version identity/storage. Optional cloud indexing and AI metadata belong to later migrations/services. No embeddings, vector stores, cloud sync, detailed diffs, AI summaries, rollback, signer or updater is implemented here.

Official references reviewed: [notify](https://docs.rs/notify/latest/notify/), [Rust Windows file options](https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html), [Windows rename semantics](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_rename_info), [SQLite transactions](https://www.sqlite.org/lang_transaction.html), and the existing Tauri/React architecture references from Milestone 1.
