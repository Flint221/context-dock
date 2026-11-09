# ContextDock release and installed-app acceptance

ContextDock 0.11.0 is merged to main and installed locally. [PR 16](https://github.com/Flint221/context-dock/pull/16) merged as `61082766026b40cb68cbd86a8c8424cc3f1ab07e` on November 9, 2026. Local main was fast-forwarded and its application source matched tested head `4307a1a49e377ab96839481bc3832f84360af775`. [Audit v7](ContextDock_App_Audit_v7.md) and all earlier versions are retained.

## Completed acceptance

| Area | Verified behavior |
| --- | --- |
| Version tracking | Numeric version renames retain source/family identity and the existing Drive ID. Earlier live move, simultaneous-version and missing-row repair evidence remains in v5/v7 and the pilot reports. |
| Cloud replacement | Markdown and Word replacements update one cloud file per family. Native downloaded-byte hashes and independent connected-Drive readback matched the newest versions. |
| Recycling | Two unchanged disposable older files reached the actual Windows Recycle Bin with matching hashes and file identities. A changed older file and newest local files stayed in place. Recycling was then disabled. |
| Token refresh | Real Google refresh succeeded and persisted the renewed expiry. |
| Revoked authorization | Google rejected a genuinely revoked pilot refresh token. Publication recorded an authorization block and retained its existing cloud identity. The user completed reconnection in Chrome; queued v4 published automatically and the block cleared. |
| Physical network outage | A disposable v5 was created while native Windows WLAN status was disconnected. Local tracking captured it, the old v4 cloud receipt stayed intact and a connection failure persisted. After reconnection the app retried automatically, verified v5 and cleared the error under the original Drive ID. |
| Interrupted verification | An injected retryable interruption after an actual remote write recovered after reopening the registry. Separate wire-level coverage verifies truncated HTTP-body recovery. This is distinct from the physical Wi-Fi check. |
| Windows energy saver | The live check exposed the legacy battery-saver flag reporting off while Windows energy saver was on. The corrected detector held publication while local tracking continued, then automatically published the pending generation when energy saver turned off. Original power settings were restored. |

The power fix subscribes once to the documented [Windows energy-saver notification](https://learn.microsoft.com/en-us/windows/win32/power/power-setting-guids), retaining the older battery-saver flag as fallback. Unknown notifications do not invent a pause. Pending approvals also now display and bind the actual saved Drive policy rather than the legacy cloud flag.

The physical outage was captured on November 9 at 08:22:14–08:23:08 UTC; v5 verified at 08:23:37 UTC. This is one small-file observation, not a guaranteed reconnect or publication interval. Drive ID `1e4lquHTh1N3s6AVU-86lSzY57OHl9Fcf` remained unchanged. Native v5 SHA-256 was `05f63fba75bf0cea80e0592a04f17b7adfd34cc3537592a96dbe9dd117e01a19`; independent Drive readback matched version 5, marker `CDNET-4307A1A-V5` and phrase `blue summit`.

The first two temporary network monitors expired before the user performed the outage. The third captured it, but its completion check incorrectly treated a space formed from two empty error fields as an error. Review of its unchanged captured events verified both error fields independently and confirmed recovery; `ContextDock_Network_Acceptance_v4.json` records that correction and preserves the earlier reports. No outage-related application change or additional user disconnect was needed after that capture.

## Validation and installer

331 Rust library tests passed with nine opt-in integrations ignored. The 198 frontend tests, TypeScript, ESLint, formatting and all-target Clippy passed. Pilot-feature Clippy and the explicit live integrations passed separately. [Application CI](https://github.com/Flint221/context-dock/actions/runs/42574545802) passed through Windows installer/artifact creation before merge. This release record changes documentation only; it does not change the tested application source.

The installed version is 0.11.0. Preserved `ContextDock_Installer_v2.exe` is 4,422,766 bytes with SHA-256 `D5A80A9DB2AB74953AB806237F75A9CCF94BC9CF24112948C884D147E5FD7922`. Installation completed silently with exit code 0. Installed executable SHA-256 is `D11F21209CEA2D866D3D67D58DC859D6013F40EF571440535109E18358A47967`.

The initial executable-hash comparison correctly stopped before resetting data: the installed binary contains Tauri's `NSS` installer marker while the build output returns to `UNK` after packaging. In-memory normalization of that single marker established that all other bytes match the verified build. No installed executable was edited to perform this comparison.

## Fresh production state

The installed native UI showed version 0.11.0, zero projects and zero pending approvals. Read-only verification including WAL returned schema 18, integrity `ok`, no foreign-key errors, latest-only mode true and zero rows in projects, documents, source files, Drive projects/publications and recycling intents. Active snapshot/staging folders contain no files. Recycling and publication have no enabled project registrations.

The cutover retired both encountered old registry views and the old internal cache; it did not merge historical data. See [reset v3](ContextDock_Local_Reset_v3.md) for the actual backup paths and verification. Permanent removal was blocked by automatic approval review, so the cache is retained outside the active profile: no disk-space saving is claimed. Actual project directories and previous backups were not changed.

The existing OAuth client JSON was imported into the installed app through Settings. Its observed status is **Ready to connect**; production Google sign-in/root selection remains onboarding work. Pilot tokens were not copied into production. Existing Windows startup registration remains on. The installed app is left open on its empty Projects screen, ready for the four consolidated project folders.

## Practical limits and next use

The small pilot's 15.06-second resource sample covered seven native/WebView processes while Settings was open: about 200 MiB private committed memory and 0.52% of one CPU core. This is not a RAM limit, a controlled before/after result, a large-project benchmark or a battery-life improvement percentage. Resource behavior with the four actual projects remains to be observed after onboarding.

The earlier successful ChatGPT v8 readback was reported by the user and matched the local/Drive evidence. This release run did not measure ChatGPT indexing delay or establish a fixed refresh interval. Native cloud acceptance used disposable pilot projects; no real consolidated project was uploaded during the reset or installation.

Next: sign in to Google Drive in Settings and choose the existing ContextDock root; add the four consolidated project folders and save their selections; enable publication after choosing each destination. Keep recycling off initially. When enabled later, cleanup still requires a fresh verified replacement and unchanged older local bytes.
