# Architecture — Milestone 4

ContextDock 0.4.0 extends the existing Tauri/React application without changing `com.contextdock.desktop`, existing entity UUIDs, or the supported per-user application data location.

## Optional cloud indexing (Milestone 4)

`cloud_credentials.rs` persists secrets only through Windows generic credential `ContextDock.OpenAI`, using per-user Credential Manager and `CRED_PERSIST_LOCAL_MACHINE` (persistence on this computer, not machine-wide plaintext). Native buffers use `Zeroizing`; the CredRead blob is zeroed before CredFree. The uncontrolled password input clears immediately on Save and never receives a saved secret. No environment fallback, developer key, browser storage, secret-bearing logs or telemetry exists. SQLite stores a one-way credential fingerprint for binding, not the key. Same-user malware/debuggers are outside this OS vault boundary.

`cloud_api.rs` provides mockable `CloudApi`. Production reqwest uses fixed HTTPS `api.openai.com/v1`, normal Windows SChannel certificate validation, TLS 1.2 minimum, no redirects, 10-second connect/30-second request limits, bounded responses and a 90-second between-request reconciliation budget. The API's current vector-store compatibility header and upload `purpose=assistants` are retained. No Assistants, Threads, Runs, Responses, chat/completions or explicit embeddings endpoint is called. Native browser opening has a fixed allowlist of official account/docs URLs.

`cloud.rs` owns selection/preview/verification/SQL views. `cloud_sync.rs` owns provider-independent durable transitions. `cloud_commands.rs` exposes IPC and an app-lifetime single-request worker with its own gate and SQLite connections. HTTP/polling never holds the local Registry mutex or an SQL transaction. Local capture, archival and comparison remain independent. Worker shutdown does not wait for an in-flight HTTP request; precommitted intents allow recovery. Idle checks are local every two seconds. Automatic projects verify remote state roughly every 15 minutes; active enabled projects reconcile on startup. Completed manual-only projects do not continuously contact OpenAI. Pause/disconnect stops new requests after the current operation yields; already sent bytes cannot be retracted.

### Schema 003 and selection

The migration is transactional after 001/002 and preserves all existing UUIDs, current pointers, snapshots, sources, archives, history, themes and local tracking/archival preferences. Cloud defaults off, automatic cloud sync off and paused. A random installation UUID is generated once; newer schemas are rejected rather than downgraded.

| Table | Responsibility |
| --- | --- |
| `cloud_installation` | Installation ownership UUID |
| `cloud_connection` | Key fingerprint, test status/time and sanitized error; no credential |
| `cloud_projects` | Consent/configuration, key binding, store ID/name/create-operation UUID, start/pause/auto/mode/size, sync/reconcile/delete intents, usage/backoff |
| `cloud_documents` | Explicit selected logical-family UUIDs |
| `cloud_exclusions` | Explicit per-version removals that automatic sync must not undo |
| `cloud_jobs` | Unique project/version and File mappings, opaque upload filename, durable phase/state, desired/current flags, provider status/usage, retry/error |
| `cloud_activity` | Sanitized events; latest 30 displayed |
| `cloud_retrievals` | Returned File/family/version IDs, score/time; never query/full excerpt/chat history |

Insert/default triggers protect new/existing projects. Unregistration disables cloud and queued deletion without deleting mappings/resources. Re-register the same root to recover identity; cloud still requires re-enable. Selection refers to logical family IDs, not every file in a folder. Only tracked selected families with committed current snapshots, supported types and no pending ambiguity qualify. An ambiguity freezes selection sync rather than silently deleting known-good associations. Current-only uses `documents.current_version_id`, including reuse of older captured bytes, rather than assuming maximum version number is current.

Preview hashes bind draft family selection/mode/limit/auto policy to project revision, eligible snapshot IDs and known jobs. Apply uses a short immediate transaction, rejects stale previews and requires a tested key plus explicit retention/cost consent. Initial Start is separate. Resume before Start creates/reconciles only the store. Confirmed changes on started indexes queue the selection/policy delta; paused projects wait for Resume.

### Upload, processing and recovery

