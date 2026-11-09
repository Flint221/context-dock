# ContextDock app audit and implementation acceptance

Document version 3 preserves [audit v1](ContextDock_App_Audit_v1.md) as the 0.10.16 implementation record and [audit v2](ContextDock_App_Audit_v2.md) as the simplification plan. Google Drive was subsequently selected and the six-step implementation was authorized. This record covers the 0.11.0 preview on draft [PR 16](https://github.com/Flint221/context-dock/pull/16), with registry schema 18. Main was brought up to date before development; the new work remains isolated until live acceptance.

## Product outcome

ContextDock's active job is to monitor selected project files, identify each family's latest document and optionally publish that file to an ordinary Google Drive folder. ChatGPT remains the separate place to ask questions. Its Pro subscription and the chat's connected Drive session do not authorize the native application's background API access.

Projects opens first. Files & tracking shows one row per selected latest family, current filename/version, source location and plain status. Folder selection is collapsible; ambiguous matches require an explicit family decision. Drive publishing contains connection/destination state, publication/pause controls and an optional recycling choice. Unrelated new files still use the shared Pending approvals inbox. Current processes is under Optional tools in Settings.

The embedded Ask AI, OpenAI/vector-store controls, comparison/history browsers and source-archive workflow are removed from the active interface and native command surface. Their workers no longer start. The old cloud runtime remains only as a passive local-mutation gate. Existing legacy data and compatibility code are retained; this refactoring does not delete snapshots, source archives, chats, stored ownership IDs or remote OpenAI resources. Retiring their controls is not remote cleanup.

New tracking hashes stable sources and records lightweight version metadata. Same-filename saves update that metadata instead of accumulating immutable content copies. Clear numeric exports, including simultaneous v1/v2 arrivals, remain one family with the highest export active. Verified moves and renames retain source/family identities. Legacy unfinished capture/archive/restore work is paused by migration rather than allowed to resume the retired workflow.

## Six-step status

| Step | Implemented and locally verified | Remaining live evidence |
| --- | --- | --- |
| 1. Google connection | Native Desktop OAuth, system browser, loopback callback, PKCE/state, cancellation, bounded timeouts, account binding, token refresh and Windows Credential Manager isolation. | New client creation, actual Google consent, refresh/persistence and disconnect/revocation with that client. |
| 2. Latest-file publication | Persisted root/project/file IDs, reserved identities before writes, raw Markdown/DOCX transfer, exact remote hash readback, stable provider revision checks and one file per family across versions/moves/renames. | App-owned live upload/readback and folder-picker access under its narrow scope. |
| 3. Automatic recovery | Stable-save coalescing, durable generations/outbox intents, bounded retries/provider delays, auth/conflict blocking, fair background scheduling and restart reconciliation. | Actual provider outages/expired authorization and interrupted live transfers. |
| 4. Simplified app | Latest-family interface, retained selection/move/recovery protections, no active embedded AI/index/archive workers, metadata-only new tracking and safe project unregistration. | Upgrade acceptance against a reviewed copy of the real profile; no installed profile has been migrated. |
| 5. Pilot/resources | Actual isolated native app with valid Word and Markdown, simultaneous versions, hidden tracking, moves, restart, actual process-tree CPU/memory samples, power-state observation and battery-policy tests. | Live provider pilot, actual battery-saver transitions during publication, longer battery/discharge and large-project measurements. |
| 6. Optional recycling | Off by default, fresh remote verification, unchanged lower numeric versions only, guarded identity-bound safety move, actual Windows Recycle Bin receipt verification and interrupted-move recovery. | Full live-Google-to-recycling chain on disposable files. Real projects remain opted out. |

All six code stages have been implemented and tested. This is not a declaration that all live acceptance gates have passed.

## Publication behavior and duplicate prevention

The source of truth is the local selected working file. The native publisher updates the existing Drive object's contents and filename, preserving its ID/link. Destination account, project folder, document family, expected hash and generation remain persisted. A lost upload response is reconciled under the reserved ID before another write. A newer local generation can never inherit a stale synced label. External cloud edits, ownership/parent mismatch and missing previously verified objects stop publication for review rather than silently creating a replacement copy.

The earlier connected-tool test proved same-ID v1/v2 replacement and fresh ChatGPT readback for that connector; see [its report](ContextDock_Drive_Test_Report_v1.md). It does not prove the native client's permissions or transfer behavior. The implementation's `drive.file` scope can hide folders created by another client. Schema 18 therefore requires an explicit existing-project-folder selection or per-project creation choice before creating a project folder. A search returning no items is not absence proof. A regression test hides a pre-existing folder from listings, then adopts its selected ID and verifies no second folder is created. Once files are mapped, selecting a different destination is refused to preserve their identities.

Native changes wake reconciliation. After the source is stable and tracked, publication waits three seconds for the observed generation to settle. The worker's idle change-detection wake ceiling is five seconds. Five-minute metadata reconciliation repairs missed watcher events; it is not a five-minute delay before every upload. Unchanged files do not cause repeated source hashing, upload staging, token refresh or Google requests. Timings are design targets under normal conditions, not network/provider/ChatGPT visibility guarantees. The primary-source industry review remains in audit v1.

The adapter uses resumable upload initiation followed by a streamed content request. A restart preserves intent and file identity, reconciles remote content and can open a fresh session; it does not promise to resume an arbitrary byte offset from a persisted session URL. Session capabilities never enter SQLite, interface messages or logs. Cloud readback hashes stream bounded buffers. One selected document is processed per worker turn. Local-only mode makes no provider calls.

## Recycling and data preservation

Recycling is a separate saved opt-in. It requires a fresh cloud metadata/download/metadata check and an unchanged newest local file. Candidate sources must belong to the same family and normalized stem, have a strictly lower numeric version, remain selected, and match recorded bytes. Changed, ambiguous, excluded, unversioned or unsupported cases retain data.

Before moving an older source, the app persists a recovery intent, verifies its physical identity/hash under a guarded handle, and renames it without replacement into an excluded safety directory. Settings/account/generation/pause checks run again before the explicit Shell recycle transfer. The returned Recycle Bin item is checked for the same identity and contents. There is no permanent-delete fallback. Interrupted moves restore known safety files if their original paths are free; occupied or uncertain paths preserve available bytes and show an independent cleanup warning. The newest working file remains in place. See [the complete recycling boundary and restore location](ContextDock_Drive_Recycling_v1.md).

Existing saved material is preserved rather than automatically purged. The latest-file missing-path action saves an exclusion and retains document/source metadata and cloud IDs. Removing a project pauses/unregisters it and keeps content. Registering the same folder again starts with monitoring/publication off. Older binaries refuse schema 18; this is a forward-only registry upgrade and no automatic rollback is claimed.

## Resource observations and limits

The actual release-mode pilot used a distinct app ID, data directory and Drive vault, with disposable files. The 12-logical-processor ARM64 Windows laptop had about 15.61 GiB RAM. Approximately 31-second samples included the native process and six WebView2 descendants:

| Condition | Total machine CPU | Ending private memory | Ending summed working sets |
| --- | ---: | ---: | ---: |
| Visible latest list | 0.147% | 201.82 MiB | 476.11 MiB |
| Hidden in tray | 0.025% | 199.23 MiB | 475.71 MiB |
| Minimized with low-memory request | 0.038% | 189.92 MiB | 90.45 MiB |

Summed working sets include shared pages; they are not unique physical RAM. These runs differ in warm-up/source count and do not establish percentage savings against 0.10.16. WebView2's low-memory target is best effort and may involve paging; normal policy is restored on show/focus. Native monitoring and publication continue independently. The laptop was on external power and charge rose from 78% to 80%; battery discharge and watt-hours were not measured. Battery-saver policy tests prove write-pause/override behavior, not a battery-life improvement. See [raw-method details and limits](ContextDock_Drive_Pilot_v1.md).

Temporary upload copies are tied to pending document intents and replaced/discarded when superseded or completed. Failed pre-journal staging cleans up its newly created temporary copy. Unfinished legacy payloads are retained. A global temporary-storage quota and long-duration outbox/crash-orphan stress measurements are not established by this pilot. Source reparse points/placeholders remain conservatively excluded, and automatic recycling currently supports fixed NTFS volumes; unsupported locations keep their files.

## Verification and next gate

The final local 0.11.0 checks passed 189 frontend tests and 321 Rust tests, with eight opt-in integrations excluded from the default Rust run. Explicit disposable native tray/tracking/reopen/exit and two actual Recycle Bin checks were also run successfully. A combined Word/Markdown provider simulation and real loopback HTTP adapter tests cover exact bytes, IDs, moves, rename, offline/restart recovery, error classification and untrusted-session rejection. Formatting, lint, TypeScript/production frontend build and all-target Clippy with warnings denied passed. The production frontend JavaScript bundle is 316.02 kB (97.63 kB gzip).

The isolated pilot and local test suites do not certify the installed application, live Google account, current PR's Windows CI result or a released installer. Packaging and CI are reported separately when observed. No release tag has been published and no real profile has been migrated. The branch remains a draft pending live acceptance.

The next required input is the local path to a newly created ContextDock Desktop OAuth JSON. Follow [setup v2](ContextDock_Drive_Setup_v2.md), which includes both Drive and Picker APIs, then complete Google consent in the browser. Continue with disposable live Markdown/Word replacement, same-ID verification, restart/offline tests, a fresh ChatGPT connected-Drive readback and opt-in recycling last. Record provider and ChatGPT timing honestly. Only then apply the migration and release to real projects.
