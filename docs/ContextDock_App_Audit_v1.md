# ContextDock app audit and sync improvements

Version: 0.10.16. Registry schema remains 013.

The audit found reproducible problems in incoming version batches, moved-file identity, idle sync work and competing interface refreshes. Those paths have been changed and tested. This is a source audit with disposable integration tests and local Windows packaging. It does not establish that every installed workflow or live OpenAI account has passed acceptance.

## Scope and evidence

Reviewed registry/migrations, discovery, source matching, filename versions, capture/archival journals, folder recovery, exclusions, cloud scheduling/request guards/cleanup, extraction limits, chat/AI authorization, desktop/tray behavior and process discovery. Interface review covered navigation, unsaved-selection protection, folder selection, approvals, tracking/history, cloud readiness and desktop settings.

The starting checkout was clean on main at `98e08ec`. Changes are on `codex/sync-tracking-resource-audit`. No installed application database, project source, saved history, credentials or cloud resources were modified. Native integration tests use temporary databases and source folders; provider requests are mocked. The browser check used real interface modules with synthetic native responses.

## Findings addressed

| Area | Finding | Result |
| --- | --- | --- |
| Incoming versions | Initial tracking grouped numbered exports, but later batches were rejected as ambiguous regardless of their clear order. | Incoming sources are processed in numeric filename order. Distinct numbered exports in the same folder/format can join one family. Unrelated new families still require saved selection or approval. |
| Version conflicts | Equivalent numbers such as v2/v2.0 cannot establish an order. | Conflicting equivalent numbers remain excluded or require review. Unnumbered/date-only ambiguity, different families and excluded folders retain their protections. |
| Identical exports | An identical-byte higher export could be treated as a duplicate copy. | A uniquely matched numbered export retains its family with its own filename-version identity. A live unnumbered copy still needs review. |
| Filename parsing | Ordering and display recognized different suffix patterns. | Ordering uses the same labels as display, including parenthesized labels and trailing dates. |
| Moves/renames | Matching bytes at another path left the original registered as missing. Moving a numbered folder could also send older members into backwards-version review. | A unique, verified, missing source is rebound to its new path with its original UUID, family, current pointer and snapshots. Both current and superseded members can move. Existing indexed File/version identities remain usable without reuploading. |
| Directory events | Directory-only events did not cover children whose size/timestamp matched the saved observations. | Dirty directories cover descendant paths using normalized separators. |
| Lock recovery | Watcher retries stopped after three failures until another event occurred. | Retries continue at 2, 5 and 15 seconds, then every 30 seconds while reconciliation requests another attempt. Disabling tracking or losing the root still prevents unsafe work. |
| Missed events | Healthy watchers lacked periodic reconciliation after startup. | Hourly metadata reconciliation recovers missed events without forcing a rehash of every unchanged source. |
| Idle CPU/I/O | The scheduler repeatedly queried families and opened selected sources about every two seconds. Notification probes rewrote unchanged signatures. | Idle checks wait for an external SQLite change, a retry/indexing deadline or a local safety probe. Unchanged notification signatures no longer write. Eligibility reuses a family-query result and selected membership uses a set. |
| Worker wakeups | Cloud/desktop workers woke every 100 ms during their two-second wait. | Workers park for that interval; explicit shutdown wakes them. An idle file monitor waits up to one second for native events, which wake it immediately. |
| Battery saver | Upload scheduling ignored Windows battery saver. | A default-on setting pauses new uploads during saver. Users can disable it. Manual pause is separate; tracking, existing indexing checks and confirmed removals continue. |
| Interface refreshes | Event bursts started competing reads; other projects refreshed the current cloud panel; late tracking reads could replace verified results. | Event reads are coalesced, with one running and a trailing refresh for later changes. Project cloud subscriptions filter unrelated events. Hidden-document reads wait for visibility. Sequence guards discard stale tracking responses. |
| File counts | Physical files were labeled as separate documents even within one family. | Cards/lists say source files and explain that filename versions are grouped under Document tracking. |

Move reassociation requires verified identical bytes, an actually absent original, a compatible filename version and one eligible candidate. Filename similarity alone is insufficient. Existing histories are not deleted or silently merged. Unfinished journal operations prevent automatic relocation reassociation.

## Sync timing and industry practice

There is no universal five-minute sync rule. Established tools combine change-driven work, settling periods, queues, paced retries and periodic verification.