Before store creation, record `store_op` and `creating`; provider metadata carries installation/project/operation identity. Before upload, record a unique job, recovery phase and opaque `cd_<installation>_<project>_<version>_<job>.md/.docx` name. Persist the File ID before attachment. Snapshot reads pin directories, reject traversal/links, use Windows sharing protection, bound bytes and verify committed size/SHA-256. Missing/corrupt snapshots fail closed; there is no live-source fallback. Extraction is conservative screening only: original full Markdown/DOCX bytes, including any DOCX embeds/metadata, are uploaded unchanged. Limits: 16 MiB raw, 4 MiB text/total expanded DOCX, stricter configured soft size; provider type/5-million-token limits still apply. The soft limit is not a spending cap.

Queue: pending → uploading → uploaded → indexing → indexed. Indexed requires a **completed vector-store association**, not successful File upload or deprecated File status. Processing polls about every 15 seconds. Attach checks the known association/File first. Failed processing stops until explicit Retry/Reconcile, which may replace an owned failed/cancelled association while reusing its File. Six bounded transient retries use exponential delays and seconds/date Retry-After (capped at one hour). Auth/permission/permanent failures need user action. Mutating timeout/5xx/partial responses become uncertain rather than blindly reposting.

Interrupted creates/uploads list candidates by recorded operation metadata or unique filename/purpose/size. A uniquely recovered File must also pass downloaded SHA-256. Missing permissions, multiple candidates or no provable candidate leave actionable uncertainty. Only after the user verifies the old request ended and the correct dashboard has no copy does typed **RETRY ABSENT OPERATION** permit another checked attempt. This is an explicit operator decision, not a universal exactly-once guarantee; delayed/inaccessible resources require account reconciliation. Definitively rejected/unsent requests can retry normally.

Every active operation verifies store ownership. Key replacement needs Test plus Verify key for each existing index before rebinding; inability to access a store does not authorize creating another. Reconciliation compares actual associations/statuses/usage, restores missing attachments with known Files and stops on foreign/unrecognized associations. Missing/expired stores never silently recreate: explicitly acknowledge/delete the mapping, prepare a new index, then review consent. Missing uploaded Files have a checked explicit recovery path.

Current-only retains old associations until **all desired replacements complete**, then detaches obsolete associations and updates `is_current`. Full history keeps selected captures with current metadata. Mode/selection changes need another preview/confirmation. Automatic sync off prevents newly captured versions from queuing until Sync Now; already authorized queued work can finish unless paused. Explicit per-version removal creates an exclusion and never changes local tracking/history. Removed/excluded captures are not automatically restored.

### Retrieval, deletion and cost boundaries

Direct search requires desired snapshots already indexed and sends only the explicit query to that store, with installation/project/current filters, `rewrite_query=false`, at most five results. IDs/attributes must map to that project's known jobs; unrecognized provenance is rejected. Excerpts render as text, not HTML. Full-history mode includes historical selected captures. No generated interpretation or cross-project synthesis exists.

Association DELETE, vector-store DELETE and File DELETE are separate. Automatic replacement/deselection detaches and retains underlying Files. Strong-confirmation store deletion verifies every association, refuses foreign/unknown ownership, detaches owned versions and deletes the store, retaining File IDs for explicit cleanup. Partial deletion preserves retryable identifiers. Local sources, snapshots, archives/history are untouched. Disable/key disconnect/unregistration/uninstall do not authorize remote deletion.

Separate File purge needs a removed owned job, exact typed File ID, filename/size/downloaded-hash checks and no references in **accessible** vector stores. Pagination is bounded at 100 pages of 100, with malformed/incomplete listings or deadline exhaustion failing closed. The API cannot enumerate every unrelated tools/threads/batches/other-credential reference; the user must attest to checking those separately. Lack of access is not proof of absence. Purge is never automatic. Clean remote data before unregistering, or re-register the same root first.

Settings/project views distinguish raw snapshot bytes from last measured OpenAI `usage_bytes`, which includes parsed/indexed storage. They link to authoritative usage/billing/pricing instead of estimating dollars or promising a permanent free allowance. Account allowances may be shared. Files/stores follow provider retention until deletion; processing, visibility, dashboards and billing can lag. API data is not used for training by default. Consult the current provider policy rather than assuming immediate erasure.

