# 0.10.5 - exFAT archive copies and named attention items

Local release checks passed: **236 native tests**, **154 frontend tests across 17 files**, TypeScript, ESLint, rustfmt, all-target Clippy with warnings denied, whitespace checks, production frontend build and Windows x64 NSIS packaging. Four explicit Windows checks passed separately, including archive publication/recovery on the actual healthy D: exFAT volume in a disposable directory, for **240 distinct native tests/checks**. The normal run leaves five entries ignored, including that explicit drive test and the existing child fixture. Versions agree at 0.10.5; schema remains 013.

Source commit `1461725` is pushed to main. Read-only installed 0.10.4 evidence confirms Weather Tool v1 removed and v2 indexed/completed with no cloud error. CottonTrax has eleven desired indexed jobs, four obsolete removed jobs and no cloud error. Its remaining two issues are blocked local archive copies for Master Context v3 and Team Tasks v2. Windows archive copy publication previously depended on hard links, unsupported by exFAT. It now uses verified no-overwrite handle rename after durable staging-identity journaling. Tests cover interrupted publication/recovery, journal refusal, preserved snapshots and collisions. The project overview and pending-operation list now identify destinations and distinguish failed archive/file operations from missing files and reviews. See [investigation and acceptance](ContextDock_ExFAT_Archive_v1.md).

Unsigned installer: `src-tauri/target/release/bundle/nsis/ContextDock_0.10.5_x64-setup.exe`, **4,732,577 bytes**, SHA-256 `FFF916413431A0F1D72FCEA2483891CEB94668F3CC3F0B8B4DCADFA1C4E31769`. File/product version is 0.10.5, built from pushed main. It was not installed against user data. The actual database/WAL evidence copy passed quick_check; original databases, sources, snapshots, archives and backups were unchanged. No live cloud API requests were made. The user should install and retry pending operations for the real project; that acceptance is not claimed. No completed GitHub workflow result is claimed.

---

# 0.10.4 - Pending cloud association removal recovery

Local checks passed: **235 native tests**, **152 frontend tests across 17 files**, TypeScript, ESLint, rustfmt, all-target Clippy with warnings denied, whitespace checks, production frontend compilation and Windows x64 NSIS packaging. Three explicit Windows integration checks passed separately, for **238 distinct native tests/checks**. Four entries remain ignored in the standard run, including the child fixture. Package, lockfile and configuration versions agree at 0.10.4; schema remains 013.

Source commit `caf4b51` is pushed to main. Read-only evidence from the running app showed CC Weather Tool v2 indexed successfully and obsolete v1 removal stopped indefinitely after a still-present verification response. The exact historical stop can resume automatically under existing policies. Acknowledged but unverified removal now uses bounded retries, rechecking ownership and absence before clearing the association. Desired-only counters exclude obsolete entries, and project errors block ready messaging/chat. Invalid receipts and foreign identities remain protected. Mocked restart/exhaustion tests preserve both uploads and local history. See [investigation and installed acceptance](ContextDock_Cloud_Removal_v1.md).

Unsigned installer: `src-tauri/target/release/bundle/nsis/ContextDock_0.10.4_x64-setup.exe`, **4,734,922 bytes**, SHA-256 `E51F15DDF127564E21C858F132E6D00DB0455AD23957EF93C204B73733C74B23`. File/product version is 0.10.4, built from pushed main. The installer was not executed against user data. A stable database/WAL evidence copy passed quick_check; originals, source files, snapshots, archives, backups, credentials and cloud resources were not modified. Live remote recovery and installed acceptance remain unverified. No completed GitHub workflow result is claimed.

---

# 0.10.3 - External-drive reconnection and folder refresh

Local release checks passed: **233 native tests**, **151 frontend tests across 17 files**, TypeScript, ESLint, rustfmt, all-target Clippy with warnings denied, whitespace checks, production frontend compilation and Windows x64 NSIS packaging. Three explicit Windows tray/startup/notification integration tests passed separately, for **236 distinct native tests/checks**; the normal native run leaves those three and the existing child fixture ignored. Version/lock/config values agree at 0.10.3. Schema remains 013 and application identity is unchanged.

Source commit `534a065` is pushed to main. Previously, lost roots were skipped by periodic health checks, and ready guards rejected their persisted state without testing whether the original directory had returned. Workspace loading, native health checks and manual refresh now recover the original readable folder only at the saved path with its saved filesystem identity. Recovery increments the folder revision and restores the saved watcher mode. The native monitor rebinds and reconciles under existing tracking/cloud policies. **Refresh folders** on Dashboard/Projects checks all registrations; **Check saved folder** in the project recovery panel retries without a picker and reports continued absence/conflict. Different or unverified directories and changed drive letters still require reassignment. See [reconnect behavior and acceptance](ContextDock_Drive_Reconnect_v1.md).

Native regressions cover restart recovery, live watcher recovery and capture without reassignment, multi-project refresh with tracking off/on, preserved source IDs/snapshots/selections/cloud store identity/pause, retained individual missing-file reporting and rejection of replacement directories or unknown saved identities. Earlier tests enforcing reassignment after the exact original directory returned were updated to the new requested behavior; safety tests for different roots remain in place. UI tests cover the global refresh action, per-project retry without a picker, continued failure feedback and existing reassignment flows. The React review covered control labels, disabled/pending states, retained navigation/draft handling and error visibility.

Unsigned installer: `src-tauri/target/release/bundle/nsis/ContextDock_0.10.3_x64-setup.exe`, **4,731,925 bytes**, SHA-256 `907EC3A1C1E4D1B075EA19466FC72BFA0FAC91AE9D569D0E163F9A5A78AEAE33`. File/product version is 0.10.3. It was built from pushed main and was not installed against user data. Actual project sources, application databases, snapshots, archives, backups, credentials and cloud resources were not modified by this work.

Disconnect tests move a disposable directory away and back while preserving its filesystem identity. Actual cable removal/remount behavior on each filesystem and installed acceptance remain manual checks. No completed GitHub workflow result is claimed.

---

# 0.10.2 - Confirmed missing-file and local-history cleanup

Local release checks passed: **230 native tests**, **148 frontend tests across 17 files**, TypeScript, ESLint, rustfmt, all-target Clippy with warnings denied, whitespace checks, production frontend compilation and Windows x64 NSIS packaging. The three explicit Windows tray/startup/notification tests passed separately, for **233 distinct native tests/checks**. The normal native run leaves four ignored entries, including the intentionally ignored child fixture. The final empty-group cleanup change was checked by rerunning all six cleanup regressions; the folder-tree UI tests were also rerun after their final adjustment. The frontend suite passed with two workers after an unrelated comparison test exceeded its five-second timeout during concurrent native compilation. Version/lock/config values agree at 0.10.2; schema 013 adds the durable snapshot cleanup journal.

Source commit `6e97c35` is pushed to main. **Forget…** is available beside confirmed missing files after saving or discarding checkbox drafts. A preview token and explicit confirmation bind the exact path and affected captured revisions. Unapproved deleted test files with no snapshots can be removed from discovery and approvals; captured revisions for that path can be permanently removed while retaining other filename versions. Empty grouping placeholders are cleaned when the last source/history is removed. Uploaded or uncertain cloud identities, approved baselines, retained grouped histories, archive records and unfinished operations are protected with blockers. See [behavior and acceptance checks](ContextDock_Forget_Missing_v1.md).

Regression fixtures verify deletion of a never-approved file, restart and rediscovery with approval still required; physical deletion of read-only snapshots for multiple edits; preservation of other sources and archive files; partial-family removal and final empty-family cleanup; stale review, returned source, unavailable root and cloud-reference refusal; foreign-key integrity; locked-file cleanup recovery after restart; and refusal of conflicting bytes or paths outside snapshot storage. UI coverage includes confirmation/cancel, no-history messaging, immediate row/dialog removal after success, protection of unsaved drafts, blockers and retained confirmation on stale failures. Physical snapshot deletion is journaled with history removal and retried after failure; it uses the verified Windows handle without changing source files or archive/backup copies.

