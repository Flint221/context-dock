# ContextDock app audit and simplification plan

Document version: 2. Supersedes the active recommendations in [audit v1](ContextDock_App_Audit_v1.md), which remains the historical implementation record for 0.10.16.

## Current decision and scope

The user wants ContextDock to focus on identifying the latest project documents and keeping files organized. They find the current cloud workflow more tedious than helpful and prefer using their ChatGPT Pro subscription for questions. They no longer see enough value in permanent old-context copies or the embedded Ask AI feature.

Confirmed destination direction: a regular cloud folder, such as OneDrive or Google Drive. The exact provider has not been selected; if Space access is a priority, Google Drive is the first provider to evaluate because the Space documentation explicitly describes that connected-source route. A later question also asks whether ChatGPT Space or the built-in Library can be integrated directly. That is evaluated below as an optional handoff; it does not silently replace the confirmed regular-cloud-folder direction.

This update is a plan. The previously audited changes were merged first; no simplification, cloud transfer, data deletion, new connection or installer execution is authorized in this planning task. Existing local snapshots, archives, chats, API keys and remote resources remain untouched.

## Recommendation

Make ContextDock responsible for three things: identify the latest selected document, publish that file to an optional destination, and recycle a clearly superseded local file only after the replacement is verified remotely. ChatGPT handles document questions.

Remove app-managed historical content and AI retrieval from the normal workflow. Keep lightweight metadata, safe file handling and a durable operation journal. A simpler interface must be backed by a simpler execution path; merely hiding history and AI panels would leave their workers, storage and failure modes in place.

## Reassessment of the core functions

| Current function | Recommendation | Reason and boundary |
| --- | --- | --- |
| Project registration, browse/open folder, rename/remove registration | Keep | These identify the source of truth. Removing registration must not silently remove source or cloud data. |
| Folder discovery, selection and saved exclusions | Keep and combine | One saved selection controls tracking and optional publication. No separate cloud-document checklist. |
| Filename version detection, family grouping and duplicate checks | Keep | This is the app's main value. Numeric versions, file type and folder scope matter; equivalent labels and ambiguous copies need review. |
| Stable source IDs, verified move/rename detection and folder reassignment | Keep | Moves must not create independent families or erase identity. A root moved outside the known location still uses reassignment. |
| Native watcher, settling checks, retry/backoff and missed-event reconciliation | Keep | Work follows changes and recovers from locks without continual whole-file work while idle. |
| SQLite registry and operation journal | Keep, reduce content scope | Persist metadata, generation IDs, hashes, destinations and receipts; do not require a complete saved document archive to track a file. |
| Retained immutable revisions of changed saves | Retire for future work | Use a bounded temporary upload copy when needed. Do not grow permanent history on each Word save. Existing snapshots require a separate migration choice. |
| Filename-version history browser and local comparisons | Remove from the main product | Their value depended on retained historical bytes, which the user no longer wants as a core service. Export legacy evidence before eventual retirement. |
| Old Context Files archival, snapshot-copy archival and restore/organize controls | Replace | Offer optional Recycle older source files after verified publication. Do not create new permanent archive folders in that mode. Existing archives are not automatically purged. |
| Current OpenAI Files/vector-store indexing | Retire as the default cloud destination | It serves retrieval and introduces indexing, association, retention and cleanup states. It does not implement publication to a ChatGPT Project or ordinary cloud folder. |
| Ask AI, saved chats, project intelligence, baselines/reports, model/token/cost controls | Remove from the active interface and runtime | The user prefers ChatGPT. Preserve/export existing material before any later deletion. |
| Advanced cloud index, Test Search, raw index jobs and current/history policy choices | Remove with the retrieval workflow | A file publisher needs upload/verification status, not indexing and semantic-search controls. |
| Cloud publication | Keep optional, latest only | Start with one provider and one destination per project. Avoid a bidirectional general-purpose sync engine. |
| Pending approvals | Fold into the file view | Show unrelated files and ambiguous matches inline. A clear new version of an already selected family should not prompt repeatedly. |
| Dashboard, recent files, source list and separate family inventory | Consolidate | Open on Projects. Use one primary latest-document list and a collapsible selection tree rather than several competing inventories. |
| Current processes, local previews, process stopping and startup inventory | Move out of the core | They are useful developer utilities, but do not help track or publish context files. Consider a separate optional Tools surface rather than a main navigation destination. |
| Tray, optional Windows startup, battery saver and pause/resume | Keep | They support quiet background tracking/publication. They must control ContextDock's work, not pause or stop the user's entire OneDrive/Drive client. |
| Notifications and activity | Keep, narrow | Notify about failures or decisions that need action. A compact recent-operation log replaces multiple permanent activity/history views. |
| Theme, keyboard navigation, progress/error feedback and unsaved-selection guard | Keep | These reduce friction without adding a second data-management system. |