Endpoints: `GET/POST /vector_stores`, `GET/DELETE /vector_stores/{id}`, `GET/POST /vector_stores/{id}/files`, `GET/POST/DELETE /vector_stores/{id}/files/{file_id}`, `POST /vector_stores/{id}/search`, `GET/POST /files`, `GET/DELETE /files/{file_id}`, `GET /files/{file_id}/content`. Test uses only a minimal vector-store listing.

Official sources checked October 19, 2026: [Retrieval](https://developers.openai.com/api/docs/guides/retrieval), [Files](https://developers.openai.com/api/reference/resources/files/methods/create), [store create](https://developers.openai.com/api/reference/resources/vector_stores/methods/create), [search](https://developers.openai.com/api/reference/resources/vector_stores/methods/search), [detach](https://developers.openai.com/api/reference/resources/vector_stores/subresources/files/methods/delete), [File delete](https://developers.openai.com/api/reference/resources/files/methods/delete), [data controls](https://developers.openai.com/api/docs/guides/your-data), [pricing](https://developers.openai.com/api/docs/pricing). No live indexing/key access was used during development; the [14-test installed-app guide](manual-testing-milestone-4.md) supplies user-run acceptance.

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

Milestone 3 adds the read-only comparison path below without changing version identity/storage. Optional cloud indexing and AI metadata belong to later migrations/services. No embeddings, vector stores, cloud sync, AI summaries, rollback, signer or updater is implemented here.

Official references reviewed: [notify](https://docs.rs/notify/latest/notify/), [Rust Windows file options](https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html), [Windows rename semantics](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_rename_info), [SQLite transactions](https://www.sqlite.org/lang_transaction.html), and the existing Tauri/React architecture references from Milestone 1.

Interrupted archive copies with a pre-existing, unfinalized temporary are retried using a newly journaled private temporary name. The earlier partial bytes are retained, never truncated or automatically deleted. This also avoids mistaking an unrelated staging-name collision for application ownership.

## Local comparison (Milestone 3)

`comparison_models.rs` defines format-version-1 IPC payloads; `comparison.rs` resolves stored version UUIDs and calculates exact line edits; `extraction.rs` extracts bounded Markdown/DOCX content. `src/comparison.ts` mirrors the contract. `VersionComparison.tsx` replaces the contents of the existing history dialog, without stacking another modal. The Milestone 2 tracking/documents sibling-key fix remains intact.

### Identity, synchronization and read-only verification

`compare_versions(earlier, later)` acquires a single comparison permit (concurrent requests receive a retry message). A short blocking registry lookup obtains both immutable version records and the family's name/type. Different document IDs are rejected. Reversed selections are ordered by internal version number. Self-comparison is allowed for identical-content testing without creating duplicate captures.

The lookup returns an owned `ComparisonJob` and releases the shared registry mutex. A separate blocking worker resolves both snapshot paths with existing `storage::checked`, pins directories, rejects traversal/reparse escapes, and opens ordinary read-only handles through `locked_read`. Windows sharing denies writers/deleters. Each bounded buffer is SHA-256 verified against the stored hash before extraction; the exact verified buffer is the extraction input, avoiding a second unverified path read. Disappearance/locks/integrity conflicts abort the result. No active-source or archive fallback exists. No comparison path calls capture, reconciliation, journal recovery, SQL writes or archive operations. Schema 2 and `com.contextdock.desktop` remain unchanged.

### Extraction policy

Markdown is strict UTF-8, with optional BOM removed for text comparison and CRLF converted to LF. Warnings identify normalization for each side. All other text/whitespace, blank lines, markup and trailing-newline presence remain meaningful. No Markdown rendering or formatting is performed; line contents display literally. Invalid UTF-8/NUL/UTF-16 produce explanatory errors. Byte identity is computed separately from normalized text equivalence.

DOCX uses `zip` 9.0.5 with only Deflate support and `quick-xml` 0.42.0 without external-resource or encoding features. An EOCD/central-directory preflight bounds entry allocations before ZIP parsing. Only `word/document.xml` is decompressed; entries are never extracted to disk. XML parsing is streaming and namespace-aware, accepting transitional and strict WordprocessingML. DTDs/custom entities are rejected. Standard/numeric character references and Unicode are preserved. No relationships or URLs are fetched.

| Structure | Text policy |
| --- | --- |
| Body paragraphs and headings | Paragraph order, newline after each paragraph; style ignored |
| Text split across runs | Concatenated without artificial separators; bold/italic/run changes alone do not change text |
| Breaks, tabs, nonbreaking/soft hyphens | Explicit characters retained |
| Tables | XML cell order; tabs between cells, newline between rows; paragraph breaks within cells retained |
| Merged/complex cells | No visual grid reconstruction; XML order only; nested tables fail explicitly |
| Hyperlinks | Visible label text included; targets never opened or compared |
| Fields/generated content | Stored displayed `w:t` result included; instructions ignored and never evaluated; generated numbering not reconstructed |
| Tracked changes | Accepted-text view: insertions/move-to included, deletions/move-from excluded; no revision metadata/Word Track Changes comparison |
| Headers/footers, footnotes/endnotes, comments | Excluded; other package parts are not decompressed |
| Images/charts, drawings/text boxes, embedded objects, altChunk | Excluded with scope disclosure; no execution/rendering |
| Formatting, hidden-text visibility, layout | Not compared; body text included regardless of run visibility properties |

The result always includes the DOCX scope disclosure. Equivalent text with different bytes means only equivalence under this policy, not identical Word documents. Malformed/missing bodies, invalid encodings and unsupported structures fail instead of returning a partial authoritative diff.

### Algorithm, payload and complexity

[`similar` 3.2.0](https://docs.rs/similar/3.2.0/similar/) uses deterministic Myers line diffing with standard-library deadline support. Myers handles insertions/deletions and shifted passages without naive positional comparison. A two-second calculation timeout bounds highly divergent inputs; because the library may return an approximate script on timeout, ContextDock checks elapsed time and rejects a deadline-crossing result. Accepted results contain the exact edit script. Modified passages appear as related removals/additions. Final-newline presence participates in comparison.

`Comparison` contains format version, family identity/name/type, ordered version IDs/numbers/source names/capture times/hashes, byte identity, text equivalence, added/removed totals, changed-block count, warnings, and ordered `DiffBlock`s. Blocks are unchanged runs or contiguous edits, with stable positional IDs, one-based old/new start lines and counts (zero count for insertion/deletion gaps). `DiffLine` has kind, nullable old/new numbers, literal text and final-newline status. Unchanged blocks supply expandable context; no changed section is silently truncated. Totals use the full native result, independently of the visible page.

Bounds: one comparison worker; 16 MiB per snapshot, 4 MiB extracted text per side, 20,000 lines per side, 64 KiB per line; 2048 ZIP entries, 128 MiB declared total expanded package, 32 MiB per entry, 200:1 ratio, 8 MiB decompressed body XML and nesting depth 128. Unsupported ZIP64/multi-volume layouts fail. Content is held in bounded memory, not persisted. No comparison cache exists: every request revalidates hashes. These are supported limits, not claims of arbitrary-size handling.

### UI and lifecycle

History offers Compare Versions with two or more captures and defaults to the latest two. Each selector shows internal number, capture date and original source name. Users can independently choose any historical pair; reversed inputs are explained and normalized natively. A single-version family gets explanatory text. The same version on both sides provides a controlled identical-content case.

Unified/split views show literal monospace text, line numbers and +/− markers as well as theme-compatible colors. Split rows pair replacement lines and retain blanks for one-sided edits. Previous/Next change selects and scrolls the corresponding region; context is three lines by default, expandable. Only 100 unified lines or split rows render at a time; paging exposes all remaining content within input bounds. Equivalent-text results show a clear state and separate byte status. Copy summary/changed text use the browser clipboard only after explicit clicks and copy the full result. Native file export is omitted.

The existing dialog remains the sole modal and restores focus to Compare Versions on return; the comparison back button receives initial focus. Closing/changing family invalidates pending UI responses. Errors clear loading, permit reselection/retry and retain Back/Close. History-load failures show retry instead of a permanent spinner. Dispatched native requests finish under their bounds even if the view closes; no cancellation API is introduced.

Primary dependency references: [similar algorithms/deadlines](https://docs.rs/similar/3.2.0/similar/), [zip reader](https://docs.rs/zip/9.0.5/zip/read/struct.ZipArchive.html), [quick-xml parser](https://docs.rs/quick-xml/0.42.0/quick_xml/). Focused tests and the [12-case manual guide](manual-testing-milestone-3.md) cover correctness and safety without a complex GUI harness.
