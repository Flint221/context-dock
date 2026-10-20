# Verification — ContextDock 0.4.0

Milestone 4 checks ran on Windows 11 on October 19, 2026, using the existing Node 24/MSVC Rust toolchain. Cloud tests use a fully in-memory scripted provider, temporary SQLite databases/source roots/snapshot stores and synthetic nonfunctional key strings. They never read/save a real Windows credential or contact the authenticated OpenAI API. Public official API documentation was consulted. User project folders, real application data, credentials and installed executables were not modified by testing or installer execution.

| Check | Milestone 4 result |
| --- | --- |
| TypeScript / ESLint | Passed |
| Frontend tests | Passed: 26 in 5 files (16 existing + 10 cloud UI) |
| Rust tests / binary targets / doctests | Passed: 83 Rust tests (37 existing + 46 cloud cases); no failures |
| rustfmt / all-target Clippy with warnings denied | Passed |
| Production Vite bundle | Passed through the NSIS build |
| npm audit | Zero reported vulnerabilities |
| Windows x64 NSIS | Built unsigned `ContextDock_0.4.0_x64-setup.exe`, approximately 4 MiB |
| Versions / identity | npm + npm lock, Cargo + Cargo lock, Tauri 0.4.0; `com.contextdock.desktop` and existing per-user paths retained |
| Migration | Transactional schema 2→3 fixture preserves tracking/archival preferences and defaults cloud off; existing schema-1/capture/archival regressions pass |
| Whitespace | `git diff --check` passed |
| Live OpenAI / actual Credential Manager persistence / installed upgrade | **Not performed by the agent**; user acceptance required |

## PR #4 initial-setup feedback

User testing showed that saving a key without the separate Test connection step left configuration unapplied and Start Initial Sync disabled. A failed confirmation stayed open but looked like a refresh. Global vertical label styling also displaced the document checkboxes from their family names.

Saving now explicitly performs a read-only key test through **Save and test key**. Project setup displays the connection status, ordered prerequisites, an explicit saved-key test/retry, selected-document count and Select all eligible documents. Empty initial selection or an untested key is blocked before consent opens, with native validation as well. Confirmation closes only after a successful save and refreshes enabled controls; failures explicitly say configuration was not saved. Test failure still refreshes saved-key state so retry remains usable. Checkbox labels now explicitly use horizontal flex layout with fixed-size controls.

Three added frontend regressions cover an untested connection and in-project retry, empty selection through successful modal close/Start unlock, and save success followed by test failure. Two native regressions cover rejecting empty initial selection while permitting deliberate deselection on a started project, and reporting an untested connection before consent without network calls. These are mocked tests; no real saved credential or authenticated cloud request was used by the agent.

## Milestone 4 coverage and boundaries

The 46 cloud tests cover local-only/paused no-request behavior, fingerprint isolation, plaintext input validation without echo, fresh and schema-2 migrations, project-scoped eligible selection, ignored/foreign family rejection, stale consent, no uploads before Start, idle index creation before Start, authorization rechecks before each individual request, bounded processing polls, missing/tampered snapshots, soft size limits, original snapshot bytes rather than live-source fallback, asynchronous processing completion, repeated-sync deduplication, current-only safe replacement, historical reattachment/current attributes, disabled/enabled automatic sync, historical current-pointer reuse, interrupted store adoption, no blind recreation on absent candidates, interrupted upload filename/size/hash recovery, mismatched remote bytes, foreign store/association refusal, Retry-After/backoff, terminal auth errors, deleted store/File recovery, missing associations, failed processing retry without reupload, explicit absent-operation reset that refuses existing copies, per-version removal/exclusion including explicit cleanup while sync is disconnected, store deletion retaining Files/local data, separately verified File purge, unregistration/disconnect retaining resources, ambiguity freezing existing selections, direct search/provenance, blocking retrieval until deselection cleanup settles, and not persisting query/excerpts. Scripted calls validate the provider-independent state machine, not a live OpenAI account's permissions, retention or ranking.

The ten cloud frontend tests cover reviewed consent before apply with Start separate; unlocking controls after a failed action; late responses after unmount; immediate clearing of the uncontrolled password field without browser-storage writes; cancellation/typed confirmation for remote deletion; actionable configuration failures inside the consent dialog; explicitly confirmed association cleanup while sync remains paused/disconnected. All previous rendering/navigation/tracking/history/diff regressions pass. These are focused mocked component tests, not a broad native GUI harness or a claim of visual acceptance on every display scale.

