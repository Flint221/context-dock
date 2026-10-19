# Milestone 1 verification

Verified locally on Windows 11, October 19, 2026, using Node.js 24.12.0 and stable Rust 1.99.1 with the x86_64 MSVC toolchain. The host is Windows ARM64; the x64 application runs through Windows emulation.

## Automated results

| Check | Result |
| --- | --- |
| `npm run typecheck` | Passed |
| `npm run lint` | Passed |
| `npm test` | Passed: 5 frontend tests across 2 files |
| `npm run build` | Passed: TypeScript and production Vite bundle |
| `npm audit` | Passed: zero reported vulnerabilities |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | Passed |
| `cargo test --locked --manifest-path src-tauri/Cargo.toml` | Passed: 6 Rust tests |
| `cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | Passed |
| `npm run tauri -- build --bundles nsis --ci` | Passed: x64 release application and NSIS setup, approximately 2.7 MiB |

Rust tests cover migration initialization/reopening and newer-schema rejection; persistent creation, rename and safe metadata removal; canonical and case-insensitive duplicate rejection; multiple independent and nested Markdown/Word observations; identical filenames in separate directories; exclusions and junction safety; missing/reconnected folders and files; real Windows ACL denial at both root and subfolder level; retained uncertain metadata; stable UUIDs; and unchanged source file bytes/modification times.

Frontend tests cover initial registration through a mocked native picker, picker cancellation, duplicate-registration error feedback, rename, manual refresh, removal cancellation and final confirmation, settings version/theme interaction, and canonical drive/UNC path presentation. Mocks verify service wiring, not the Windows picker implementation.

## Native packaging and smoke checks

- Production executable launched as a native Windows desktop window.
- Dark/system welcome state, sidebar, registration modal and native **Choose a project folder** dialog were observed.
- SQLite initialized under Tauri's per-user local app-data location.
- Generated NSIS installer ran with `/S`, returned exit code 0 and installed the executable under the current user's local application data directory.
- Full GUI registration was not completed by the native automation helper: it returned stale screenshot/control references during folder selection. No end-to-end GUI success is claimed for that step. Native persistence/discovery tests and frontend service interaction tests pass independently.

The installer is unsigned. SmartScreen reputation, clean-machine prerequisite setup, upgrade/uninstall behavior, Explorer/clipboard interaction and extended manual accessibility testing remain additional distribution checks. No tag was created and no GitHub Release was published as part of this milestone PR. The tag release job is configured but has not been executed.

## CI and reproduction

See [PR #1](https://github.com/Flint221/context-dock/pull/1) and the [Windows installer workflow](https://github.com/Flint221/context-dock/actions/workflows/windows.yml) for live results and downloadable installer artifacts. CI runs the checks above and produces an installer independently on GitHub's Windows runner; a tag release additionally publishes the validated artifact. Consult the run for the intended commit before downloading.

The [README](../README.md) gives prerequisites, exact commands, installer download/install steps, and a manual registration/restart/missing-file/removal checklist. Tests create temporary data; no personal project files or databases are included in source control.
