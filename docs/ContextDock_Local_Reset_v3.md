# ContextDock fresh production profile record

Completed November 9, 2026 after the live acceptance checks and merge of PR 16. [Reset v2](ContextDock_Local_Reset_v2.md) is the retained plan; [v1](ContextDock_Local_Reset_v1.md) describes the separate October operation.

## Installation and retirement

The 0.11.0 production installer completed in current-user silent mode with exit code 0. Its installed file/product version and executable bytes were verified before retiring application data. The only difference from the build executable was Tauri's three-byte NSIS bundle marker; see [audit v8](ContextDock_App_Audit_v8.md). No old app process remained running during cutover.

The active profile is `C:\Users\Mihir\AppData\Local\com.contextdock.desktop`. The first registry view contained a schema-10 database with no active projects. Its database, WAL and SHM, plus the preceding application files, were copied and hash-verified while holding the session lock. The original registry files and internal cache were then moved into the new backup rather than permanently deleted. Automatic approval review had rejected the permanent-removal command with `blocked by policy`; no further reason was supplied.

That retirement exposed a separate older desktop registry: schema 13, four projects, 140 document rows and 139 source-file rows. It was preserved through File Explorer as a complete database/WAL/SHM group in `C:\Users\Mihir\Downloads\ContextDock_Profile_Backup_v1`. All three files disappeared from the active location before launch, and their destination hashes matched the observed source hashes:

| File | SHA-256 |
| --- | --- |
| `contextdock.db` | `600d7bc2bb69e31a7abf015b2b704077866077a547b0c6b156a8ecde03a204f5` |
| `contextdock.db-wal` | `fb94b90e642cd96dca4febe6589c35e3d3b6f557fa02ae52f187df830c980b6e` |
| `contextdock.db-shm` | `dae0a6060f919538e2e50329dae42fb3b2c03c0df13cd29999c166f54781894b` |

The earlier view's physical backup is `C:\Users\Mihir\AppData\Local\Packages\OpenAI.Codex_2p2nqsd0c76g0\LocalCache\Local\ContextDock_Reset_Backup_v1`. It contains the verified small registry/application copies, retired registry originals and 231 retained cache files totaling 718,500,889 bytes under `Retired_Cache`. Existing historical backup folders were not altered. Retiring the cache does not free disk space; it keeps obsolete payloads outside the active app profile.

These are byte-preservation records, not a claim that historical metadata or WAL has been repaired or that every old history is recoverable. No data was merged. No consolidated project file, external archive folder or Google Drive file was removed or moved by the reset.

## Observed fresh startup

The installed production app created a new registry. Read-only SQLite inspection including WAL confirmed:

- Schema 18; integrity check `ok`; no foreign-key errors.
- `workflow.latest_only=true`.
- Zero projects, documents, source files, Drive projects, Drive publications and recycling intents.
- No files in active snapshot/staging folders.

The native UI independently displayed **ContextDock 0.11.0**, **Projects 0** and **Pending approvals 0**. This installed-app observation is the final fresh-start acceptance, rather than relying on the initially visible older registry view.

The production OAuth client was imported from the user's existing Downloads JSON; the UI confirmed **Ready to connect**. No pilot session tokens were copied. Google sign-in and root selection remain user onboarding steps. Existing Windows startup registration is still on; no OS permission or security setting was changed. The app is open on Projects for adding the four consolidated folders. Recycling remains off initially.

Local evidence artifacts include `ContextDock_Installation_v1.json`, `ContextDock_Reset_v1.json` and `ContextDock_Network_Acceptance_v4.json` in the existing pilot artifact directory. The production database was verified directly, and the preserved older registry's three hashes were checked after its move.
