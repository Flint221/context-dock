# ContextDock local reset v1

October 22, 2026 (America/Chicago). The user explicitly authorized a fresh local database after verified ContextDock cloud cleanup, requiring actual project files and backups to remain safe. This operation is separate from the installer; installing 0.8.1 does not automatically reset databases for other users.

## Reset and preservation

No ContextDock process was running. An exclusive handle on runtime.lock prevented competing app startup during the reset. A fresh complete copy of the original database/WAL/SHM, snapshots and staging was created under `%LOCALAPPDATA%\ContextDock_Local_Backup_v1\Original_Copy`. All 60 files matched the source SHA-256 manifest before changing active registry files.

The exact three original registry files were moved, without modification, into that backup's Retired_Registry folder. Original_Copy retains a second copy. All 57 snapshots (21,585,474 bytes) remain in the application's snapshots directory as well as the verified backup. Staging is retained. The earlier backup and investigation evidence were not deleted or replaced. Forty known source/archive files were rehashed before and after and remain identical. No project folder, actual document or archive was moved or deleted.

The fresh database was built in a staging folder from the repository's migrations 001 through 010, with the same installation-identity initialization as Registry::open. The construction used a transaction, foreign-key enforcement, WAL mode and FULL synchronization. After checkpoint/close it was installed at the exact app-owned contextdock.db path. The reset procedure included rollback to the retired originals if activation or post-verification failed; no rollback was needed.

Post-reset readback confirms schema 10, integrity_check ok, no foreign-key errors, 29 tables, zero projects, zero documents and zero cloud jobs. Only cloud_installation and cloud_connection contain their single initialization rows. The global connection is disconnected, with a new installation identity and no saved cloud resource bindings. The Windows credential vault was not modified. No GUI or application launch, installation, upload, cloud deletion or index creation was performed in this reset turn.

The fresh registry intentionally does not display old projects/chats/history. Their surviving evidence and snapshot bytes are retained; this is not a claim to have repaired missing metadata in the malformed database. Re-added projects begin fresh histories and cloud associations under the new identity. Backups should remain available for any later history recovery.

## Installer and checks

Release versions in package/lockfile, Rust/Cargo lockfile and Tauri config are 0.8.1. The Settings version assertion was updated. The release distinguishes the unified folder-selection build from earlier 0.8.0 installers. The supported registry remains schema 010, application identity remains com.contextdock.desktop and page order is unchanged.

Fresh validation passed: 197 native tests, one intentionally ignored child-fixture entry, and 131 frontend tests across 15 files. TypeScript, ESLint, rustfmt, all-target Clippy with warnings denied, production frontend compilation and Windows x64 NSIS packaging pass. These are backend/component/build checks; no installed visual acceptance or real re-upload is claimed.

Unsigned installer: `src-tauri/target/release/bundle/nsis/ContextDock_0.8.1_x64-setup.exe`, 4,495,604 bytes. SHA-256: `AAEE6F98804B63871E5AFFFF497E33AE2798A833CCC36C5199022802E8F4F4F7`. File and product version are 0.8.1. The previous installer is retained.

Install 0.8.1 before reopening ContextDock because the previous installed reader predates schema 010. Re-add project folders, review and save the folder-tree selection, enable tracking/capture, test/reconnect the saved OpenAI key in Settings, then enable Cloud sync. Newly discovered unrelated files remain excluded until selected and saved. Confident family replacements follow selection subject to excluded parents and ambiguity checks. Local reset and source preservation were verified without sending new cloud traffic.

Detailed private reset/backup manifests are in ContextDock_Local_Backup_v1. Repository changes are on codex/new-file-selection; no main merge is claimed.