Unsigned installer: `src-tauri/target/release/bundle/nsis/ContextDock_0.10.2_x64-setup.exe`, **4,727,259 bytes**, SHA-256 `3BB5CFB4CD621376E6C4A4EBE856F35AB0B0F655FA28D35562BBA70DF86CD827`. File/product version is 0.10.2. It was built from pushed main and was not installed against user data. Actual PV-GIN and other project sources, application databases, snapshots, archives, backups, credentials and cloud resources were not modified during implementation or verification. Installed acceptance and a completed GitHub workflow are not claimed.

---

# 0.10.1 - First-send Windows notification repair and test control

Local release checks passed: **224 native tests**, **144 frontend tests across 17 files**, TypeScript, ESLint, rustfmt, all-target Clippy with warnings denied, whitespace checks, production frontend compilation and Windows x64 NSIS packaging. Three explicit Windows integration tests then passed separately, for **227 distinct native tests/checks**. The normal run leaves four ignored entries: the existing child fixture and the three explicit Windows checks. Package/lockfile, Cargo/lockfile and Tauri versions agree at 0.10.1. Schema 012 and application identity are unchanged.

Source commit `c347fe0` is pushed to main. A disposable notification identity reproduced `0x80070490` specifically from `ToastNotifier.Setting` before first submission. Registering identity metadata alone still left that first permission query failing. The repaired path allows only that specific not-yet-initialized exception through to `Show`, rechecks Windows permissions and verifies actual retention through Notification Center history. The isolated native test suppresses its popup and removes its own history and registry entries. It does not verify a visible installed banner or protocol click. Per-user installer/runtime registration supplies display name, logo and protocol-only stub identity; Windows notification permissions are not changed. A Settings test button submits an explicit harmless alert even while focused, with actionable disabled/error feedback. Failed and disabled alerts back off per project/category without marking delivery or blocking other projects; recovered/resolved notification errors clear.

The two existing Windows fixture checks again passed for disposable startup registration/override and hidden native tray monitoring/reopen/Exit. Frontend tests cover test-notification success/failure without modifying preferences or approvals. See [the full installed test walkthrough](ContextDock_Tray_Testing_v1.md).

Unsigned installer: `src-tauri/target/release/bundle/nsis/ContextDock_0.10.1_x64-setup.exe`, **4,709,249 bytes**, SHA-256 `EC8C7DC8343594637FEEF5E8778314B2F971AE4C131C2B28EA8DA96507854824`. File/product version is 0.10.1. Generated NSIS includes `ContextDock.ico` at the installation root, notification registration at postinstall and owned identity cleanup on explicit uninstall. The installer was built from pushed main and was not executed against user data.

Actual project sources, application databases, snapshots, archives, backups, credentials and cloud resources were not changed. Temporary notification/startup test entries were cleaned up. Installed upgrade, real sign-out/sign-in, visible banners and clicking installed notifications remain user acceptance checks. No completed GitHub workflow result is claimed.

---

# 0.10.0 - Tray operation, Windows startup and approval inbox

Local release checks passed: **222 native tests**, **143 frontend tests across 17 files**, TypeScript, ESLint, rustfmt, all-target Clippy with warnings denied, whitespace checks, production frontend compilation and Windows x64 NSIS packaging. The standard native run leaves three entries ignored: the existing intentionally ignored child fixture and two explicit Windows checks. Both Windows checks were then run separately and passed, bringing distinct native tests/checks passed to **224**. Package/lockfile, Cargo/lockfile and Tauri versions agree at 0.10.0. Schema is 012 and the application identity remains `com.contextdock.desktop`.

Feature commit `ceae5ef` is pushed to main. It adds native close-to-tray lifecycle, per-user opt-in hidden startup with Windows override readback, single-instance restoration, a confirmed per-project batch approval inbox, durable upload pause and grouped native Windows notifications. Review uses existing discovery and saved-selection rules. Explicit Exit waits off the event thread for current work; no new requests start after stopping. See [background behavior, controls and installed acceptance](ContextDock_Background_Review_v1.md).

The Windows fixture tests verified startup registration/removal and preservation of a disabled StartupApproved value under uniquely named disposable entries. The native tray test used a separate registry, sources, application identity and blank window to verify hidden startup, shortcut restoration, intercepted close, hide, continued local capture/new-file discovery while hidden, review navigation and guarded Exit. It did not access the installed project's database, real credentials or cloud account. Temporary startup entries were removed; one failed-test entry was explicitly verified and cleaned after correcting the registry write-handle bug. UI regressions cover reversible controls, OS-disabled startup display, batch selection/preview/cancel/confirmation, stale errors and tray Exit through unsaved selection protection. Migration and cloud tests preserve existing history/identities, block unapproved uploads, respect legacy manual sync, retain pause across restart and permit confirmed removals while paused.

Unsigned installer: `src-tauri/target/release/bundle/nsis/ContextDock_0.10.0_x64-setup.exe`, **4,697,170 bytes**, SHA-256 `672B3246CC6ABA0D0F5C6E5FC630AC18BCF7EF215424CE28F580F945E1EB7D5B`. File/product version is 0.10.0; application PE machine is 8664 (x64). The binary manifest contains Common Controls 6 and `asInvoker` privileges, without duplicate manifest resources. Generated NSIS includes the startup uninstall hook and quoted per-user `contextdock:` protocol registration/cleanup. The artifact was built from pushed main and was not installed against user data.

Original project sources, application databases, snapshots, archives, backups, credentials and remote resources were not modified by this implementation or validation. Actual installed upgrade, Windows sign-out/sign-in, Windows notification delivery/click activation and installer execution/uninstall remain manual acceptance checks. The fixture and packaging results do not claim those checks, authenticated provider acceptance or a completed GitHub workflow.

---

# 0.9.2 - Shared project overview and pending index verification

Local release checks passed: **211 native tests**, one existing intentionally ignored child fixture, and **138 frontend tests across 16 files**. TypeScript, ESLint, rustfmt, all-target Clippy with warnings denied, diff whitespace checks, production frontend compilation and Windows x64 NSIS packaging passed. Package/lockfile, Cargo/lockfile and Tauri versions agree at 0.9.2. Schema 011 and application identity are unchanged.

Feature commit `63365de` is pushed to main. The overview remains mounted and visible across all three project tabs, with the existing saved-state and navigation guards. Index attachment now distinguishes absent verification attributes from conflicting identities, retrieves incomplete responses and resumes bounded checks automatically against the same uploaded File. Reconciliation handles incomplete list responses; deletion/search remain blocked without complete ownership verification. Five new native regressions cover delayed 13 MiB illustrated Word verification, unrelated files progressing, restart, partial attributes, bounded verification, refusal of unsafe removal and conflicts in every immutable identity field. See [the investigation and acceptance record](ContextDock_DOCX_Indexing_v1.md).

Read-only inspection of Boyd's source confirms 13,778,302 raw bytes, 14,743,413 expanded bytes, 670,146 body XML bytes, 50 ZIP entries, a largest entry of 3,112,531 bytes and a maximum expansion ratio of 24.05:1. These satisfy the existing raw/package/XML/entry/ratio limits. The application text now correctly lists 16 MiB raw, 4 MiB extracted text, 8 MiB body XML and 128 MiB expanded DOCX packages. The original failing provider response was not retained; saved activity confirms that the same File succeeded after manual retry, while optional-attribute regression fixtures establish the code defect and recovery behavior. No live provider re-upload or mutation was performed to reproduce it.

Unsigned installer: `src-tauri/target/release/bundle/nsis/ContextDock_0.9.2_x64-setup.exe`, **4,501,910 bytes**, SHA-256 `0F5422C7DC4843ED640444B1DD73067F7D079D314F859F55CF3019FF026C6A33`. File/product version is 0.9.2; application PE machine is 8664 (x64). The artifact was built from pushed main. It was not installed or launched against user data. Original application databases, project sources, snapshots, backups and cloud resources were not modified by this investigation or validation; registry evidence was read from separate copies. Local checks do not claim a completed GitHub workflow or installed authenticated acceptance.

