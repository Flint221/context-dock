# ContextDock

ContextDock 0.7.0 is a lightweight Windows 11 application for local document history and optional project questions. Register a folder, enable document tracking, then turn on **Cloud sync** to sync selected documents to OpenAI and ask questions about their current indexed versions. Cloud sync off keeps local history and saved chats available while blocking new project AI requests.

## Folder tree and saved exclusions (0.7.0)

Open a project → **Folder structure and selection** to expand nested folders and choose folders or individual files. Checkboxes are drafts: **Save changes** applies them, and **Discard changes** restores the saved selection. Leaving the project or closing the app with a draft offers **Keep editing**, **Discard changes**, or **Save changes and continue**. A failed save keeps the draft open; closing during a save waits for its result.

An excluded folder covers every descendant, including files discovered later, and survives restart. Selecting a parent again includes its descendants. Discovery still shows supported Markdown/Word files, including excluded files; saved exclusions stop new captures and exclude the affected versions from the project's cloud index. Local source files, snapshots, archives, version history and chats remain intact. Reassigning the project folder preserves relative exclusions.

Saving removes affected cloud index associations before unrelated upload work and blocks new questions until removal is verified. Failed removals retain their identifiers and a retry action. Raw uploaded Files remain reusable under the existing retention policy. Including paths again follows the saved cloud selection/current-or-history policy without enabling tracking, enabling cloud sync, resuming paused uploads or overriding separate document/version opt-outs. Saved removals can finish while uploads are paused or cloud data was explicitly retained with sync off.

See the [folder-selection acceptance walkthrough](docs/manual-testing-folder-selection.md). Schema 007 upgrades the registry in place and migrates legacy ignored-file choices; do not downgrade to an older application afterward.

0.6.5 fixed project removal being blocked by a rejected uploaded-file download during cloud cleanup. Cleanup verifies the scoped upload journal and file metadata without downloading the file, handles resources already absent or disappearing during cleanup, and retains deletion receipts/absence checks. The dialog also reads back verified cleanup after a lost command reply. Errors use red text throughout the interface; native API failures include HTTP status and cleanup-stage context.


## Simplified local previews and final polish (0.6.4)

**Current processes** now opens in **Local previews** mode by default. Compact cards show likely local frontend/web servers with their URLs, **Preview**, **Copy URL**, **Stop app**, and a reviewed **Stop all previews** action. System services, UDP, common database/debug ports and unrelated background listeners are hidden from this view. Ordinary dual-stack localhost bindings are grouped to reduce duplicates. **Advanced processes** restores the complete original inventory, filters, selections and startup diagnostics; switching views clears pending selections, and reopening the tab defaults to Local previews again.

This is a display heuristic using verified current-user process eligibility, local TCP bindings, runtime names and common preview ports. It does not probe sites or claim every result speaks HTTP; unusual/custom servers may require Advanced processes. Stopping remains a confirmed whole-process action, with all existing identity/protection checks. Automated checks and local installer packaging pass; installed acceptance remains a user check.

Small text is larger throughout the interface, with emphasized version values. Green indicates active/available/success states, amber indicates warnings or paused/review states, and red indicates errors/missing items and destructive actions. Status labels remain visible in both themes. Project removal closes its dialog and returns to All projects using the verified snapshot; older background reads cannot restore the removed card. Local source files, captured history, archives and chats remain retained, with cloud deletion controlled by the removal choice.

The cloud-inside-folder logo is shared by the sidebar, browser favicon and native Windows assets. Edit `public/contextdock.svg` and run `npm run icons` to regenerate the tracked PNG/ICO files and `app-icon.png`; this command does not build an installer. Installed Windows icons update with the next installer.

## Motion and feedback (0.6.3)

Pages reveal from top to bottom with short, capped stagger delays. Buttons respond to hover/press, project cards lift gently, dialogs ease into view, and the sidebar clearly marks the active page. Primary Refresh/Sync actions are easier to find, regular buttons have at least 40-pixel targets, and compact process controls remain at least 34 pixels tall. Light and dark themes share the same visual hierarchy.