## Cloud destination and ChatGPT integration

### Regular cloud folder: primary proposed destination

Use an ordinary cloud folder for original Markdown/DOCX files. Keep original version-bearing filenames and separate families that happen to share a basename in different source folders. Do not convert DOCX files into Google Docs as part of backup verification.

There are two implementation levels:

| Level | What ContextDock does | What it can claim |
| --- | --- | --- |
| Simple handoff | Atomically writes a verified copy into an existing cloud client's destination folder. | Copied to sync folder. This is not proof the provider received it; automatic recycling stays blocked. |
| Verified publication | Adds a narrowly scoped adapter for the chosen provider, records remote identity/revision and confirms the uploaded content. Existing desktop-client transfer may be reused only if that exact remote item can be verified. | Uploaded and verified. Optional recycling can be considered under the separate policy. |

Recommendation: choose one provider before implementation and prove the complete verification path. Delegate transfer to an existing client where practical, but do not treat a local copy, elapsed timer, path in OneDrive, or success toast as a remote backup receipt. If a supported provider checksum is sufficient, compare it with the corresponding local checksum; otherwise stream a remote readback and compare SHA-256. Tie the proof to the exact item and generation. Metadata size, filename or an ETag alone does not prove matching bytes.

[Microsoft Graph file metadata](https://learn.microsoft.com/en-us/graph/api/resources/file?view=graph-rest-1.0) exposes hashes when available. [Google Drive file metadata](https://developers.google.com/workspace/drive/api/reference/rest/v3/files) includes content checksums for stored files. These are provider capabilities to evaluate, not evidence that either integration is already implemented.

Keep the destination outside monitored source roots where possible; otherwise explicitly exclude the app-owned output and staging paths. A filename move into an excluded destination must not bypass saved exclusions. OneDrive/Drive placeholders and reparse points need provider-aware handling without weakening existing junction/traversal protections.

### ChatGPT Space and Library: possible handoff, not yet a background sink

The current app's OpenAI API files and retrieval index are separate from ChatGPT project sources. [Projects documentation](https://learn.chatgpt.com/docs/projects) describes uploaded/connected context; [Space documentation](https://learn.chatgpt.com/docs/space) describes adding source files or links and using connected services such as Google Drive.

There is a documented Library integration beyond vector stores: a ChatGPT-hosted plugin can use the optional upload helper with its Library option, and supported hosts expose a picker/download helper. Availability must be detected in the actual account. This is a host-side, user-selected-file workflow; it does not establish a native background upload endpoint. See the [plugin file API reference](https://developers.openai.com/plugins/reference).

This chat has connected Space tools that can upload a selected local file to an authorized Page and return a persistent reference, SHA-256 and access confirmation. The currently exposed tool has a 10 MiB limit; that is a limit of this connected tool, not a verified Library-wide limit. That demonstrates a chat-assisted capability, not a public endpoint that can simply be embedded in ContextDock. No upload was performed or account capability tested in this task.

A small future proof of concept could let ChatGPT request the latest selected file from a read-only ContextDock MCP connection, then use the available host's Library/Space flow. Validate file types/sizes, persistence, readback, source placement and receipt handling with a disposable file. Do not assume Library upload automatically attaches a file to a particular Space or Project, replaces older copies, or provides a documented deletion API.

[MCP plugins](https://developers.openai.com/api/docs/guides/custom-mcp-server) are a supported way to expose app data to ChatGPT. Local/public distribution has its own restrictions; the [desktop-only submission guide](https://developers.openai.com/plugins/deploy/desktop-only) currently requires approval for public local-MCP submissions. Personal setup and actual account access need separate validation. Use supported ChatGPT-host and MCP authorization for a companion integration.

The subscription supports the user's ChatGPT workflow; it is not itself a general storage credential for ContextDock. Current [API credit documentation](https://developers.openai.com/api/docs/guides/chatgpt-api-credits) also describes benefits eligible subscribers may claim, but account eligibility was not checked. Credits do not establish Space/Library access or make retaining embedded AI useful for this user.

A Google Drive source link may provide the simplest Space handoff without building a companion plugin. However, a copied new filename does not update a previously uploaded attachment or an old source link automatically. Define how the current item/link is refreshed and show its actual version; do not claim that existing ChatGPT conversations silently switch to the newest file.

For the first simplified release, use the regular cloud folder as the verification/cleanup authority. Treat ChatGPT/Space/Library as an optional convenience until an equivalent supported persistence/readback contract is proven. Merely exposing a live local file to ChatGPT is not a retained cloud backup.

## Proposed upload-before-recycle workflow

Example: a selected Project_Context_v1.md already exists and Project_Context_v2.md appears.

1. Detect the new source, wait for a stable readable save, and determine that it belongs to the same family. Version order comes from the filename; modified time alone does not choose a winner.
2. Create a durable publication generation and, if needed, one immutable temporary upload copy. Record its hash, source identity, selection revision, intended destination and older candidates. Do not create permanent copies of all prior saves.
3. Publish the newest eligible generation. Keep older sources and the previous verified cloud item while upload or verification is incomplete.
4. Confirm remote identity, generation and matching content. Persist the verified receipt before advancing the latest pointer.
5. Recheck that this remains the family's newest selected generation and that the root, destination and saved policy have not changed.
6. If recycling is enabled for this project, recheck each older candidate's exact identity/hash and current absence from the active/latest set. A changed, locked, ambiguous or moved candidate remains untouched for review/retry.
7. Send only eligible older local source files to the Windows Recycle Bin and record the actual result. Never recycle the newest local working file. A failed/unsupported recycle operation leaves the old file in place; it must not fall back to permanent deletion.
8. Mark the cleanup receipt and release unneeded app-owned temporary bytes. Retain lightweight operation evidence for restart and deduplication, rather than a full historical document archive.

Default: recycling is off until the user explicitly enables it for a project. The recommended product does not expose permanent deletion as the normal automatic behavior. Windows offers [recycle operation flags](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ifileoperation-setoperationflags), but actual behavior/receipts must be tested on the target volumes, including removable and network locations.

Local source recycling and cloud cleanup are separate policies. Latest-only publication does not authorize deletion of unrelated remote files or old copies uploaded manually to ChatGPT. If old app-owned cloud copies are later trashed, do it only after replacement verification and only by verified item/revision ownership. Reused remote IDs must never cause cleanup to remove the current replacement. Recovery then belongs to the provider's normal trash/versioning policy, not a new ContextDock archive.

## Failure and race rules

| Event | Required behavior |
| --- | --- |
| v1, v2 and v10 arrive together | One family when unambiguous. Publish the highest stable selected version; do not require three intermediate cloud uploads. |
| Another save of the same v2 filename | Update that generation's content as needed; never recycle the current path or count every save as a new authored version. |
| v2 and v2.0 conflict, version decreases, or copy matching is uncertain | Require inline review; never guess a recycle candidate. |
| v3 appears while v2 uploads | Preserve/reconcile the in-flight result, but v2 cannot authorize source cleanup or replace a newer latest pointer. Queue the newest generation and discard obsolete unstarted work. |
| Network/authentication fails or remote outcome is uncertain | Keep original sources and the prior verified cloud item. Recover by durable job/remote identity before retrying; no blind duplicate uploads. |
| A local cloud-folder copy exists but sync is pending | Show handoff status, not Uploaded and verified. Keep older sources. |
| Provider contents change or destination/account is reassigned | Invalidate the old proof and recheck the new target. Do not use stale receipts to authorize cleanup. |
| Old file is edited, replaced at the same path or reopened during cleanup | Recheck identity and bytes; leave it in place if the safe-operation contract cannot be met. |
| Cloud/source roots overlap | Protect/exclude app-owned output and staging; avoid detection loops and accidental deletion propagated by the cloud client. |
| Folder/drive disappears, placeholder cannot be read, or destination is excluded | Stop the affected operation, retain state and show the next useful action. |
| App exits or crashes between upload, verification and recycling | Resume from recorded receipts, recheck eligibility, and never repeat a destructive step against a replacement file at an old path. |
| Recycle Bin unsupported, full, denied, or operation cancelled | Keep the old source and report Cleanup pending. No permanent-delete fallback. |
| Library/Space helper unavailable, upload exceeds its limit or result is unreadable | Leave files in place; do not label the handoff a verified backup. |

## Minimal interface

Main navigation: Projects and Settings. Each project has one latest-document list with family, active filename/version, source location and a plain status. Selection is a collapsible tree. Review decisions and failed operations appear beside the affected row.

Suggested statuses: Local only, Waiting for save, Publishing, Copied to sync folder, Verifying, Uploaded and verified, Cleanup pending, Paused and Needs review. Do not use Indexed for an ordinary file transfer, and do not show upload completion before remote verification.

Primary actions: Open latest file, Open source folder, Publish now/Retry and Pause. Project settings contain source selection, one optional destination and the separate recycle-after-verification preference. A Prepare latest files for ChatGPT action is a handoff, not in-app AI or a second permanent archive.

## Metadata, storage and resource design

- Store project/family/source IDs, normalized paths, author version labels, hashes, current generation and selected destination.
- Keep upload/verification/recycle receipts and a bounded recent-operation log; retain unresolved-operation metadata until resolved.
- Bound temporary content to the in-flight generation and newest pending candidate per family, with a global size limit. Coalesce repeated saves and obsolete queued generations.
- Stream hashing/transfer/readback with bounded buffers. Remove whole-file Vec uploads and DOCX extraction from the normal publisher path where no text analysis is required.
- Keep monitoring event-driven, avoid continuous requests when publication is disabled, and respect battery saver for new transfers/heavy verification.
- Load the active project on demand and page large lists. Removed panels must no longer mount, subscribe, poll, recover AI work at startup or build large status payloads.
- Measure native plus WebView2 idle CPU/private memory, save bursts, large files, minimize/reopen and long offline sessions. Do not promise a RAM or battery percentage before measurement.

Existing code couples path selection, tracking and missing-file cleanup to cloud helpers and document_versions. First move shared database/path logic into neutral services and introduce a metadata-first pipeline. Do not simply remove cloud.rs, empty snapshot folders or drop history tables: old AI baselines, reports, chats, jobs and journals still reference them.

## Owning code to revisit during implementation

| Boundary | Existing code | Planned treatment |
| --- | --- | --- |
| File identity and watching | [tracking.rs](../src-tauri/src/tracking.rs), [monitor.rs](../src-tauri/src/monitor.rs), [matching.rs](../src-tauri/src/matching.rs), [registry.rs](../src-tauri/src/registry.rs) | Preserve the recent move/batch fixes; separate observed metadata from permanent capture. |
| Selection and status | [path_selection.rs](../src-tauri/src/path_selection.rs), [tracking_queries.rs](../src-tauri/src/tracking_queries.rs), [forget.rs](../src-tauri/src/forget.rs) | Remove dependency on retrieval eligibility and full historical-byte ownership for ordinary tracking. Preserve legacy cleanup rules separately. |
| Journal and filesystem operations | [operations.rs](../src-tauri/src/operations.rs), [storage.rs](../src-tauri/src/storage.rs), [archival.rs](../src-tauri/src/archival.rs) | Reuse durable intent and identity checks; add publisher/recycle receipts without requiring archive/snapshot history. Retain old operation recovery until migrated. |
| Retired retrieval and AI | cloud, cloud_sync, cloud_cleanup, cloud_commands, chat, ai and their companion modules | Keep cleanup/migration access until resources are resolved; then remove the active retrieval and embedded-model runtime. Comparison/extraction dependencies can be removed once unused. |
| Project interface | [App.tsx](../src/App.tsx), [ProjectDetails.tsx](../src/components/ProjectDetails.tsx), ProjectTabs, ProjectSummary, TrackingPanel, ProjectCloud, DocumentHistory and ProjectIntelligence | Consolidate selection/current-file status; retire AI/index/history interfaces and their subscriptions after backend separation. |
| Background and utilities | [desktop.rs](../src-tauri/src/desktop.rs), [startup.rs](../src-tauri/src/startup.rs), [processes.rs](../src-tauri/src/processes.rs) | Retain tray/startup/power support, narrow notifications and move process utilities outside the core. |

This is a refactoring map, not a list of files to delete immediately.

## Migration and implementation order

1. Approve the smaller product scope and select one regular-cloud provider. Keep the Space/Library proof of concept optional.
2. Implement a metadata-first tracking path and bounded temporary outbox behind a reversible migration. Preserve project/source/family IDs and saved exclusions. Test the recent move/batch fixes against it.
3. Retire the active AI, retrieval and permanent-history interfaces/workers. Preserve legacy data in place or export it through an explicit reviewed migration; do not auto-purge it. Retain a one-time legacy cleanup/export interface until owned remote resources and pending operations are resolved or intentionally retained.
4. Prove the chosen provider's generation-bound upload/readback receipts and restart recovery. Expose verified status only after that proof.
5. Add opt-in native recycling, identity/race checks and volume-specific acceptance. No automatic cleanup before this gate passes.
6. Simplify the interface, remove dead dependencies/code paths and run performance/upgrade acceptance.
7. Only if requested, test the companion ChatGPT handoff and later review deletion of legacy data.

Retire existing OpenAI cloud resources carefully: keep ownership IDs, credentials and the existing cleanup capability until the user chooses verified cleanup or intentional retention. Hiding the AI/cloud UI is not remote deletion and can leave chargeable resources behind. Existing snapshots, Old Context Files, chats and reports are not deleted merely because the future product will stop creating them.

## Acceptance gates for the future implementation

- One current family for clear simultaneous numeric exports; same-version saves do not inflate counts.
- Current source and latest verified remote content survive offline work, restarts, expired authentication and newer generations appearing mid-upload.
- No recycling without a current matching remote receipt, unchanged old-source identity/hash, saved per-project policy and a supported reversible operation.
- Same-path replacements, changed old files, locks, copies, exclusions, root moves, placeholders and unsupported recycle volumes cause retention/review rather than deletion.
- Copied to sync folder is visibly distinct from remote verification.
- Temporary content remains bounded through many saves, offline days and interrupted jobs; successful work does not create permanent historical copies.
- Existing projects, exclusions, legacy evidence and cleanup ownership survive migration; rollback/export behavior is reviewed.
- Hidden or removed panels do not keep reading data; local-only mode makes no provider requests.
- Any ChatGPT handoff proves actual file persistence/readback and source placement on this account before it can be considered a cleanup authority.

## Baseline and documentation validation

PR [#15](https://github.com/Flint221/context-dock/pull/15) merged as `72628b1fbdaf0477fe4d4234b67a7c3b6736c20f`. The checkout was switched to main and fast-forwarded; main and origin/main matched with no local modifications before this documentation update.

The first PR workflow failed because the Settings test still expected Version 0.10.15 after the release bump. Only that assertion was corrected to 0.10.16 in `1b783cf728767a25e470c43c318abd9a5c5f1508`; no runtime behavior changed. All 167 frontend tests then passed locally. [Windows CI run 42535683244](https://github.com/Flint221/context-dock/actions/runs/42535683244) passed TypeScript, ESLint, frontend tests, rustfmt, native tests, all-target Clippy, NSIS packaging and installer artifact upload before merge. The release/tag job was intentionally skipped for the untagged PR.

The baseline suite contains 265 passing native tests with six explicitly ignored entries, plus the separately passed disposable tray fixture recorded in v1. This documentation-only follow-up does not require repeating functional tests. Verify Markdown references, whitespace, changed-file scope and main/origin synchronization instead.

This planning update does not change application version, schema, installation, saved settings or user data. It does not implement the proposed removals or integrations. The implementation audit and installer evidence remain in v1; this document changes the product direction and next priorities.

Previously recommended optimization of retrieval/history features is superseded by evaluating their removal. The file identity, move safety, retry and resource findings that support the retained core remain relevant.