Production credentials use Win32 Credential Manager, and the Windows build validates the native bindings. Reading/writing a real user credential, actual account Test/upload/index/search/delete behavior, billing measurements and an upgrade over the user's real v0.3.0 installation remain **manual**. No developer key, environment key fallback, real user document upload, release tag, public GitHub Release, auto-update or PR merge is authorized/performed. The built installer was not run against the existing installation.

The [14-test installed Windows guide](manual-testing-milestone-4.md) provides exact two-file Markdown contents, preparation, clicks, endpoints, expected UI/remote states, failure/retry instructions, offline/pause/resume, two-project isolation, safe revoked-key testing, separate association/store/File cleanup and an acceptance checklist. The implementation report also includes essential numbered testing steps directly.

Limits: raw upload 16 MiB, text/expanded DOCX 4 MiB, conservative package/type screening, fixed provider origin, one cloud request at a time, 30-second individual HTTP timeout, bounded 100×100 listings and a 90-second between-request reconciliation budget. Large/inaccessible accounts fail closed rather than assuming complete remote absence. No provider-wide exactly-once or global external File-reference enumeration exists: ambiguous/no-match recovery and File purge require separate explicit user assertions after dashboard checks. Automatic detach/store deletion retains uploaded Files until explicit purge. Cleanup/dashboard/search/billing can be eventually consistent. No dollar spending cap or guaranteed free indexing is claimed.

The existing Windows Actions workflow remains enabled for PR/main/codex branch pushes and uploads **ContextDock-Windows-x64-setup** with 30-day retention. Consult the intended commit's PR run for its live status; local pass results do not imply remote CI completion. Local installer: `src-tauri/target/release/bundle/nsis/ContextDock_0.4.0_x64-setup.exe`. Download/upgrade instructions preserve the existing application identity and app-data directory. Prior milestone evidence follows as a historical record.

---

# Historical verification — ContextDock 0.3.0

Milestone 3 local checks completed on Windows 11 on October 19, 2026, with Node.js 24.12.0, Rust 1.99.1 and the x86_64 MSVC toolchain on an ARM64 host. Tests use disposable temporary databases, source roots and snapshot stores. No registered user project, real database, archive or snapshot was modified by testing. The prior Milestone 2 record is retained below.

| Check | Milestone 3 result |
| --- | --- |
| TypeScript / ESLint | Passed |
| Frontend tests | Passed: 16 tests in 4 files |
| Rust tests | Passed: 37 tests, including all 24 existing foundation/tracking/archival tests |
| rustfmt / Clippy all targets with warnings denied | Passed |
| Production TypeScript/Vite bundle | Passed |
| npm audit | Zero reported vulnerabilities |
| Windows x64 NSIS build | Passed: unsigned `ContextDock_0.3.0_x64-setup.exe`, approximately 3.47 MiB |
| Versions / identifier / database | npm + npm lock, Cargo + Cargo lock, Tauri all 0.3.0; `com.contextdock.desktop` retained; schema stays 2 |
| Diff manual samples | Actual Rust engine verifies A/B +4/−2, B/C +7/−2, A/C +10/−3 |
| git diff --check | Passed |

## Milestone 3 coverage

- Markdown identity, empty inputs, insertions, deletions, separate edit regions, exact old/new ranges and line numbers, Unicode, BOM, CRLF/LF equivalence, meaningful whitespace and final-newline changes, invalid UTF-8/NUL/UTF-16 rejection.
- A 10,000-line document with one real change; line-count/line-length limits reject unsupported inputs rather than truncating.
- Actual temporary-registry captures compare nonconsecutive versions after the source disappears, normalize reversed order, allow self-comparison and distinguish byte changes from normalized equivalence. SQL total-change counts and version counts stay unchanged.
- Different-family/missing UUID rejection; missing snapshots, hash mismatch and traversal fail closed. Real Windows exclusive sharing locks, snapshot-directory junction escape and oversized snapshot rejection are exercised.
- Locally generated ZIP/DOCX fixtures cover paragraph/run/Unicode/entity/break/table extraction, formatting-only differences, displayed hyperlink labels, accepted inserted/deleted text policy, skipped instructions/drawings, corrupt archives, absent body, malformed XML, trailing data, custom entities/DTDs, excessive compression, XML size/depth and forged central-directory counts. These are deterministic Open XML fixtures, not a claim of passing Microsoft Word application behavior.
- Frontend service mocks cover default/independent nonconsecutive selectors, both modes, previous/next change, summary/byte distinction, error/retry, one-dialog history transitions and focus restoration, discarded responses after switching family, 100-line rendering with all 350 changed lines reachable, and history-load retry. Existing repeated-tracking-panel/navigation tests remain green.