Actions show animated progress tracks; unknown durations stay indeterminate. Cloud indexing uses actual indexed/pending/failed counts rather than a timer, so failures do not count as completion. Page navigation returns to the top, while ordinary registry/cloud updates preserve controls and do not replay page transitions. Optional automatic process refresh is quiet to avoid repeatedly shifting the list. Motion follows the operating system's reduced-motion preference, retaining static progress, visible focus and all actions. No animation library, new background timer, preference or schema migration was added. See the [motion walkthrough](docs/manual-testing-motion.md).

## Current processes (0.6.2)

Open **Current processes** above Settings to inspect Windows TCP listeners and UDP bound endpoints across IPv4/IPv6 and all interfaces. Search names, PIDs, paths or ports, filter protocol/interface, or switch to **User apps and listeners** to include eligible apps without listening ports. Each row shows the executable, PID, parent PID, start time and endpoints. **Preview** opens local TCP listeners in the default browser; **Use HTTPS for previews** switches the scheme. Not every TCP listener is a website. Copy endpoints or process details for troubleshooting.

**Stop**, **Stop selected**, and **Stop all shown** review the exact owning processes before force termination. Bulk actions use the current filters across all pages and exclude protected processes. Stopping an owner closes all its ports and can lose unsaved work. ContextDock, its ancestors/helpers, Windows system processes, services and other sessions are protected; Windows may refuse other targets. Creation times are verified to avoid stopping a replacement process with a reused PID. Each result distinguishes confirmed exit from failure or unconfirmed termination. Child processes are not automatically stopped. No process is classified or stopped automatically as "lingering."

Refresh is manual by default. Optional five-second polling stops when unchecked, while a stop review is open, when the page is hidden, or when navigating away. Preview HTTPS and selections are temporary controls that can be reversed; reopening the tab resets them. **Inspect startup entries** reads Run/RunOnce registrations and Startup folder item names; an entry may be disabled in Windows and is not proof of what launched an app. **Windows Startup apps** opens the system settings. Stopping a process does not disable startup, scheduled tasks, services or parent-app relaunching. This feature runs locally without a cloud key, elevation, persisted inventory or a new background service.

See the [process diagnostics test walkthrough](docs/manual-testing-processes.md).

## Moved or missing project folders (Milestone 6)

Move or rename a project folder in Explorer, then open its existing ContextDock project and choose **Reassign folder**. Select the project folder at its new location, review the known/unchanged/changed/additional documents and exact missing paths, confirm the folder belongs to this project, then **Confirm reassignment**. The project ID, document histories, snapshots, saved chats, cloud mappings and tracking/archival preferences are retained. Reassignment changes the registration; it does not move files or combine registrations.

Missing, inaccessible or replaced roots show **Folder lost — please reassign it**. This state persists across closing/reopening the app and stops project tracking and new cloud/AI work until explicit reassignment. Restoring the original location still requires selecting it through Reassign folder. Other projects, navigation and app shutdown remain available. Folder health is checked even with tracking off; a different directory occupying the old path is detected using its filesystem identity.

Missing individual files remain listed with their original history. Restore them and **Refresh documents**. Selected missing/inaccessible files block syncing and new questions; saved chats remain readable once the folder is reassigned. After reassignment, opted-in tracking rechecks the new root; questions wait for local reconciliation and any changed versions to finish indexing. An already-sent network request may finish. Invalid/unreadable destinations, links/junctions, app-storage overlap, other current/retained project roots and stale reviews fail visibly. An empty or incorrect parent folder lists every known file as missing; review carefully before confirming.

See the [Milestone 6 folder recovery walkthrough](docs/manual-testing-milestone-6.md). Schema 006 upgrades existing databases in place; do not downgrade to a pre-0.6.0 build afterward.

## Changing your cloud selection (0.6.1)

Unchecking a document family under **Advanced cloud settings** reviews and saves an explicit opt-out. All its indexed captures are removed from this project's store before unrelated uploads, including while uploads remain paused. Each removal checks ownership, the deletion receipt and remote absence; failures retain identifiers and show **Retry document removals**. New questions stay blocked until removals are verified. Pure deselection can be saved even if sources or the tested key are temporarily unavailable; actual cleanup still needs the authorized original account/key.

