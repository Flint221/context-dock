# ContextDock testing cleanup

Completed local cleanup on November 9, 2026 after the 0.11.0 merge and installation. The user requested removal of stray testing files and explicitly chose to keep older database/reset/investigation backups until the four consolidated projects are working.

## Removed from working locations

| Location or artifact | Result |
| --- | --- |
| Repository `src-tauri/target` | Verified Cargo's target directory, then ran `cargo clean`. Cargo reported 73,182 files and 100.4 GiB of build output removed. The installed application is outside this directory. |
| Pilot artifact directory | Pilot executables, disposable projects, raw reports/screenshots and duplicate installers were grouped and sent to Windows Recycle Bin. The managed workspace root remains, empty. Compact acceptance records remain in the repository. |
| Pilot/verification WebView profiles | The isolated `com.contextdock.pilot` and `com.contextdock.verification` cache directories were included in the recycled group. The production profile was preserved. |
| Named Temp leftovers | Four empty ContextDock test directories and the obsolete `contextdock-rustup.exe` download were recycled. The installed Rust toolchain remains available. |
| SQLite test fixtures | 8,468 temporary roots containing 4,246,777,048 bytes were grouped and recycled. All original fixture paths were absent afterward. These include accumulated test leftovers, not only the latest branch's acceptance artifacts. |

The fixture inventory required the ContextDock SQLite tables `projects`, `documents`, `source_files`, `cloud_connection` and `cloud_installation`, and checked that every registered project path was contained within its temporary root. Reparse points and foreign project paths were excluded; no such exceptions occurred among the confirmed roots. All inspection connections were closed before moving directories.

Free space on C: rose from 87,009,767,424 bytes before build cleanup to approximately 171.9 billion bytes after cleanup, about 84.9 GB recovered. This is an observed system-wide difference and can fluctuate with other activity. Cargo's logical file-size total is larger than the measured free-space gain. Recycled files remain recoverable and still occupy disk space until removed from Windows Recycle Bin.

Automatic approval review rejected permanent removal with `blocked by policy`, without a more specific reason. Cleanup therefore used reversible recycling. Existing disposable recycling-test items remain in the Bin; the whole Bin was not emptied. A separate attempted pilot credential deletion was also rejected, so removal of that isolated credential is not claimed. No production grant was revoked.

## Preventing repeat test leftovers

Ten Rust test fixtures declared their temporary-directory owner before their SQLite registry. Their fields now place the temporary directory last so database handles close before directory teardown on Windows. The regression `drive_fixture_drop_removes_database_and_temporary_directory` opens a real fixture database, drops the fixture and checks that its root is gone.

The test-only correction is committed as `51d239cda567753338c1e0acd980f0ecf18195f4`. Formatting and whitespace checks passed locally. [Windows CI](https://github.com/Flint221/context-dock/actions/runs/42590443500) provides the Rust regression/full-suite results. Production application code and the installed binary were not changed by this correction; rebuilding the large local target directory was avoided.

## Intentionally retained

- Installed ContextDock 0.11.0 at `C:\Users\Mihir\AppData\Local\ContextDock\contextdock.exe` and the active `com.contextdock.desktop` profile.
- `C:\Users\Mihir\Downloads\ContextDock_OAuth_Client_v1.json`.
- `C:\Users\Mihir\Downloads\ContextDock_Profile_Backup_v1` and the reset backup described in [reset v3](ContextDock_Local_Reset_v3.md).
- Earlier ContextDock backup, archive-acceptance, registry-reset and investigation directories, as requested. Their removal can be reconsidered after onboarding.
- The repository, tracked documentation, normal frontend dependencies/assets and desktop shortcut. Only one Git worktree is registered, on `main`.
- Actual local project folders and existing Google Drive content.

After cleanup the installed executable still matched SHA-256 `D11F21209CEA2D866D3D67D58DC859D6013F40EF571440535109E18358A47967`. The running production database remained schema 18, returned integrity `ok` and contained zero projects, documents, source files, Drive projects and Drive publications. The four projects have not been imported. Recycling remains off initially.

Follow [setup v3](ContextDock_Drive_Setup_v3.md) to connect production Drive and add the consolidated projects.
