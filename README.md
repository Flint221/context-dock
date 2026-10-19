# ContextDock

ContextDock 0.1.0 is a lightweight, local-first Windows 11 desktop utility for finding project documentation across existing folders. Each project can contain many independent Markdown and Word documents. Everything in Milestone 1 runs locally, without an account, telemetry, or cloud service.

## Milestone 1

- Dashboard with real project/document counts, recent file observations, and folders needing attention.
- Register an existing folder using the native Windows folder picker and a custom display name.
- Rename the registration, copy its folder path, open File Explorer, or remove its metadata after confirmation.
- Recursively discover `.md`, `.markdown`, and `.docx` metadata; search and page through the document table.
- Preserve document IDs on repeated observations, including missing-file metadata. Unreadable folders and partial scans are reported explicitly.
- Persist projects, document observations, and light/dark/system theme preferences in SQLite.
- Per-user NSIS Windows setup, with builds and installer artifacts in GitHub Actions and release packaging on version tags.

Discovery reads filenames, paths, sizes, and timestamps. It does not read document contents, create versions, or modify registered folders. Removal only deletes ContextDock metadata. Dashboard data reflects the most recent scan; opening a project or choosing **Refresh documents** performs a new scan.

## Download, install, and try it

1. Open [Windows installer workflow runs](https://github.com/Flint221/context-dock/actions/workflows/windows.yml). Choose a successful run for the branch/commit you intend to test.
2. Under **Artifacts**, download **ContextDock-Windows-x64-setup** (GitHub sign-in is required for workflow artifacts). Extract the ZIP and run `ContextDock_0.1.0_x64-setup.exe`.
3. Alternatively, after a matching version tag has built successfully, download the setup executable directly from [GitHub Releases](https://github.com/Flint221/context-dock/releases). A release does not exist merely because a PR is open.
4. Install for your Windows user, then launch **ContextDock** from Start. The x64 build also runs on supported Windows 11 ARM machines through Windows emulation.
5. Select **Add project**, enter a name, click **Browse**, select an existing folder, and save. Open another folder as a second project to verify independent document lists.
6. In a project, test search, **Copy path**, **Explorer**, **Rename**, and **Refresh documents**. Close and reopen ContextDock to verify persistence.
7. For a missing-file test, use a disposable folder you created: add a Markdown file, refresh, then move that file outside the folder and refresh again. Its metadata remains with a missing status. Restore it and refresh to recover the same ID.
8. **Remove** unregisters the project after confirmation; verify the original folder and its files still exist.

Installers are unsigned. Windows SmartScreen may identify the publisher as unknown or show a reputation warning. Follow your organization's installation policy and verify that the installer came from this repository. No automatic updater is implemented. Future installers use the same application identifier so per-user data survives upgrades.

The application itself works offline. On a Windows installation missing WebView2, initial setup uses Microsoft's bootstrapper and needs internet to install that prerequisite. Windows 11 normally includes WebView2.

## Stack and prerequisites

Tauri 2, React, TypeScript, Vite, Tailwind CSS 4, Lucide icons, Rust, bundled SQLite via rusqlite, the official Tauri dialog plugin, and NSIS.

For Windows development install:

- Node.js 24 LTS and npm.
- Stable Rust with the `x86_64-pc-windows-msvc` toolchain, `rustfmt`, and `clippy`.
- Visual Studio 2022 Build Tools: **Desktop development with C++**, the MSVC x64/x86 build tools, and a Windows SDK.
- Microsoft Edge WebView2 Runtime.
- Git. GitHub CLI is only needed if manually publishing releases.

See the official [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) and [Windows installer documentation](https://v2.tauri.app/distribute/windows-installer/). The CLI downloads NSIS build tools as needed. Both npm and Cargo lockfiles are committed.

## Development and checks

```powershell
npm ci
npm run tauri -- dev
```

`npm run dev` alone starts the frontend in a browser but has no native registry or folder dialog; use the desktop command for working functionality.

```powershell
npm run typecheck
npm run lint
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Tests use disposable directories and databases. Rust coverage includes migration/reconnection, project management, canonical duplicate rejection, recursive mixed document discovery, duplicate names in subfolders, ignored directories, stable IDs, missing/reconnected folders, Windows junction exclusion, Windows permission errors, and read-only scanning. React tests mock the native service to verify picker cancellation, registration/error feedback, removal confirmation cancellation, and theme interaction. They complement native tests; they do not verify the actual Windows picker.

## Build the installer

```powershell
npm ci
npm run tauri -- build --bundles nsis --ci
```

Output: `src-tauri/target/release/bundle/nsis/ContextDock_0.1.0_x64-setup.exe` when building with the x64 toolchain. A production frontend build is included automatically. Unsigned builds need no signing secrets.

The **Windows installer** workflow runs for PRs to `main`, pushes to `main` and `codex/**`, manual dispatches, and `v*` tags. It runs checks and tests, builds NSIS, and retains the installer artifact for 30 days. Only the separate tag release job has repository write permission. Actions are pinned to commit SHAs and checkout does not persist credentials.

After review/merge, a maintainer can publish a matching tag (not performed automatically by this PR):

```powershell
git switch main
git pull --ff-only
git tag v0.1.0
git push origin v0.1.0
```

The tag must match the version in `package.json`. Keep `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` versions synchronized for subsequent releases. The workflow publishes a GitHub Release only after the build succeeds.

## Local data and filesystem safety

Tauri resolves its supported per-user local app-data directory. On Windows the registry is normally `%LOCALAPPDATA%\com.contextdock.desktop\contextdock.db`, with SQLite WAL sidecars alongside it. It is independent of the checkout, installation, and registered project folders. Close ContextDock before backing up the directory. Do not commit the database or personal project registrations.

Migrations use SQLite `user_version` and transactions. UUIDs identify projects and independent logical documents; UTC RFC 3339 timestamps record observations. A normalized canonical folder key prevents duplicate registrations under Windows case-insensitive path semantics. Relative paths distinguish identical filenames in different subfolders. A renamed/moved file currently appears as a new document, while its old observation remains missing; explicit identity reconciliation belongs to future milestones.

Scans skip child symlinks and Windows reparse points (including junctions), `.git`, `node_modules`, `target`, `dist`, `build`, `.next`, `.contextdock`, `.contextdock-archive`, and `contextdock-archive`. Only a complete traversal can mark old observations missing. A partial scan retains uncertain documents as unavailable. A disconnected drive retains its registrations and document metadata. No filesystem watcher is installed.

## Architecture and roadmap

See [architecture](docs/architecture.md), [roadmap](docs/roadmap.md), and [verification record](docs/verification.md). Future milestones cover safe archival, independent version histories, local diffing, optional cloud indexing, on-demand AI analysis, and signed updates. None of those capabilities are represented as working Milestone 1 features.