---

# 0.9.1 - Filename versions and same-version cloud updates

Local release checks passed: **206 native tests**, one existing intentionally ignored child fixture, and **138 frontend tests across 16 files**. TypeScript, ESLint, all-target Clippy with warnings denied, rustfmt, diff whitespace checks, production frontend compilation and Windows x64 NSIS packaging passed. Version/lockfile/config values agree at 0.9.1. Schema is now 011; application identity is unchanged.

Feature commit `ce92599` is pushed to main. The implementation distinguishes authored filename versions from immutable save revisions, publishes the latest revision per filename version and keeps edited content current in both cloud policies. Regression coverage includes repeated Markdown/Word saves, timestamp-only changes, restart, identical-content filename bumps, content undo without changing authored version, lower-suffix review, valid DOCX media-only updates in both cloud modes, schema-10 metadata/cloud/snapshot preservation, migration rollback on invalid references and UI labels for high revision counters. See [filename-version behavior and acceptance checks](ContextDock_Filename_Versions_v1.md).

Unsigned installer: `src-tauri/target/release/bundle/nsis/ContextDock_0.9.1_x64-setup.exe`, **4,499,350 bytes**, SHA-256 `572E25871A98E523A53E15F173B235CE11E4AF76C640E0A43CC4BCF3F1D80926`. File/product version is 0.9.1; the application binary PE machine is 8664 (x64). The artifact was built from pushed main and was not installed or launched against user data. No actual project files, application databases, backups, credentials or remote resources were changed by this implementation/testing. Existing projects are migrated, not reset; older schema-10 readers reject schema 011.

