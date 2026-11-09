# ContextDock live Drive pilot

This continues [pilot v1](ContextDock_Drive_Pilot_v1.md) with actual app-owned Google authorization and publication. Tests used only the separate ContextDock Pilot identity, Windows credential target, marked Data directory and disposable Project folder. The installed application and real projects were not migrated. Local recycling stayed off throughout.

## Connection and destination

The user created a Desktop OAuth client, imported its JSON and completed Google's consent screens. The initial 403 test-user restriction was resolved by adding the sign-in account to the project's testing audience. The pilot then displayed Connected and passed Test Drive connection. Selecting the existing ContextDock folder through Google's native-app Picker saved the same root used in the earlier connected-tool experiment.

An independent connected-Drive listing showed only the earlier Drive API Test subfolder. The native app's Create Drive folders action then created one ContextDock Pilot subfolder under that existing root. Publication configuration, the destination and the credential grant persisted through two native-process restarts without another sign-in.

## Failure found and corrected

The first native publication stopped before transferring document bytes with “The registered root changed location or became a link.” The background publisher's registry opener had retained an ordinary Windows storage path while Registry::open used its canonical extended-length form. Guarded upload staging therefore rejected the private storage root, even though the project had not moved.

Commit ef3d683 canonicalizes the reopened storage directory, matching Registry::open without relaxing traversal, link or file guards. A new regression test reproduced the failure before the fix and passed afterward. It publishes, reopens the worker registry and replaces a numbered document under the same cloud ID. Retrying in the corrected native pilot reused the already reserved document IDs and existing folders.

## Observed live results

| Scenario | Observed result |
| --- | --- |
| Initial Markdown v4 and Word v2 publication | Both synced; app-downloaded SHA-256 receipts matched independently hashed local files. Connected Drive fetched the expected readable text and retained DOCX MIME type. |
| Markdown v5/v6 arrivals together, plus Word v3 | Automatic publication selected Markdown v6 and Word v3. The same two document families and cloud IDs were retained. Exactly two files appeared in the project folder. |
| Move Markdown v6 into the selected Moved subfolder | Its source UUID, family UUID and Drive ID persisted. The source became available at its new relative path. |
| Rename v6 to v7 while publication was paused | Local tracking selected v7 in the same family. Drive retained v6. The old numeric path remained as a missing metadata record; see the remaining issue below. |
| Add Markdown v8 and Word v4 while the pilot was stopped | Startup reconciliation selected both newest exports. Publication stayed disabled after restart, and Drive retained v6/v3. |
| Resume after restart | Automatic publication updated the original cloud IDs to v8/v4. Both final downloaded-content receipts matched the local SHA-256 hashes, pending intents cleared, and the UI showed synced without the old retry error. |

Drive project folder ID: `13InBo8CoS4jLtp5FAZsipOVEqvpgdrkZ`.

| Family | Stable Drive ID | Final name | Final SHA-256 |
| --- | --- | --- | --- |
| Markdown | `1TsqhzLgSGMZcbqYxpyk0oo9iNzsbmxOr` | ContextDock_Context_v8.md | `fd5e3d0baee850de81974f03385ceb4fabd348767611b032d7e0a5eef35b44c7` |
| Word | `1yBQ1YkvcyHjn-RPsAZK5CA4wPgTdIPkZ` | ContextDock_Notes_v4.docx | `c14d000929351984a3f7071756839fc08f04961a7f252ac3defc030edca717c9` |

Independent connected-Drive folder listings and content fetches confirmed both replacements and their new markers. These connector reads verify Drive visibility, not a fresh ChatGPT web conversation's retrieval behavior. The final Markdown marker is `CDPILOT-A6F3E619DB99426C-MD8`, Version 8, Phrase violet harbor; the Word marker ends in WORD4.

## Timing and UI follow-up

For the simultaneous exports saved at 11:39:13 PM America/Chicago on November 8, Markdown verification completed at 11:39:26 PM and Word at 11:39:29 PM: approximately 13 and 17 seconds including settling, transfer, verification and a provider consistency retry. This small-file observation is not a service guarantee or a fixed polling interval. Google changed a Word revision during verification; the app kept it queued, retried and verified under the same ID.

The failed manual attempt left a local error banner visible after automatic recovery. Commit 41a1be9 renders persisted failures from the live journal so resolved retries retire their messages, while retaining unsaved operation errors. The panel also subscribes to local registry changes so paused saves invalidate stale synced status. Tests cover recovery, retained actionable failures, paused local updates and subscription disposal. The final native UI showed both documents synced without the stale error banner.

The previously pending hidden-startup WebView memory fix was separately committed as 8409929. No new battery-life measurement was made during this cloud pilot.

## Validation and remaining acceptance

The Rust suite passed 323 tests with eight opt-in integrations ignored. All-target Clippy with warnings denied passed. The frontend suite passed 198 tests; ESLint, TypeScript, production frontend build and isolated native release build passed. Final pilot v7 was built from code commit 41a1be9; executable SHA-256: `7FB3F6C8150DD88E58B51B8A88BA70228A6A029821A962686AEEC8EE332D5030`. This is a development executable, not a new production installer or release.

Read-only SQLite probes used mode=ro with WAL visibility. Local ContextDock_Drive_Acceptance_v1 through v4 JSON observations preserve receipts, source identities and fixture hashes. The original six fixture files retained their contents; there were eleven physical fixture files at the end. No source was recycled or permanently deleted, and no second cloud file was created for either family.

Remaining issues and gates:

- A numeric-version-changing rename leaves the previous source path marked Missing, producing a project warning despite a valid current file and successful publication. Ordinary moves passed; this specific warning needs a separate tracking fix that preserves real missing-file detection.
- Fresh ChatGPT web readback and its observed freshness delay are pending. Do not substitute the successful connector fetches for that acceptance.
- Actual offline/provider outage, interrupted transfer, expired/revoked authorization and refresh after token expiry remain unproved against live Google. Pause/resume and credential persistence across restarts passed.
- Full live-Google-to-Recycle-Bin acceptance, actual battery-saver transitions during queued publication, large-project/resource stress, and upgrade acceptance on a reviewed copy of the real profile remain pending.
- New branch-head GitHub CI, production installer packaging and release acceptance are separate from these local checks. No merge, release tag or installed-app upgrade was performed.
