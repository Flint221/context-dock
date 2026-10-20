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