Selecting the family again restores the captures allowed by the saved current/full-history policy in the same project store, reusing retained uploads. A previously removed individual version keeps its separate explicit exclusion. Turning **Cloud sync off** deletes all stores and uploads owned by this installation/project, including older owned stores. Re-enabling creates a fresh owned store and uploads the saved selection. An empty or ineligible saved selection requires review; the app does not silently select other documents.

**Pause** and **automatic sync off** stop new upload work and retain cloud data; previously confirmed document removals may finish. Local tracking and archival have their own controls and retain captured history when stopped. See the [complete toggle/reversal audit and acceptance checks](docs/toggle-audit.md).

## Ask project (0.6.1)

Enter sends; Shift+Enter adds a new line. Sources show the original filename and capture number without passage previews. Follow-ups include up to six recent completed exchanges, bounded to 12,000 UTF-8 bytes; older messages stay saved locally.

Open a project → **Cloud sync**. One confirmation enables automatic syncing of the displayed document selection and starts initial sync. After documents are ready, type a question in **Ask project** and click **Send**. No baseline, analysis mode or per-question preview is required. Advanced cloud selection, version policy, recovery and remote cleanup remain under **Advanced cloud settings**; the **Remove project** action is visible beside Refresh documents. Folder and rename actions are under **Project folder and options**.

Major settings require confirmation before changing: project Cloud sync, advanced pause/resume and automatic sync, document tracking/archival, and global OpenAI disconnect. While applying, controls show a spinner and ignore duplicate clicks. Success is shown only after backend state is verified; mismatches and failed status reads show an error. An unverified project status blocks questions/settings changes until refreshed. Confirming a sync request does not mean document indexing has finished; Ask project still waits for indexed current versions.

Each Send searches the project's vector store, verifies returned project/document/version provenance and generates one answer from those passages. It always searches current versions, even when full captured history is indexed. Pending local versions block answering instead of silently using outdated files. Follow-up questions use recent messages from the same chat; earlier answers are context, not authoritative evidence. Answers show source document names, capture versions and the retrieved passages. Search is partial evidence, not an exhaustive version comparison.

Use **New chat** for another conversation. Chats and answers are stored locally in `contextdock.db` under the existing application data directory; they survive app restarts and can be read with Cloud sync off. **Delete chat** deletes only that local conversation and its messages, never project files, snapshots or cloud resources. No remote conversation is created. Historical 0.5.0 baselines/reports are retained in the database but their authoring interface is removed from the main project/history workflow for this pass.

Send is an explicit paid OpenAI action, using pinned GPT-5.4 Mini. There are no automatic model calls or paid retries. Empty retrieval returns a local explanation without a model call. Responses use `store:false`, `background:false` and no tools; provider processing/retention rules still apply. This first pass uses explicit vector search followed by one bounded Responses request; answers stream live through Responses SSE. Completed replies are saved only after final response and source validation; interrupted previews are cleared and nothing retries automatically. The AI tool interface remains future work. See the [simple chat test guide](docs/manual-testing-chat.md). The [earlier Milestone 5 guide](docs/manual-testing-milestone-5.md) records the superseded baseline workflow.

## Implemented through Milestone 4

- Dashboard, project registration, metadata discovery/search, rename, clipboard paths, Explorer locations, non-destructive unregistration, and light/dark/system themes.
- Per-project **Document tracking**, initially off for existing and new registrations. Select initial files; each starts its own independent v1 history. Initialization never moves existing files or groups similarly named historical exports.
- Native filesystem monitoring while the app is running, debounced writes, bounded retries for unstable/locked files, and startup reconciliation of missed changes.
- SHA-256-verified immutable local snapshots, independent version numbering, duplicate-content prevention, and remembered manual associations.
- Conservative matching of trailing version/date filenames within the same directory. Ambiguous matches and duplicate copies go to **Needs Review** for assignment, independent tracking, or ignore.
- Separately enabled archival into **Old Context Files**. In-place edits archive a copy of the earlier snapshot. Confirmed differently named replacements can move the previous source only after both snapshots and the old source hash are verified.
- Document history with capture reasons, hashes, source metadata, snapshot/archive paths and availability, plus persisted activity and recoverable pending operations.
- Offline Markdown and DOCX body-text comparisons between any two captured versions of one document: verified snapshots, accurate line changes, unified/split display, paged context and change navigation. Equivalent text is distinguished from identical file bytes.
- An unsigned per-user NSIS installer and Windows GitHub Actions build artifacts. Matching version tags can publish installers, but no public release is created automatically during development.