## Architecture and validation boundaries

Comparisons look up existing immutable UUID records briefly under the shared registry mutex, then release it before guarded I/O, hashing, extraction and Myers computation on a blocking worker. No migration, writes, capture, journal replay or comparison cache is introduced. The application identifier and per-user NSIS/data configuration are unchanged. Production IPC exposes version IDs only.

Dependencies introduced: `similar` 3.2.0, `quick-xml` 0.42.0 and `zip` 9.0.5 (Deflate; defaults disabled), with a committed Cargo lockfile. Limits and the complete DOCX inclusion/exclusion policy are in [architecture](architecture.md#local-comparison-milestone-3) and displayed in comparison warnings. Byte identity is separate from text equivalence; no AI interpretation exists.

The installer builds locally. It has not been installed by the agent over the user's real installation/database. Interactive upgrade/retention, Word save/formatting/locking behavior, SmartScreen, clipboard/Explorer and visual accessibility across themes remain user manual checks. Focused React tests do not constitute a full native GUI harness. No public tag/release or automatic updater was created, and the PR is not merged automatically.

The [mandatory 12-test guide](manual-testing-milestone-3.md) provides exact A/B/C contents, safe capture order, UI clicks, expected results, failure/retry instructions, equivalent-content testing compatible with tracking deduplication, independent families, restart, optional Word, source safety, acceptance and cleanup. The final response also includes essential installation/testing steps.

The retained [Windows workflow](https://github.com/Flint221/context-dock/actions/workflows/windows.yml) validates the PR and publishes **ContextDock-Windows-x64-setup** after a successful run. Check the run for the intended commit; local success does not imply remote completion. Remote status/artifact availability are reported in the PR/final response when available.

---

# Prior verification — ContextDock 0.2.0

Local checks completed on Windows 11 on October 19, 2026, using Node.js 24.12.0, Rust 1.99.1 and the x86_64 MSVC toolchain. The host is ARM64, running x64 executables through Windows emulation. Milestone 1 is the merged foundation; the following record covers Milestone 2.

| Check | Result |
| --- | --- |
| `npm run typecheck` | Passed |
| `npm run lint` | Passed |
| `npm test` | Passed: 11 tests in 3 files |
| `npm run build` | Passed: TypeScript and production Vite bundle |
| `npm audit` | Zero reported vulnerabilities |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | Passed |
| `cargo test --locked --manifest-path src-tauri/Cargo.toml` | Passed: 24 Rust tests |
| `cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | Passed |
| `npm run tauri -- build --bundles nsis --ci` | Passed: x64 production executable and unsigned 0.2.0 NSIS setup, approximately 3.23 MiB |
| Version/identifier consistency | npm, Rust and Tauri all 0.2.0; existing `com.contextdock.desktop` identifier retained |
| Manual fixture script | Parsed successfully by PowerShell; not executed against Documents or registered projects |
| `git diff --check` | Passed |

## Focused native safety coverage

All fixtures use temporary roots and databases, never registered user projects.

- Existing foundation tests: SQLite initialization/reconnection, persistent project operations, safe unregistration, duplicate canonical paths, recursive mixed document discovery, distinct same-name subpaths, ignored directories, missing folders, actual Windows ACL denial and junction safety.
- Migration from an actual schema-1 database: original project/document/source UUIDs and theme survive, tracking/archival default off, no versions invented, no original file moved, and schema reopening is idempotent. Newer schemas are rejected without downgrade.
- Independent initial histories, recognizable trailing date/version normalization, ambiguity across families/directories, explicit association memory, initial ignore selection and Word lock-file exclusions.
- Byte-exact immutable snapshots and archives for binary `.docx` fixtures; identical filenames in nested directories retain independent families. This verifies byte storage, not Microsoft Word application behavior or document parsing.
- Repeated scans and historical-content reuse do not create duplicate versions; filename relocations preserve family identity; live duplicate copies require review.
- Same-path overwrite preserves old snapshot/archive bytes and leaves the current source in place. Confirmed different-name replacement archives only the old source.
- Unexpected old-source hashes, destination collisions, archive junctions, sharing locks and lost ownership evidence refuse unsafe movement/overwrite.
- Crash recovery before/after snapshot finalization; archive rename before database commit; partial archive-copy staging; archive hard-link finalization before database commit. Recovery is idempotent. Incomplete private staging is retained, and copying retries with a fresh name rather than truncation.
- Persisted file identity prevents adopting an unrelated destination merely because its hash matches. Archive edits do not mutate independent snapshots; verified-location access reports integrity conflicts.
- A real `notify` watcher captures changed content, deduplicates repeated content, stops processing when disabled, catches up on resume, and captures changes after a simulated application restart.
- Unregistration retains snapshot bytes and history; registering the same root restores its stable identity with tracking off. Pending archives resume only after explicit archival permission.

Frontend interaction tests cover existing registration/cancellation/error behavior, theme/version, rename/refresh/confirmed removal, selected initial sources with archival off by default, separate archival consent, paused history access, explicit ambiguous-family selection and historical snapshot location/metadata actions. Native services are mocked in these frontend tests; they do not constitute complete GUI automation.

## PR #2 manual-test fixes

User testing exposed repeated tracking panels, stale Dashboard/Projects content, disappearing project tools, and controls apparently stuck on Working. The tracking panel and documents list had identical sibling React keys. A regression test reproduced three panels after a registry update before the fix; distinct, stable keys now preserve each component correctly.

The regression covers eight registry revisions, scan completion, and navigation through Dashboard, Projects, Settings and back to project details. It asserts exactly one tracking panel in project details, none elsewhere, and an available Refresh documents toolbar after returning. A separate test verifies resume unlocks review decisions and scan success/failure unlocks controls. All 11 frontend tests, TypeScript and ESLint pass. These service-mocked tests establish the rendering/control fix; they do not measure native scan latency.

Read-only inspection of `ContextDock-Test` found four current Markdown files totaling 361 bytes, plus retained archives. A project this small should normally scan in seconds; minutes of permanently disabled controls are unexpected. Locked files, cloud hydration and large folder trees can increase native scan time. The stale duplicate panels could continue showing Working after their live component completed. If a fresh installer still shows a prolonged scan, investigate that native operation separately.

The user confirmed archival, independent Technical Documentation tracking, retained history, same-name content replacements and startup capture. Captured version numbers advance with content changes independently of filename suffixes, so a file named `_v2.md` can correctly have captured versions 3 and 4. No source, archive, snapshot or database contents were changed during this investigation.

## Installer and manual verification boundary

The NSIS executable builds locally using the existing per-user configuration and application identifier. No release tag or public GitHub Release was created. The user tested the earlier 0.2.0 installer and reported the issues above. The rebuilt installer containing the rendering fix has not been executed against the user's existing installation/database by the agent. Interactive upgrade, Explorer/clipboard, actual Word saves/locks, SmartScreen, clean-machine prerequisites, uninstall and large-project usability checks remain manual checks. No claim of passing those interactions is made.

The required [complete manual guide](manual-testing-milestone-2.md) supplies exact disposable Markdown files, sample contents, UI actions, expected internal version counts, expected archive results, failure indicators and safe cleanup. Do not enable archival for important projects as a substitute for testing the disposable walkthrough.

## CI and artifacts

[PR #2](https://github.com/Flint221/context-dock/pull/2) contains the implementation. The [Windows installer workflow](https://github.com/Flint221/context-dock/actions/workflows/windows.yml) independently runs the checks and uploads **ContextDock-Windows-x64-setup**. Consult the intended commit's run for current status; local results above do not imply a remote run has completed. Old branch runs can be cancelled when a newer push supersedes them.

Installer output locally: `src-tauri/target/release/bundle/nsis/ContextDock_0.2.0_x64-setup.exe`. Artifact download/install steps are in the README and manual guide. Tag-release publishing is maintained but was not exercised or authorized as a public release in this milestone.

## Remaining limits

- Unsigned distribution; no automatic updater, rollback, detailed diff, AI or cloud integration.
- Reparse-point roots/children are refused. Fully local folders on supported Windows filesystems are the safest initial test target. Unsupported hard links/handle renames or cross-volume conditions remain visible failures with snapshots/sources preserved.
- Save stability uses quiet metadata and Windows sharing protection, not an editor-provided universal save-complete signal. Persistent locks require closing the writer and manual reconciliation after bounded retries.
- Scans and metadata results are currently serialized and held in memory; UI input remains off the native worker, but large scans can delay subsequent native queries. Streaming DB pagination/cancellation is future hardening.
- Snapshots, retained unregistered history, and orphan temporary bytes are not automatically purged; storage can grow over time. Incomplete archive staging is retained safely rather than deleted.