The GitHub [Windows workflow for the feature commit](https://github.com/Flint221/context-dock/actions/runs/39857833951) was in progress when checked. The local results above are complete and do not claim a completed remote workflow or live provider acceptance.

---

# 0.9.0 - Project tabs, status overview and readable archives

Local validation passed: **200 native tests**, one existing intentionally ignored child fixture, and **136 frontend tests across 16 files**. TypeScript, ESLint, rustfmt, all-target Clippy with warnings denied, production frontend compilation and Windows x64 NSIS packaging passed. Package/lockfile, Cargo/lockfile and Tauri versions agree at 0.9.0. Schema 010 and application identity remain unchanged.

The feature commit `4acd2ba` was pushed to `codex/new-file-selection`, merged into main as `b3f56c5`, and pushed. The merged application source/config tree exactly matched the tested feature commit before packaging. Browser fixture checks and visual inspection covered Files/Cloud/AI separation, sticky header after scrolling, saved counts, draft protection and 1180x780 / 780x560 layouts. See [project workspace details](ContextDock_Project_Tabs_v1.md). Tests used isolated fixtures; no installed application acceptance or live authenticated cloud/API result is claimed.

Unsigned installer: `src-tauri/target/release/bundle/nsis/ContextDock_0.9.0_x64-setup.exe`, **4,506,424 bytes**, SHA-256 `A3500B67C5632B7E8FC935B6AD3E51745303019E31B0CD2733572545976D8E14`. File and product version are 0.9.0; packaged application PE machine is 8664 (x64). The installer was built from merged main and was not executed against user data. The temporary browser and preview server were stopped.

The GitHub [Windows build for the merge](https://github.com/Flint221/context-dock/actions/runs/39853872822) was running when checked; frontend checks and rustfmt had passed, native compilation was still running. Local checks above are complete; this record does not claim a completed GitHub workflow result.

---

# 0.8.1 - Unified selection installer and authorized local reset

October 22, 2026. Fresh checks pass: **197 native tests** plus the intentionally ignored child fixture, **131 frontend tests across 15 files**, TypeScript, ESLint, rustfmt, all-target Clippy with warnings denied, production frontend compilation and Windows x64 NSIS packaging. Version/lockfile/config values agree at 0.8.1; supported schema is 010 and application identity is unchanged.

The user's explicitly authorized fresh local reset is complete: 29 schema-10 tables, integrity ok, no foreign-key errors, zero projects/documents/cloud jobs and cloud disconnected. The original DB/WAL/SHM were preserved twice in a newly verified backup. All 57 snapshots remain in place and backed up; known source/archive hashes and the earlier backup are unchanged. No app GUI, installation or real re-upload was performed. See [the reset and preservation record](ContextDock_Local_Reset_v1.md) and [the preceding investigation](ContextDock_DB_Investigation_v1.md). Resetting is an account-specific backend action, not automatic installer behavior.

Unsigned installer: `src-tauri/target/release/bundle/nsis/ContextDock_0.8.1_x64-setup.exe`, **4,495,604 bytes**, SHA-256 `AAEE6F98804B63871E5AFFFF497E33AE2798A833CCC36C5199022802E8F4F4F7`. File/product version is 0.8.1. No installed acceptance, main merge or GitHub CI result is claimed.

---

# 0.8.0 — Reviewed reversible archival

October 22, 2026. **187 native tests pass**, plus one intentionally ignored child-fixture entry invoked by its parent test. **127 frontend tests across 15 files pass**. TypeScript, ESLint, rustfmt, all-target Clippy with warnings denied, production frontend compilation and Windows x64 NSIS packaging pass. Package/lockfile, Rust and Tauri versions agree at 0.8.0. Schema 009 records durable archive intents and rejects older readers; application identity remains com.contextdock.desktop.

Eleven new native tests cover repeated source archive/restore cycles without snapshot/history changes, original-path conflicts and restart retry, stale reviews, changed archive identities, failed metadata commits after forward/reverse renames, retained in-place snapshot copies, folder exclusions, initial import archival consent, foreign archive destinations, nested original paths, ignored setting writes, restoration while tracking remains paused and 24 Windows Unicode/path-length rename alignments. Three added frontend regressions cover review failures/reload/cancel, stale token recovery and paused-tracking opt-out, alongside the existing confirmation/readback/duplicate-action tests.

Disposable nested-path tests exposed an existing FILE_RENAME_INFO buffer issue: some destination lengths acquired trailing garbage characters. Explicit UTF-16 termination plus exact destination identity verification fixes the observed case; all alignment cases pass. No installed user project sources, snapshots or cloud resources were modified by validation. Installed upgrade, Explorer organization, real file locks and authenticated cloud acceptance remain user checks in [the reversible archival walkthrough](manual-testing-reversible-archival.md). Existing checked archival does not trigger silent backfill after upgrade; use Organize older files now.

Unsigned installer: `src-tauri/target/release/bundle/nsis/ContextDock_0.8.0_x64-setup.exe`, **4,486,107 bytes**, SHA-256 `37BBE14B5682EDB2FA0E42690C2AA07771B98366CD587F6075313AC9DECB2521`. File/product version is 0.8.0. The installer was built, not executed against user data. Local checks do not claim a GitHub CI result or main merge.

---

# 0.7.3 — Current filename version versus capture count

October 22, 2026. **176 native tests pass**, plus the intentionally ignored child-fixture entry invoked by its parent test. **124 frontend tests across 15 files pass**. TypeScript, ESLint, rustfmt, all-target Clippy with warnings denied and diff whitespace checks pass. Versions agree at 0.7.3; schema 008 and application identity are unchanged.

Native regression coverage imports distinct v2/v4/v6 filenames and checks tracking `currentLabel=v6`, `currentNumber=3`, `versionCount=3`, and unchanged numeric cloud readiness metadata. A refresh preserves snapshot IDs/numbers/hashes; editing the v6 source retains its label while increasing the capture sequence/count to four. Filename-label cases cover decimals, parentheses/case, trailing dates, leading zeroes and refusing unversioned/draft labels. A frontend regression verifies Current v6 and Versions 3 in the same row, the saved-capture tooltip and explicit Capture N fallback for unversioned filenames. Existing grouping, retained history, folder-selection and cloud tests remain green. Tests use disposable files/stores; no installed user data or cloud resources were changed. Native installed visual acceptance remains a user check; no re-import is required for the display update.

Production frontend compilation and x64 NSIS packaging pass. Unsigned installer: `src-tauri/target/release/bundle/nsis/ContextDock_0.7.3_x64-setup.exe`, **4,418,463 bytes**, SHA-256 `15434BBB14F600E1AABC5F86BFB5E64EE1D374350F95093C845CBF08ABA4D5B7`. File/product version is 0.7.3. The installer was built but not run against user data. Main-branch CI is separate from these local checks.

---

# 0.7.2 — Ordered initial filename versions

October 22, 2026. **174 native tests pass**, plus the intentionally ignored child-fixture entry invoked by its parent test. **123 frontend tests across 15 files pass**. TypeScript, ESLint, rustfmt, all-target Clippy with warnings denied, production frontend compilation, x64 NSIS packaging and diff whitespace checks pass. Package/lockfile, Rust and Tauri versions agree at 0.7.2. Schema 008 adds ordered-import support; application identity remains `com.contextdock.desktop`. Older applications must not reopen the upgraded registry.

Six added native tests cover numeric initial ordering (v1/v2/v10), current-source choice, source/snapshot retention with archival enabled, later replacement association, independent folders/types/unrelated stems, decimal ordering, duplicate-rank ambiguity, selected-file exclusions, identical-byte deduplication, injected capture failure/restart recovery, restart between consent and planning, legacy remove/re-add repair, immutable earlier history IDs/bytes, explicit cloud re-selection and refusal of retained-cloud/complex-history repairs. The existing ambiguous-family regression now uses conflicting numeric ranks; a unique initial v1/v2 sequence is intentionally no longer ambiguous. Incomplete groups are checked as cloud-ineligible. All tests use temporary project/registry/snapshot stores; the user's installed project and cloud resources were not changed.

Frontend coverage verifies the initialization explanation, existing selected-source/confirmation behavior, one active grouped family with access to earlier separate histories by their original document ID, and all existing tracking/history/folder-selection/cloud flows. Runtime import notices explain cases that remain separate. This release has automated component/native validation; it does not claim a native installed upgrade or authenticated cloud acceptance. Follow [the numbered-import walkthrough](manual-testing-numbered-import.md), particularly the retained-history and cloud-selection checks.

Unsigned installer: `src-tauri/target/release/bundle/nsis/ContextDock_0.7.2_x64-setup.exe`, **4,423,986 bytes**, SHA-256 `38EDE2280E65DC901E38CBE051032DF44851A77017ED085E4DA8C76CD78A4B74`. File/product version is 0.7.2. Built successfully but not run against user data. Implementation is pushed on `codex/folder-selection`; no main merge or GitHub CI result is claimed.

---

# 0.7.1 — Larger Word packages and independent sync failures

October 22, 2026. **168 native tests pass**, plus the intentionally ignored child-fixture entry invoked by its parent test. **122 frontend tests across 15 files pass**. TypeScript, ESLint, rustfmt, all-target Clippy with warnings denied, production frontend compilation, x64 NSIS packaging and diff whitespace checks pass. Versions agree at 0.7.1; schema 007 and application identity are unchanged. An overlapping native test invocation encountered a Windows executable lock; the final complete native test/Clippy/format sequence was rerun successfully.

Four new native regressions verify that an illustrated DOCX above the old 4 MiB expanded-package screen uploads original bytes and indexes, the 128 MiB package safety boundary remains enforced, a malformed nested DOCX records path/version/reason while a healthy file finishes, local bytes/history remain retained, and automatic sync respects per-file backoff without a busy retry loop. The mixed-failure test also checks visible summary counts, question blocking and an idle scheduler after terminal outcomes. A frontend regression verifies the Some files need attention label, red alert content with filename/reason, available Sync now action and blocked question input. Existing scoped deletion, opt-out, uncertainty/recovery and pause/stop tests still pass.

Provider evidence uses mocks and temporary source/snapshot/SQLite stores; no real credentials or cloud resources were used. Installed upgrade, real document compatibility and authenticated provider acceptance remain checks in [the Word-sync walkthrough](manual-testing-word-sync.md). The old package-byte screen is removed, while raw snapshot, soft-upload, body XML, extracted-text, ZIP entry/ratio and integrity checks remain. This does not claim arbitrary-size or provider-token acceptance.

Unsigned installer: `src-tauri/target/release/bundle/nsis/ContextDock_0.7.1_x64-setup.exe`, **4,418,719 bytes**, SHA-256 `B6AE67179CC388E7C666A70C36DCBA9E6AB5DC1DD605D800145C64541E3CE00F`. File/product version is 0.7.1. The installer was built but not run. Changes are pushed on `codex/folder-selection`; no main merge or GitHub CI result is claimed.

---

# 0.7.0 — Saved folder tree selection

October 22, 2026. **164 native tests pass**, plus one intentionally ignored child-fixture entry invoked by its parent integration test. **121 frontend tests across 15 files pass**. TypeScript, ESLint, rustfmt, all-target Clippy with warnings denied, production frontend compilation, Windows x64 NSIS packaging and diff whitespace checks pass. Package/lockfile, Rust and Tauri versions agree at 0.7.0; application identifier remains `com.contextdock.desktop`. Schema 007 upgrades the registry; downgrading afterward is unsupported.

Native coverage checks nested/future-file exclusions across registry restart, case/slash normalization and folder boundaries, empty directories, invalid/stale/silently ignored writes, legacy ignored-file migration, history/source/snapshot retention and safe replay after injected capture failure. Scripted-provider regressions cover paused and off-but-retained cloud removals, historical-version exclusions, questions blocked until removal verifies, explicit retry, retained File/store reuse and stale upload authorization. No real account/key/cloud resources were used.

Frontend coverage checks draft-only checkbox changes, Save/Discard, mixed parent selection, actual sidebar navigation guards, native close-event handling, failed/in-flight saves, duplicate commands, lost-reply readback, stale drafts, listener failure and folder-loss draft retention. The actual App/components and production CSS were also inspected in a browser with synthetic registry/cloud responses: nested folders and disabled excluded descendants render clearly, navigation opens the three-choice prompt, Save continues to Settings, and reopening the project shows the saved exclusion with Save disabled. Screenshots are in ignored `src-tauri/target/ui-preview/folder-tree-draft.png` and `folder-unsaved-prompt.png`. The temporary tab/server were closed.

Unsigned installer: `src-tauri/target/release/bundle/nsis/ContextDock_0.7.0_x64-setup.exe`, **4,392,931 bytes**, SHA-256 `F1FC9A0099EEBD9B3B0D42653DD1DABAE098FE9DDEC1341B23DCFCC39319DB65`. File/product version is 0.7.0. The installer was not run against user data. Real Windows window-close handling, installed upgrade/restart and authenticated cloud behavior remain acceptance checks in [the folder-selection walkthrough](manual-testing-folder-selection.md). Implementation is pushed on `codex/folder-selection`; this record does not claim a main merge or CI result.

---

# 0.6.5 — Cloud cleanup blocking project removal

October 20, 2026. A new native regression first failed on the old cleanup path with the same generic rejected-request message shown in the user's screenshot when the mocked provider refused File content downloads. The screenshot alone does not establish which live request failed; no real credentials/API calls were used. Cleanup now verifies File identity/metadata against the opaque scoped upload journal without a content download. It still verifies deletion acknowledgements, resource absence, accessible shared references and scope/ownership, and retains IDs on incomplete cleanup. Read failures are never treated as deletion success merely because a request failed.

**157 native tests** pass (plus one intentionally ignored child-fixture entry invoked by its parent test), including three new tests covering download denial with intact/missing/all-absent resources, store disappearance during reference checking, and refusal before deletion for File ID/name/size/purpose mismatches. Local source bytes, saved versions/chats and absence of the removed registration across registry restart are checked. The prior remote-content-tamper cleanup assertion now checks metadata mismatch: cleanup intentionally no longer claims a remote SHA-256 check. Explicit per-version purge and lost-upload attachment recovery retain their separate existing content checks.

**110 frontend tests across 14 files** pass, including two new cases for verified cleanup after a lost command reply and failed cleanup followed by a successful explicit retry. TypeScript, ESLint, rustfmt, all-target Clippy with warnings denied, production frontend compilation, x64 NSIS packaging and diff whitespace checks pass. React review retained event-driven mutations, operation locking, awaited readback and accessible error roles. Error roles have shared red styling, including nested paragraph text; persisted cloud-job, tracking-operation and scan errors are marked explicitly. Native HTTP failures include numeric status, and cleanup errors identify the failed stage without exposing raw provider bodies or credentials.

The actual App/components and production CSS were rendered with synthetic API/registry responses. A removal failure displayed red `rgb(255, 172, 172)` text in the dark-theme dialog. Retrying with verified synthetic cleanup closed the dialog, returned to Projects, changed the sidebar count from three to two and removed the selected project's card. Temporary screenshots are under the ignored `src-tauri/target/ui-preview/` directory. The tab/server were closed. This is mocked browser coverage plus native/provider-mock regressions, not an authenticated OpenAI deletion or installed upgrade acceptance.

Unsigned installer: `src-tauri/target/release/bundle/nsis/ContextDock_0.6.5_x64-setup.exe`, **4,374,700 bytes**, SHA-256 `7844A1AC260258C38CEB54C43C6550F5897AC8BD5C8C10CDEF8799F2FF2F0640`. The executable reports file/product version 0.6.5. Versions/lockfiles agree, application identifier and schema 006 are unchanged. The installer was not run against user data. Install 0.6.5 and retry the failed project removal; main-branch CI and live account/installed acceptance remain separate confirmation.

---

# 0.6.4 — Final Milestone 6 polish

October 20, 2026. **108 frontend tests across 14 files** and **154 native tests** pass (one intentionally ignored child-fixture entry point is invoked by its parent integration test). TypeScript, ESLint, rustfmt, all-target Clippy with warnings denied, production frontend compilation, Windows x64 NSIS packaging and `git diff --check` pass. Package/lockfile, Rust and Tauri versions are 0.6.4; identifier `com.contextdock.desktop` and schema 006 remain unchanged.

Removal regressions verify dialog closure after confirmed absence, handing the same verified snapshot to the workspace, stale background reads not restoring removed cards/documents, and navigation retaining the removal. The native retain/delete-cloud variants preserve exact source/snapshot/archive bytes and local version/chat rows across registry restart. Simplified process regressions cover default preview cards, local runtime/port filtering and deduplication without mutating inventory, URL schemes, preview dispatch, scoped reviewed bulk stop and cancellation, reversible Advanced mode, cleared hidden selections and the default after remount. Final validation corrected unsupported test query options, an incomplete project fixture, the version assertion and explicit Node imports in the icon generator.

Browser review rendered the actual App/components and production CSS with synthetic registry/cloud/process data and replaced native APIs. Reviewed light/dark dashboards, the shared cloud-folder logo, the 780px layout, Local previews cards and red stop controls, switching to/from Advanced diagnostics, and the tracking badge. The dark Watching badge computed to 13px text, green `rgb(140, 221, 176)` on `rgb(32, 59, 48)`. No console warning/error entries were returned during the process-page check. The temporary tab/server were closed. These checks do not establish installed WebView2 behavior, live cloud behavior or interactive upgrade acceptance.

Unsigned installer: `src-tauri/target/release/bundle/nsis/ContextDock_0.6.4_x64-setup.exe`, **4,384,655 bytes**, SHA-256 `1FBCE1F8E6FD26E769CD129EB976DAD8011D7476800EDD8D84F077F478E49D13`. The packaged executable reports file/product version 0.6.4. It was not installed or launched against user data. Native branding assets and the browser/sidebar logo derive from `public/contextdock.svg` via `npm run icons`.

Local success does not imply that GitHub Actions has finished. Installed acceptance and the main-branch CI result remain user confirmation after merge.

---

# 0.6.3 — Motion and interface polish

October 20, 2026, `milestone-6`. **154 native tests** pass (plus the intentionally ignored child-fixture entry point invoked by its integration test), and **99 frontend tests** pass across 13 files. TypeScript, ESLint, rustfmt, all-target Clippy with warnings denied, the production frontend and Windows x64 NSIS packaging pass. No native production behavior or database schema changed; versions/lockfiles are 0.6.3, identity remains `com.contextdock.desktop`, and schema remains 006.

Seven additional frontend regressions cover measured/indeterminate progress transitions, invalid/nonfinite/range handling, success only after confirmed completion and removal on idle, backend event-driven indexing counts with failures excluded, paused/off meter behavior, route-only scroll reset with preserved input/subscriptions during metadata updates, and modal field focus/restoration. Existing consent, cancellation, duplicate-click, streaming, missing-folder, cloud opt-out and process-action regressions remain green. No animation timers, stored motion preference or animation dependencies were added.

Browser verification rendered the actual App/components and built production CSS with synthetic data and replaced native APIs in a build-only fixture. Reviewed the light/dark dashboard, project controls, current-process controls and loading, Settings, a 780px iframe layout, and Add project dialog. Verified 40px primary targets and 160ms transitions through computed styles; page reveal/45ms section delay and navigation scroll Y=0; the live indexed meter changing from 2/5 (40%) to 3/5 (60%) after a synthetic backend event; modal entry at 260ms; and the name field receiving focus after opening with focus restored after Cancel. The final dashboard screenshot shows a hovered card using the new raised/highlighted state.

A separate fixture forced the shipped reduced-motion media rules on without changing OS settings. Computed styles confirmed no page animation, zero transition duration, and a static visible 5px loading track with no `aria-valuenow`. Normal and reduced modes preserve their underlying percentage transforms. The compact-layout review led to aligned summary numbers, left-aligned wrapping navigation and a compact single-line brand caption. Browser review does not establish installed WebView2 performance, the actual OS-to-WebView preference bridge, live cloud timing or interactive upgrade; the [motion walkthrough](manual-testing-motion.md) covers those checks.

Final unsigned installer: `src-tauri/target/release/bundle/nsis/ContextDock_0.6.3_x64-setup.exe`, **4,370,393 bytes**, SHA-256 `C2B506E611977B98149E4F716BFBE5CED8B4FC9CF25471620F043AE100D8BB01`. The packaged application reports file version 0.6.3. It was not installed or launched against user project data. The temporary browser tab/server were closed and port 17426 was verified free of listeners. No real cloud request, credential edit or startup change was made during UI review.

---

# 0.6.2 — Current processes verification

October 20, 2026, `milestone-6`. Local checks passed: **154 native tests** (plus one intentionally ignored child-fixture entry point, invoked by its parent integration test), **92 frontend tests**, TypeScript, ESLint, rustfmt, Clippy with warnings denied, and the production frontend build. This adds four native regressions and thirteen frontend regressions to 0.6.1. The full prior folder recovery, toggle/cloud removal, tracking, archival, comparison and chat suites remain green. Application/package versions are 0.6.2; identifier `com.contextdock.desktop` and database schema 006 are unchanged.

Native process tests bind real disposable IPv4/IPv6 TCP and UDP sockets and verify all four owner-PID tables, correct address/port conversion and self-stop protection. Synthetic ancestry tests distinguish protected app children from unrelated sibling apps. The termination test starts its own test executable as a temporary listener child, rejects an incorrect creation time, verifies the production app-descendant refusal, then bypasses only that ancestry guard for the owned fixture to exercise production ownership/session/critical/image checks, force termination and exit confirmation. The test retains and cleans up only its own child handle. It never terminates a pre-existing user app, changes startup configuration, opens a browser, reads a cloud key or accesses project data. The test inventory reads real local process metadata without printing or persisting that inventory.

Frontend service-mocked tests cover placement immediately above Settings, project-independent navigation, protected entries, filtered/paginated bulk targets, cancellation, individual confirmation, duplicate clicks, partial native failures, failed refresh/retry, failed command retry, PID-reuse invalidation, checkbox deselection, HTTPS reversal, local TCP-only preview, alternate nonlistener view, protocol/interface filters, startup inspection and explicit settings opening. Polling tests verify default-off behavior, five-second refresh, review pause, opt-out/re-enable and disposal on unmount. Production code additionally skips polling intervals while the page is hidden.

The [interactive process walkthrough](manual-testing-processes.md) covers browser/Settings launches, clipboard integration, installer upgrade, visual layout, Windows refusal, parent relaunch and reboot behavior. These GUI/OS integration checks have not been completed by the agent. Startup listings are registered Run/RunOnce names and Startup folder names; they do not establish enabled state or launcher attribution, and omit scheduled tasks/services. Process termination does not disable auto-start. No public release, new startup routine, service or automatic cleanup policy was created.

The final Windows x64 NSIS installer build passed after the process-table/dialog layout fixes. Installer: `src-tauri/target/release/bundle/nsis/ContextDock_0.6.2_x64-setup.exe`, **4,365,045 bytes**, SHA-256 `BD4139FD1D0A72A016565AC260E22513C907C13097094B7B64D79AC5178A9655`. The packaged application reports file/product version 0.6.2. It was not installed or launched against user data. A subsequent test-only adjustment preserves expected session-zero refusal on service-hosted test runners; the four focused native tests and all-target Clippy were rerun successfully. No production behavior changed after packaging.

---

# Verification — ContextDock 0.6.1 toggle reversal audit

October 20: **150 native tests** and **79 frontend tests** pass. TypeScript, ESLint, rustfmt and all-target Clippy with warnings denied pass. Version manifests and lockfiles are 0.6.1; schema 006, application identity and per-user storage are unchanged. The full [toggle audit](toggle-audit.md) records each control's stop, retention and restoration behavior.

The production frontend and Windows x64 NSIS build pass. Final unsigned installer: `src-tauri/target/release/bundle/nsis/ContextDock_0.6.1_x64-setup.exe`, **4,295,048 bytes**, SHA-256 `2928301D6FDB8E283E7950FD67C091263AF14D793ED49734B3B74C27D2E24B8F`. It includes the concurrent-cleanup and paused-switch fixes and was not installed or run against user data.

Eight additional native regressions verify paused full-family history removal/re-selection in the same store, opt-outs preceding unavailable/unrelated additions, deletion receipts and absence with restart/retry, saving pure opt-outs without sources/tested credentials with narrowly scoped deletion authorization, lost-upload recovery through deselect/reselect without duplicate Files, cleanup of all old project-owned stores while preserving another project's store, full-history/current round trips, and concurrent whole-project cleanup retaining its stop intent after an in-flight document-removal failure.

Six additional frontend cases verify opt-out/cancel/pending retry without resuming uploads, empty/ineligible saved selections requiring review, preserving a saved subset on enablement, archival on/cancel/off with tracking retained, and the paused main Cloud sync switch staying checked so unchecking consistently requests verified deletion. Existing cancellation, duplicate-click/readback, ownership/scope, history, cleanup, streaming, folder recovery and migration suites remain green.

All provider results, scope values and files in these tests are synthetic/disposable. No real API key, user database, registered folder or live cloud resource was changed. Installed GUI and authenticated account acceptance are the user-run scenarios in the audit; local tests do not establish actual provider/dashboard timing. Individual opt-outs retain uploaded Files for re-selection, while whole-project Off verifies owned stores and uploads are removed. Explicit pause retains cloud data. Local history remains preserved throughout.

---

# Verification — ContextDock 0.6.0 folder recovery (Milestone 6)

October 20: **142 native tests** and **73 frontend tests** pass. TypeScript, ESLint (no warnings), rustfmt, all-target Clippy with warnings denied, production assets and Windows x64 NSIS packaging pass. Versions/lockfiles are 0.6.0; the application identifier and per-user store remain unchanged. Schema 006 upgrades existing databases in place.

The unsigned installer is `src-tauri/target/release/bundle/nsis/ContextDock_0.6.0_x64-setup.exe`, **4,289,246 bytes**. SHA-256: `BF9CCB72420F3EC11797C26C5C5362B6EBE2E068D4289A8EF71D56CECE167350`. It was built locally and was not installed or run against the user's application data. Windows Actions now also runs on pushes to `milestone-6`; local results do not assert that a remote run has finished.

Eleven added native regressions cover the saved loss latch through restart/returning roots, replacement directory identity, metadata-only relocation with retained histories/chats/cloud IDs/preferences, partial/all-missing sources, stale review and ignored-write rollback, invalid/overlapping/retained/app-storage destinations, actual Windows junction refusal, root guards blocking new work, schema-5 migration, missing selected source refusal without provider calls, stale cloud request wrappers, and actual native watcher rebinding/capture plus tracking-off loss detection. Existing migration fixtures were adjusted to represent their genuine older schemas; the prior implicit reconnect expectation now requires explicit reassignment. All previous capture/archive/comparison/cloud/chat regressions pass.

Ten added frontend regressions cover a persistent reminder with blocked work panels and usable navigation, missing/inaccessible source warnings, reviewed confirmation and project identity, picker/review cancellation, duplicate click protection through readback, verification retry without reapplying, invalid/stale reviews, obsolete picker replies, all-missing/205-path pagination, saved-chat reading while missing selected files block questions, and obsolete registry reads not hiding newer folder-loss state. These are component/IPC-mock checks, not installed GUI acceptance at every scale/theme.

Tests use disposable sources, snapshots and databases, synthetic scope values and mocked provider responses. No real API key was read, no authenticated provider request was made, and no registered user folder/database was modified. Reassignment does not perform automatic filesystem search, source movement, data deletion or cloud recreation. An already-dispatched request may finish; explicit cleanup remains separately available. Installed upgrade, Explorer interaction, real account behavior, removable/network volumes and visual/Word acceptance remain the user-run steps in [Milestone 6 acceptance](manual-testing-milestone-6.md).

---

# Verification — ContextDock 0.5.4 cloud erasure and project removal

October 20: **131 native tests** and **63 frontend tests** pass. TypeScript, ESLint, rustfmt, all-target Clippy with warnings denied, production assets and Windows x64 NSIS packaging pass. The unsigned installer is `src-tauri/target/release/bundle/nsis/ContextDock_0.5.4_x64-setup.exe`, **4,268,411 bytes**. Application identity and schema 005 are unchanged. The agent did not install it or perform authenticated/destructive acceptance against user cloud data.

Eight new native regressions verify actual mocked bucket+File deletion, fresh upload after re-enable, false deletion acknowledgements retaining IDs, lost upload/delete responses and explicit retry, shared references, hash/ownership/key-scope/confirmation refusals, native retain/delete registration choice, failed local completion rollback and global key-disconnect rejection before vault access. Five new UI regressions cover the visible removal dialog's default deletion/explicit retention, duplicate click guard through cleanup/readback, failed cleanup blocking registration removal, retry after cleanup succeeds but local removal fails, and cleanup-needed status rather than false Off. Existing streaming/context/history suites pass.

Cleanup uses only synthetic resource inventories and disposable SQLite/source fixtures. No API key, user upload, real provider bucket or saved local chat was changed by agent tests. Local tests verify the intended DELETE calls, acknowledgement/absence handling and persistence; installed live-account deletion remains user acceptance. The updated manual test guide covers disposable cloud cleanup, cancellation, interruption/restart, re-enable and explicit retention. Earlier verification sections are historical behavior for their named versions.

---

# Verification — ContextDock 0.5.3 streamed chat

October 20: **123 Rust tests** and **58 frontend tests** pass. TypeScript, ESLint, rustfmt, all-target Clippy with warnings denied, production build and Windows x64 NSIS packaging pass. New native tests cover progressive decoded JSON text, escaped quotes/newlines/Unicode, CRLF SSE frames, completion usage, interrupted/failed/incomplete/malformed/oversized streams and split credential suppression. Chat provenance tests verify original source filenames. UI tests cover provisional text followed by the completed reply, ignored late callbacks, filename-only sources, Enter on the first chat, Shift+Enter and IME composition.

Installer: `src-tauri/target/release/bundle/nsis/ContextDock_0.5.3_x64-setup.exe`, **4,257,822 bytes**, unsigned and not installed by the agent. Tests use synthetic streams, mocked IPC/APIs and disposable local databases; no real API key, authenticated request or user upload was used. A fresh mocked browser preview was blocked by the browser client before it opened; 0.5.3 browser appearance/native account acceptance is not claimed. The corrected switch dimensions follow from isolating its 42×24 sizing from the later generic 18×18 checkbox rule. Use the updated manual chat guide for installed acceptance. Application identity and schema 005 are unchanged.

---

# Verification — ContextDock 0.5.2 major control safeguards

October 20: **120 Rust tests** and **56 frontend tests** pass. TypeScript, ESLint, rustfmt and all-target Clippy with warnings denied pass. Three new native regressions inject database triggers that silently ignore/reverse settings writes: cloud disconnect, cloud configuration and tracking now report verification errors instead of false success; configuration failure rolls back its transaction. Nine new frontend regressions cover double-click suppression through backend readback, unknown/read-failure states, mismatch errors, confirmation/cancellation for tracking/archival/pause/resume/global disconnect and idempotent shared operation locks. Existing cloud-chat/history suites remain passing.

Major settings show a spinner while pending and a success check only after saved-state confirmation. Acknowledged requests with unchanged state remain errors; backend read failures do not unlock project questions using a stale On value. Global key save/test calls now begin inside the same synchronous operation lock instead of dispatching a save before the lock. Native key disconnect verifies removal, but automated tests never read/delete an actual saved key. All backend failure injection uses disposable databases; frontend key/storage tests use synthetic values and mocked IPC. No authenticated API, installer execution, production app-state change, release or merge was performed by the agent.

The basic current-document Q&A/local-chat feature is implemented; the redesigned AI tool interface, historical-version chat selection, streaming, export and storage controls remain outside this pass. Queuing a sync operation is separately reported from completed remote indexing. See manual-testing-chat.md for current installed acceptance.

Production build and unsigned Windows x64 NSIS packaging pass. Updated installer: `src-tauri/target/release/bundle/nsis/ContextDock_0.5.2_x64-setup.exe`, **4,234,035 bytes**. Application identity and schema 005 are unchanged from 0.5.1. The installer was not run by the agent.

---

# Verification — ContextDock 0.5.1 simple cloud chat

October 20: **117 Rust tests** and **47 frontend tests** pass, including nine new native chat tests and six new interaction tests. TypeScript, ESLint, rustfmt, all-target Clippy with warnings denied and production build pass. Chat tests use scripted vector-search/Responses results and disposable SQLite/files; no actual key, user document upload or paid API call was used. The application identity/data location remains unchanged, and schema 4→5 migration preserves old snapshots, cloud mapping and approved baseline rows.

Native coverage verifies current-version cloud retrieval, source IDs, explicit one-model-call behavior, follow-up context, independent conversations, restart persistence, local cascade deletion without cloud/source changes, off/paused/unsynced/wrong-key/project isolation guards, disable between retrieval and inference, foreign-source rejection, empty retrieval without inference, uncertain timeout/restart without replay and credential-like question rejection. Frontend coverage verifies one enable confirmation followed by automatic Start, cancel/disable behavior, offline chat reading/deletion, multiple saved chats after remount, waiting for latest indexed versions and errors unlocking controls without paid retries. Existing tracking/diff/cloud/baseline suites still pass.

A disposable browser preview of the built assets with mocked IPC was used to inspect dark-theme spacing, collapsed project/advanced controls, question submission, source-backed answer display and Cloud sync off retaining readable chats while disabling the composer. Preview actions use synthetic data only; this is not a live native/API acceptance claim. See manual-testing-chat.md for the installed-app acceptance sequence.

The unsigned Windows x64 NSIS installer built successfully at `src-tauri/target/release/bundle/nsis/ContextDock_0.5.1_x64-setup.exe`, **4,231,778 bytes**. The agent did not install it, close the user's app, modify real app data or run authenticated acceptance requests. No merge, release tag or updater was created.

---

# Verification — ContextDock 0.5.0

## October 20 alignment citation follow-up

User acceptance reached OpenAI and returned numeric usage, but the alignment result failed literal quotation validation. The rejected provider text was intentionally not retained, so its exact mismatched quotation cannot be reconstructed. A local regression reproduces a baseline-formatting failure: decoded multi-line text with quotes/backslashes cannot be cited literally against the original JSON storage string. Prompt version 2 now supplies labeled decoded baseline fields and explicitly limits quotations to the named evidence text. ASCII whitespace-only differences are mapped back to exact source substrings before saving; altered words, punctuation, case, spliced passages and foreign references remain rejected. New diagnostics identify the summary/finding citation and evidence region without storing rejected output. No automatic retry or live API request was made, and approved baseline records remain unchanged.

Three added scripted regressions cover decoded baseline provenance, Unicode/CRLF/spacing restoration with a single request, and strict rejection of changed/noncontiguous quotations. The complete Rust suite now passes **108 tests**; **41 frontend tests**, TypeScript, ESLint, rustfmt and all-target Clippy with warnings denied pass. The production build and unsigned Windows NSIS rebuild pass; the updated local 0.5.0 installer is **4,207,707 bytes**. It was not installed or used for a live API request by the agent. The original implementation checks below record the initial Milestone 5 build.

Milestone 5 was implemented on Windows with the existing Node/MSVC Rust toolchain. All new request tests use scripted Responses output, disposable SQLite/source/snapshot fixtures and synthetic credentials. No real key was read, no authenticated OpenAI call was made, and no installer was run against user data. The application identifier remains com.contextdock.desktop; version manifests/locks are 0.5.0.

| Check | Result |
| --- | --- |
| Rust tests, binary targets, doctests | **105 passed**: 87 prior regressions + 18 focused AI tests; no failures |
| Frontend tests | **41 passed in 6 files**: 35 prior + 6 intelligence interaction tests |
| TypeScript, ESLint | Passed |
| rustfmt, all-target Clippy with -D warnings | Passed |
| Production frontend build | Passed; about 307 kB uncompressed JS |
| Windows x64 NSIS | Built ContextDock_0.5.0_x64-setup.exe, 4,208,701 bytes (about 4.01 MiB), unsigned |
| npm audit --omit=dev | Zero reported production vulnerabilities |
| Transactional migration | Schema 3→4 fixture retains stable project/version identity, verified snapshots and cloud mapping/settings; existing schema 1/2/future-schema regressions pass |
| Security audit | Redacted current-source/history checks and credential/HTTP/UI/CI/installer review documented in security-audit-0.5.0.md |
| Live API / user's vault / installed upgrade / GUI acceptance | Not performed by agent; explicit user acceptance remains in manual-testing-milestone-5.md |

## Coverage and practical boundaries

Native coverage includes approved baseline creation, immutable supersession, exact/reversed/nonconsecutive pair identity, verified extraction and factual block anchors, complete ordered intermediate timeline evidence, explicit cross-family selection/project isolation, malformed citation/schema/refusal/truncation rejection, prompt-injection/tool/storage boundaries, cache reuse without another call, invalidation after model/limits/baseline changes, paid consent, no inference on capture/view/approval, local review decisions, conservative input/output bounds, rate limits, ambiguous timeout, cancellation of unsent work and startup recovery without paid replay. Existing cloud, credential, source tracking/archival, Word extraction, comparison and navigation suites remain passing.

The six new frontend regressions verify exact selected version pairs survive preview, checked paid consent precedes generation, cached opening sends no Generate action, manually approved baselines make no model call, cancel sends no inference, errors unlock controls without retry, and old unmounted-project replies are ignored. These are component tests; no elaborate native GUI automation was added and no claim of visual acceptance at every scale is made.

Analysis is always one explicitly approved POST /v1/responses with strict JSON Schema, store:false, background:false, no tools and no automatic retry. Cloud indexing is independent. Deterministic local evidence is used for every workflow; remote retrieval augmentation and multi-stage inference are intentionally not implemented. Larger scopes fail locally with a narrower-scope option. Default model is pinned GPT-5.4 Mini; a pinned GPT-5.4 option is supported. Limits: 12 timeline captures, 2–6 distinct consistency families, up to six alignment versions, 65,536 conservative serialized input units and 8,192 max output tokens. Existing extraction/diff limits still apply.

Quotes/IDs/ranges validate provenance, not semantic correctness. Reports are interpretations with uncertainties and review decisions, never rewritten source history. DOCX body-only exclusions remain disclosed. Baseline starting text is extracted locally and explicitly edited/approved; no AI baseline proposal is required or silently generated. Cached reports and baselines are plaintext local application data, not an encrypted content vault. The latest 100 request-history entries are listed; no storage purge, streaming output or multi-stage scheduler is added. An already-sent request may finish and incur charges after cancellation; a valid arriving report is saved. Restart uncertainty requires a new reviewed plan, never automatic replay.

Input token figures are heuristic, with conservative byte-based budget units; price estimates use the saved model/standard-rate assumptions and output maximum. Actual returned numeric token usage is retained when available. Estimates are not official account totals or guaranteed dollar caps. Credential access remains native Windows Credential Manager; the model never receives authorization as evidence. The focused audit cannot inspect private Discord messages or prove the user's real key has never been exposed.

The complete [14-test Windows guide](manual-testing-milestone-5.md) supplies exact v1/v2/v3 and technical-document sample contents, capture sequence, baseline/approval/version-pair/timeline/consistency/cache/cost/offline/failure/security/regression checks, expected API request counts, safe recovery and explicit cleanup. [Architecture](architecture.md) records models, pricing sources, schemas and lifecycle. [Security review](security-audit-0.5.0.md) records audit scope and limitations.

GitHub's retained Windows workflow builds a downloadable ContextDock-Windows-x64-setup artifact (30-day retention) for the intended commit. Local results do not imply a completed remote run; inspect the PR's current commit status. No public release/tag, updater, installation or automatic merge was performed. Local installer: src-tauri/target/release/bundle/nsis/ContextDock_0.5.0_x64-setup.exe.

---

The following verification records describe earlier milestones as historical evidence.

# Verification — ContextDock 0.4.0

Milestone 4 checks ran on Windows 11 on October 19, 2026, using the existing Node 24/MSVC Rust toolchain. Cloud tests use a fully in-memory scripted provider, temporary SQLite databases/source roots/snapshot stores and synthetic nonfunctional key strings. They never read/save a real Windows credential or contact the authenticated OpenAI API. Public official API documentation was consulted. User project folders, real application data, credentials and installed executables were not modified by testing or installer execution.

| Check | Milestone 4 result |
| --- | --- |
| TypeScript / ESLint | Passed |
| Frontend tests | Passed: 35 in 5 files (16 existing + 19 cloud UI) |
| Rust tests / binary targets / doctests | Passed: 87 Rust tests (37 existing + 50 cloud cases); no failures |
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

Four added frontend regressions cover an untested connection and in-project retry, empty selection through successful modal close/Start unlock, save success followed by test failure, and clear queued-change feedback for already-started indexes. Two native regressions cover rejecting empty initial selection while permitting deliberate deselection on a started project, and reporting an untested connection before consent without network calls. These are mocked tests; no real saved credential or authenticated cloud request was used by the agent.

## Windows cloud worker snapshot root

The native worker now canonicalizes its existing database path before deriving the snapshot root, matching Registry initialization. Windows ordinary paths and canonical verbatim paths previously compared unequal and produced “The registered root changed location or became a link.” before the first upload, despite a successfully created index. Snapshot root identity, directory/reparse guards, size limits and SHA-256 verification remain enforced.

A Windows-only regression passes an ordinary database path through the production startup resolver, opens the worker connection and completes guarded snapshot upload/indexing against the scripted provider with exact-byte assertions. The full 87-test Rust suite and all-target Clippy pass. After installing this fix, use **Retry / Reconcile / Verify key** on the existing project to requeue the failed pre-upload job; no index deletion, project registration reset or key replacement is needed.

## Full-history policy transition

A focused regression now completes v1 indexing, replaces it with v2 under current-only policy, verifies v1 is removed, then confirms full history and verifies v1 reattaches using the same retained File ID without another byte upload. Local source archival is disabled throughout. This mocked transition passes; it does not establish the user's running app's saved policy or remote state. Saved project settings now act directly: unchecking indexing opens a short Stop syncing? dialog. Confirmation uses the local disconnect command without a key test, upload consent or worker-lock wait. Unchecking automatic sync likewise confirms a local stop, clears automatic/sync-request flags, cancels pending deletion work and pauses the worker. Existing per-request authorization guards block the next request; an in-flight request may finish. Policy/selection/enabling changes open their review automatically; confirming re-enabled indexing or automatic sync also resumes an already-started project; limit edits open review after blur/Enter. Cancel/Close/Escape restores the saved configuration. Initial setup still collects eligible documents before the first review and requires a separate initial Start.

Seven focused frontend regressions cover immediate policy/selection review, cancel/close rollback, numeric editing and save failure, stop success/failure without a tested key, automatic-stop persistence, and reviewed re-enabling of an existing index. A native regression exercises both stop commands with queued replacement/deletion work and a disconnected key, restores key availability and verifies no further mock request/upload, no remote deletion and preserved local histories/tracking. All 87 Rust tests, 35 frontend tests, TypeScript, ESLint and all-target Clippy pass.

## Cloud index deletion confirmation

Project-index deletion now asks users to type the displayed cloud index name rather than DELETE plus an opaque vector-store ID. The UI sends the index ID separately. Native validation requires both the exact saved name and the unchanged nonempty index ID before credential access; ownership verification and ID-based deletion remain unchanged. Focused UI/native regressions cover a Unicode name, rejecting the old ID phrase, wrong names, stale IDs, renamed indexes and empty/missing identity. Uploaded-File purge retains its separate File-ID confirmation and reference attestation.

## Milestone 4 coverage and boundaries

The 50 cloud tests cover confirmed stop controls blocking queued work without credentials while retaining history and remote resources, Windows runtime database-path normalization before guarded snapshot uploads, local-only/paused no-request behavior, fingerprint isolation, plaintext input validation without echo, fresh and schema-2 migrations, project-scoped eligible selection, ignored/foreign family rejection, stale consent, no uploads before Start, idle index creation before Start, authorization rechecks before each individual request, bounded processing polls, missing/tampered snapshots, soft size limits, original snapshot bytes rather than live-source fallback, asynchronous processing completion, repeated-sync deduplication, current-only safe replacement, historical reattachment/current attributes, disabled/enabled automatic sync, historical current-pointer reuse, interrupted store adoption, no blind recreation on absent candidates, interrupted upload filename/size/hash recovery, mismatched remote bytes, foreign store/association refusal, Retry-After/backoff, terminal auth errors, deleted store/File recovery, missing associations, failed processing retry without reupload, explicit absent-operation reset that refuses existing copies, per-version removal/exclusion including explicit cleanup while sync is disconnected, store deletion retaining Files/local data, separately verified File purge, unregistration/disconnect retaining resources, ambiguity freezing existing selections, direct search/provenance, blocking retrieval until deselection cleanup settles, and not persisting query/excerpts. Scripted calls validate the provider-independent state machine, not a live OpenAI account's permissions, retention or ranking.

The nineteen cloud frontend tests cover reviewed consent before apply with Start separate; unlocking controls after a failed action; late responses after unmount; immediate clearing of the uncontrolled password field without browser-storage writes; cancellation/typed confirmation for remote deletion; actionable configuration failures inside the consent dialog; explicitly confirmed association cleanup while sync remains paused/disconnected. All previous rendering/navigation/tracking/history/diff regressions pass. These are focused mocked component tests, not a broad native GUI harness or a claim of visual acceptance on every display scale.

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