**Discovery** reads metadata. **Tracking** reads document bytes to capture local immutable snapshots. **Archival** can move previous source files, and requires its own explicit project opt-in. Tracking alone never moves sources. Disabling tracking stops monitoring and retains history. There is no Windows service; close the app to stop it, and reopening catches up.

## Download and install

1. Close any running ContextDock window before upgrading.
2. Open [Windows installer workflow runs](https://github.com/Flint221/context-dock/actions/workflows/windows.yml). Choose a successful run for the intended branch/commit, then download **ContextDock-Windows-x64-setup** under **Artifacts**. Workflow artifacts require GitHub sign-in and expire after 30 days.
3. Extract the ZIP and run `ContextDock_0.6.3_x64-setup.exe`. The installer retains the existing application identifier and per-user data location. Earlier databases migrate transactionally through schema 6; existing tracking/archival/cloud preferences and snapshots survive. AI requests remain off until explicitly approved. Do not delete app data or reinstall into a new application identity.
4. Launch **ContextDock** from Start. Existing projects, history and theme remain. Tracking/archival initially remain off for schema-1 migrations; existing schema-2 tracking preferences are preserved.
5. Follow the complete [14-test Milestone 4 cloud guide](docs/manual-testing-milestone-4.md) using disposable files and your own API account. The [Milestone 3 comparison guide](docs/manual-testing-milestone-3.md) and [Milestone 2 tracking guide](docs/manual-testing-milestone-2.md) remain available for offline/archival testing.

Installers are unsigned. Windows SmartScreen may show an unknown-publisher or reputation warning; follow your organization's installation policy and verify that the file came from this repository. Windows 11 normally includes WebView2. If it is absent, the initial Microsoft WebView2 bootstrapper requires internet; the application then works offline.

The x64 build runs on Windows 11 x64 and supported Windows 11 ARM devices through emulation. No automatic updater is implemented. A future successful matching version tag exposes an installer under [GitHub Releases](https://github.com/Flint221/context-dock/releases); a PR alone does not publish a release.

## Optional OpenAI cloud indexing

Cloud is **off by default**, including on upgrade. Local tracking, archival and comparisons work without a key or internet. In Settings, use **Save and test key** to save your personal key in **Windows Credential Manager** and verify it with a read-only listing (no upload). Never put the key in chat, an environment file or the repository. The password field clears on Save; SQLite/browser storage never receives the key.

In each project, select eligible tracked logical document families, choose **Current version only** (default) or **Full captured history**, review file counts/raw bytes/additions/removals, and explicitly consent to upload/retention/API charges. Click **Start Initial Sync** separately. New projects do not upload just because a global key exists. The separate **Automatically sync newly captured versions** checkbox is off by default. Resume cloud sync can create the owned store before Start without uploading documents.

Each project owns a persisted OpenAI vector store. Only committed SHA-256-verified immutable snapshots are uploaded as original Markdown/DOCX bytes; never a live-source fallback or diff extraction. Uploaded, processing, indexed, failed and uncertain states are distinct. The native cloud worker uses its own connection/lock, short SQL transactions, bounded backoff and restart reconciliation without holding the local registry mutex across HTTP. Advanced **Test Search** returns excerpts and actual family/version/File provenance. Use **Ask project** for generated answers.

Use Sync Now, Pause/Resume and Retry/Reconcile for control. Current-only replacement keeps old indexed content until replacements finish, then detaches superseded associations. Full history retains earlier selected captures. Changing a saved policy or document selection opens its confirmation preview automatically; Cancel restores saved values. Unchecking project indexing confirms deletion of its entire vector store and uploaded Files. It stops new work first and verifies remote removal before showing Off; a saved, tested original key is required for cleanup. Unchecking automatic sync asks to stop automatic syncing and pauses queued work; Sync Now remains available for manual sync. Upload-limit edits open review when you leave the field or press Enter. Ambiguous or ignored local families do not become upload candidates.

**Cloud cleanup:** turning Cloud sync off deletes this project's vector store bucket **and its uploaded Files**, including earlier uploads, after confirmation. Removal waits for deletion acknowledgements, remote absence checks and local state verification. Failures retain identifiers, pause uploads and display **Cloud deletion incomplete** with a retry action. Re-enabling creates a fresh bucket and uploads selected captured versions according to the saved version policy. Pause/automatic sync off explicitly retain cloud data temporarily.

**Remove project** asks whether to delete cloud data (default) or explicitly retain it in OpenAI. With deletion selected, the registration stays available until cloud cleanup succeeds. Local source files, snapshots, history and chats remain on this computer. Retained remote data may continue to incur storage charges; re-register the same folder to recover its mappings for later cleanup. Global OpenAI disconnect requires active projects to be cleaned first, or removed with an explicit retain choice. Uninstalling cannot delete remote resources. Cleanup only targets this installation/project's owned resources and verified upload identities; shared files, mismatched ownership, inaccessible resources and failed verification remain visible errors. No authenticated cleanup is performed automatically on upgrade.

Settings shows raw indexed snapshot bytes separately from last measured OpenAI index usage and links to [usage](https://platform.openai.com/usage), [billing](https://platform.openai.com/settings/organization/billing/overview) and [current pricing](https://developers.openai.com/api/docs/pricing). Parsed/indexed bytes differ from file size; account allowances may be shared. The soft upload-size limit is not a dollar spending cap and indexing is not guaranteed free. Application upload bounds are 16 MiB raw and 4 MiB extracted text/expanded DOCX; provider type/token limits also apply. Follow [the installed-app walkthrough](docs/manual-testing-milestone-4.md) for exact samples, 14 tests, recovery and cleanup. Live API/upgrade behavior requires user acceptance testing.

## First tracking workflow

Register a folder using **Add project**, a display name, and **Browse**. On its detail screen choose **Enable tracking…**, select initial files, and click **Capture selected and enable**. Initially selected files are independent families, even when their names look related. Unselected existing files remain discoverable but are ignored until you choose **Track independently**.

The separate **Enable automatic archival of replaced source files** checkbox authorizes archival for future replacements. New unrelated files become independent families automatically. Exact tracked paths remain associated with their family. A unique, normalized version/date stem in the same folder can match automatically; conflicting candidates, different workstream folders and live duplicate copies require review. Assigning a pending file remembers its normalized name and directory association for future exports.

Use **Scan for changes** or **Refresh documents** for manual reconciliation. **View history** opens captured versions; **Snapshot location** opens a verified preserved location in Explorer. Open the file there in Notepad or Word to inspect it. It is marked read-only; do not edit the preserved snapshot. No rollback is provided.

Turning archival off pauses pending archive operations; enabling it again safely retries them. Turning tracking off pauses all tracking work; **Resume tracking** reconciles changes made while paused. Meaningful events and failures remain in **Recent tracking activity** and **Pending operations**. Fix permissions/locks/conflicts before choosing **Retry pending operations**. The application never overwrites an archive collision or deletes a conflicting source to make an operation succeed.

## Local version comparison

From **View history → Compare Versions**, select either version independently and click **Compare selected versions**. The latest two captures are selected by default; reversed selections are ordered by internal capture number. Switch Unified/Side-by-side modes, use Previous/Next change, expand unchanged context and page through long regions. Copy comparison summary or all changed text to the clipboard. One captured version has a helpful second-capture explanation; with two captures, selecting the same version on both sides provides an identical-content check without defeating tracking deduplication.

Markdown requires UTF-8. BOM and CRLF/LF differences are normalized with warnings; whitespace, blank lines and final-newline changes remain detectable. DOCX extraction compares body text, paragraphs/runs and tables, not Word rendering or Track Changes markup. Formatting-only/package changes can have equivalent text but different bytes. The interface discloses excluded structures and accepted-change policy; see [extraction scope and safety](docs/architecture.md#local-comparison-milestone-3).

Comparisons resolve existing version UUIDs, never active files or arbitrary frontend paths. Missing, corrupt, locked or unsupported snapshots fail visibly. No comparison cache, new versions, duplicate snapshots, AI calls or export writes are created. Limits per side: 16 MiB snapshot, 4 MiB extracted text, 20,000 lines and 64 KiB per line; exact Myers calculation has a 2-second complexity budget. ZIP/XML have additional bounds. The worker releases the registry mutex before reading/parsing/diffing. The UI renders at most 100 lines or aligned rows at a time; all changed content remains reachable.

## Storage, retention and safety

Tauri resolves `%LOCALAPPDATA%\com.contextdock.desktop` on a normal Windows installation:

```text
contextdock.db                         Registry, versions, preferences and operation journal
snapshots/<project UUID>/<document UUID>/<version UUID>.<extension>
staging/<random UUID>.tmp               Temporary snapshot staging
runtime.lock                           Single running process per application store
```

Snapshots remain independent of installed executables and source folders. Back up the whole application data directory with ContextDock closed, including any SQLite WAL sidecars. Do not downgrade to 0.1.0 after migration; it does not understand the new schema.

Archive files live inside the registered project:

```text
Old Context Files/<safe family name> - <document UUID>/
  v1 - <safe original stem> - <operation UUID>.md
```

UUID suffixes prevent collisions; exact original filenames remain in history metadata. Existing unrelated files in a pre-existing archive directory are preserved. `Old Context Files` is excluded case-insensitively from discovery and monitoring. Editing an archive cannot change its independent immutable snapshot; opening an edited archive location reports a hash conflict.

Unregistering hides the project and disables tracking/archival. It does **not** delete source files, archives, snapshots, or retained history metadata. Registering the same canonical folder again restores its stable project ID/history with tracking and archival off. No automatic archive/snapshot retention cleanup or storage-purge feature is implemented.

Snapshots use flushed staging files, no-overwrite finalization and readonly sealing. SQLite's durable operation journal records capture and archival intent; interrupted finalization can be retried without duplicating committed versions. Windows archive moves hold write/delete-denied source handles, verify both snapshots and the replacement, verify the old source hash, persist its file identity, and rename that exact handle without replacement. Directory guards reject path traversal, symbolic links and reparse points. A destination collision, unexpected edit, lost ownership evidence, unsupported filesystem or sharing violation leaves a visible pending operation rather than falling back to deletion.

Scanning also excludes `.git`, `node_modules`, `target`, `dist`, `build`, `.next`, `.contextdock`, `.contextdock-archive`, `contextdock-archive`, child symlinks/junctions and Word `~$` lock files. Tracking is refused for roots overlapping the app's own storage. Missing sources retain their observations and version history.

## Development

Stack: Tauri 2; Rust; React/TypeScript/Vite; Tailwind CSS 4; Lucide icons; bundled SQLite via rusqlite; notify native watchers; SHA-256; official Tauri dialog plugin; NSIS.

Prerequisites: Node.js 24 LTS/npm, stable Rust x86_64 MSVC with rustfmt/clippy, Visual Studio 2022 Desktop development with C++ build tools and a Windows SDK, and Microsoft Edge WebView2. See [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) and [Windows installer instructions](https://v2.tauri.app/distribute/windows-installer/).

```powershell
npm ci
npm run tauri -- dev
```

`npm run dev` alone has no native filesystem/database service; use the desktop command.

```powershell
npm run typecheck
npm run lint
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run tauri -- build --bundles nsis --ci
```

Installer output: `src-tauri/target/release/bundle/nsis/ContextDock_0.6.3_x64-setup.exe`. Both dependency lockfiles are committed. Tests use mocked cloud APIs and temporary roots/databases; no real keys or destructive tests run against registered user projects.

The existing Windows workflow runs checks, Rust safety tests, frontend tests and NSIS packaging on PRs to `main`, pushes to `main`/`codex/**`, and manual dispatches. Tagged releases require a tag matching all app versions. No release tag is created automatically. Keep `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json` synchronized. Only the isolated tag-publishing job has repository write permission.

See [architecture](docs/architecture.md), [roadmap](docs/roadmap.md), and [verification](docs/verification.md). Milestone 5 now presents simple cloud-backed questions and local chats; earlier analysis/baseline records are retained. Milestone 6 begins with persistent folder recovery and safe reassignment. Signing, storage management and automatic update delivery remain future work.