| Source | Documented behavior | Implication |
| --- | --- | --- |
| [Dropbox sync status](https://help.dropbox.com/sync/check-sync-status) | Desktop updates respond to added/updated files; prioritized Sync next starts immediately. | Saved changes should begin syncing promptly. |
| [Syncthing synchronization](https://docs.syncthing.net/users/syncing.html) | Default watcher batching is 10 seconds, with hourly full scans and longer deletion delays. | Responsive detection can coexist with settling and periodic fallback. |
| [OneDrive pause/resume](https://support.microsoft.com/en-us/onedrive/how-to-pause-and-resume-onedrive-sync) | Sync can pause automatically for battery saver or a metered connection. | Power/network conditions should influence work. This release adds battery-saver handling. |
| [Windows power status](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-system_power_status) | SystemStatusFlag indicates battery saver. | Read this native signal without WMI/helper processes. |

Recommendation: keep automatic sync responsive after a stable save; reserve longer intervals for safety checks and retries. A five-minute upload timer would delay work without fixing identity or retry bugs.

ContextDock publishes immutable captures to an OpenAI retrieval index. It is not a bidirectional filesystem mirror. Provider indexing time is separate from local detection and upload scheduling.

| Work | Timing in this release |
| --- | --- |
| Native change batching | About 950 ms quiet time, with a four-second maximum burst window; unstable files are still deferred by capture checks. |
| Cloud scheduling | Two-second ticks; committed registry changes invalidate idle checks. Multiple phases/provider processing take additional time. |
| Unchanged readiness | Local safety probe after at most 30 seconds; recognized editor-busy errors use five seconds. |
| Provider indexing | Existing 15-second polls and processing bounds retained. |
| Remote verification | Existing roughly 15-minute automatic reconciliation policy retained. |
| Missed local events | Hourly metadata reconciliation, plus startup/manual reconciliation. |

## Resource assessment and remaining work

The changes remove avoidable probes, writes, overlapping event reads and wakeups. Parked cloud/desktop waits reduce scheduled wakeups in each waiting worker from roughly ten per second to one every two seconds. This is a code-path improvement, not a measured CPU or battery percentage.

No before/after battery, idle CPU or full process-tree RAM benchmark was completed. Native process memory alone omits WebView2. The saver UI was verified against synthetic state; actual Windows power transitions remain an installed-device check.

Remaining work, in priority order:

1. **Measure installed behavior.** Compare native/WebView2 CPU time, private memory and working set during idle, large-folder refresh, repeated Word saves, indexing and minimize/reopen. Include equivalent AC/battery workloads before making savings claims.
2. **Reduce upload peak RAM.** `cloud::verified` still reads a whole permitted snapshot into a byte vector before extraction/upload. The normal limit is 50 MiB; the configurable ceiling is 512 MiB. Streaming must preserve hash verification, extraction bounds and uncertain-upload recovery.
3. **Make large-folder reconciliation incremental.** Reconciliation still refreshes the whole directory and local capture holds the registry mutex. Large/slow external or network roots can delay requests. Move expensive I/O outside the lock with revision checks before changing this boundary.
4. **Add metered-network policy and optional bandwidth limits.** Battery saver is implemented; metered-network handling is not.
5. **Reduce frontend startup work.** The production entry remains approximately 504 kB minified and triggers Vite's 500 kB warning. Lazy-loading diagnostics and project analysis/history is a candidate for a measured startup comparison.
6. **Bound status delivery and plan storage maintenance.** Some views materialize all sources/history/jobs even when the UI pages rows. Snapshots are retained by design. Failed staging reads also need a separately tested orphan-stage cleanup/reclamation policy that protects captured and journal-owned files.
7. **Review previously split families.** Future clear batches are fixed. Existing complex/cloud-linked separate histories remain retained. A reviewed merge must reconcile citations, jobs, aliases and snapshots before combining them.

Moves outside the registered root require Reassign folder. Moves combined with edited bytes, multiple identical missing candidates, excluded destinations or unfinished operations can require review; these cases are not automatically repaired.

## Validation and delivery

- Native suite: **265 passed, six explicitly ignored**. Added coverage includes batches, moves, directory events, idle deadlines, unchanged notification probes and power preferences. Existing migration, archival, cleanup, extraction and authorization suites pass.
- Explicit Windows tray/hidden tracking/reopen/guarded-exit fixture: **passed separately**, using a blank disposable native window.
- Frontend: **167 passed across 19 files**. Added checks cover burst/trailing refreshes, project filtering, hidden-window deferral, unmount cleanup, stale reads and separate saver/manual pause controls.
- TypeScript, ESLint, rustfmt, all-target Clippy with warnings denied, whitespace checks and production compilation passed.
- Production npm dependency audit: **zero reported vulnerabilities**. This is not a Rust advisory audit or penetration test.
- Browser: real Settings modules rendered with synthetic responses; toggling the saver preference removed/restored the pause banner. No key was entered and no external API was contacted by the fixture.
- Windows x64 NSIS packaging passed. Executable product/file versions are **0.10.16**. No new schema migration.

Installer: `src-tauri/target/release/bundle/nsis/ContextDock_0.10.16_x64-setup.exe`.

Size: **4,806,260 bytes**. SHA-256: `FA436A252376A0E4BC04346E810BE26A03D4FE31B8C76D5FB95AE684143BEDE6`.

The installer has not been installed against user data. Authenticated cloud acceptance, physical external-drive behavior, actual power transitions and performance measurements remain separate checks. No completed GitHub workflow is asserted by this local report.

## Installed acceptance

Use a disposable project and explicitly approved cloud selection:

1. Add `ContextDock_Audit_Sample_v1.md` and `_v2.md` together, approve together and verify one family with v2 active. Repeat with v3/v10 after tracking is enabled.
2. Move the files to another included subfolder. Verify old missing rows disappear, identities/history remain and indexed files are reused. Repeat with an excluded destination and verify it stays excluded.
3. Keep a selected Word file locked through several retries, then close it. Verify capture/sync resume without an extra filename version or manual Sync command.
4. Verify current-only/full-history indexing, replacement, retry and removal. Ready status must wait for provider verification.
5. Enable actual Windows battery saver; verify deferred uploads. Turn saver off and verify resume. Confirm a manual pause persists across the transition.
6. Minimize/reopen during a batch and switch tabs. Verify final status refreshes, draft choices remain and other-project events do not disrupt the view.
