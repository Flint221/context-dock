# ContextDock

ContextDock 0.4.0 is a lightweight, local-first Windows 11 application for project documentation. Register existing folders, discover Markdown and Word files, and opt in to independent version histories and safe archival. Compare preserved versions entirely offline. Optional OpenAI indexing requires your own API key, explicit project selection and upload consent.

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
3. Extract the ZIP and run `ContextDock_0.4.0_x64-setup.exe`. The installer retains the existing application identifier and per-user data location. Earlier databases migrate transactionally to schema 3 with cloud off; tracking/archival preferences and snapshots survive. Do not delete app data or reinstall into a new application identity.
4. Launch **ContextDock** from Start. Existing projects, history and theme remain. Tracking/archival initially remain off for schema-1 migrations; existing schema-2 tracking preferences are preserved.
5. Follow the complete [14-test Milestone 4 cloud guide](docs/manual-testing-milestone-4.md) using disposable files and your own API account. The [Milestone 3 comparison guide](docs/manual-testing-milestone-3.md) and [Milestone 2 tracking guide](docs/manual-testing-milestone-2.md) remain available for offline/archival testing.

Installers are unsigned. Windows SmartScreen may show an unknown-publisher or reputation warning; follow your organization's installation policy and verify that the file came from this repository. Windows 11 normally includes WebView2. If it is absent, the initial Microsoft WebView2 bootstrapper requires internet; the application then works offline.

The x64 build runs on Windows 11 x64 and supported Windows 11 ARM devices through emulation. No automatic updater is implemented. A future successful matching version tag exposes an installer under [GitHub Releases](https://github.com/Flint221/context-dock/releases); a PR alone does not publish a release.

## Optional OpenAI cloud indexing

Cloud is **off by default**, including on upgrade. Local tracking, archival and comparisons work without a key or internet. In Settings, use **Save and test key** to save your personal key in **Windows Credential Manager** and verify it with a read-only listing (no upload). Never put the key in chat, an environment file or the repository. The password field clears on Save; SQLite/browser storage never receives the key.

In each project, select eligible tracked logical document families, choose **Current version only** (default) or **Full captured history**, review file counts/raw bytes/additions/removals, and explicitly consent to upload/retention/API charges. Click **Start Initial Sync** separately. New projects do not upload just because a global key exists. The separate **Automatically sync newly captured versions** checkbox is off by default. Resume cloud sync can create the owned store before Start without uploading documents.

Each project owns a persisted OpenAI vector store. Only committed SHA-256-verified immutable snapshots are uploaded as original Markdown/DOCX bytes; never a live-source fallback or diff extraction. Uploaded, processing, indexed, failed and uncertain states are distinct. The native cloud worker uses its own connection/lock, short SQL transactions, bounded backoff and restart reconciliation without holding the local registry mutex across HTTP. Direct **Test Search** returns excerpts and actual family/version/File provenance; no generated answer or model analysis is implemented.

Use Sync Now, Pause/Resume and Retry/Reconcile for control. Current-only replacement keeps old indexed content until replacements finish, then detaches superseded associations. Full history retains earlier selected captures. Policy/selection changes require a fresh confirmation preview. Ambiguous or ignored local families do not become upload candidates.

**Remote retention matters:** uploaded Files are separate from vector-store associations. Automatic replacement, disconnect, unregistration and uninstall do not delete underlying Files. View Cloud Index offers confirmed per-version association removal and separate owned-File deletion; the panel offers strong-confirmation project-index deletion. File purge verifies ownership, exact bytes and references in accessible vector stores, and requires you to verify references in other tools/resources. Inaccessible/foreign resources are not automatically deleted. Keep registrations until cleanup, or re-register the same root to recover mappings.

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

Installer output: `src-tauri/target/release/bundle/nsis/ContextDock_0.4.0_x64-setup.exe`. Both dependency lockfiles are committed. Tests use mocked cloud APIs and temporary roots/databases; no real keys or destructive tests run against registered user projects.

The existing Windows workflow runs checks, Rust safety tests, frontend tests and NSIS packaging on PRs to `main`, pushes to `main`/`codex/**`, and manual dispatches. Tagged releases require a tag matching all app versions. No release tag is created automatically. Keep `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json` synchronized. Only the isolated tag-publishing job has repository write permission.

See [architecture](docs/architecture.md), [roadmap](docs/roadmap.md), and [verification](docs/verification.md). Milestones 5–6 remain future work: on-demand AI interpretation/baselines, signing and automatic update delivery.
