# Verification — ContextDock 0.2.0

Local checks completed on Windows 11 on October 19, 2026, using Node.js 24.12.0, Rust 1.99.1 and the x86_64 MSVC toolchain. The host is ARM64, running x64 executables through Windows emulation. Milestone 1 is the merged foundation; the following record covers Milestone 2.

| Check | Result |
| --- | --- |
| `npm run typecheck` | Passed |
| `npm run lint` | Passed |
| `npm test` | Passed: 9 tests in 3 files |
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

## Installer and manual verification boundary

The NSIS executable builds locally using the existing per-user configuration and application identifier. No release tag or public GitHub Release was created. The 0.2.0 installer has not been executed against the user's existing installation/database in this milestone. Interactive upgrade, Explorer/clipboard, actual Word saves/locks, SmartScreen, clean-machine prerequisites, uninstall and large-project usability checks remain manual checks. No claim of passing those interactions is made.

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
