# ContextDock app audit and live Drive acceptance

This revision preserves [audit v3](ContextDock_App_Audit_v3.md) as the implementation baseline and updates its live-acceptance gates. The 0.11.0 preview remains on draft [PR 16](https://github.com/Flint221/context-dock/pull/16), with schema 18. The installed app and real project data have not been upgraded.

## Current outcome

The app-owned Google connection now works with the user's Desktop OAuth client. Native consent, account verification, existing root selection and a pilot-only subfolder were exercised against Google. Markdown and Word publication, simultaneous numbered arrivals, one ordinary move, stable document/cloud IDs, exact downloaded-content verification, persisted pause, startup reconciliation and resume after restart passed. Independent connected-Drive reads confirmed the updated filenames and markers, with one file per family.

The active product remains focused on local monitoring, filename-version families, selection/exclusions, moves and optional latest-file publication to Drive. Embedded Ask AI, vector indexing and historical/archive interfaces remain retired from the active surface; existing saved material is preserved. ChatGPT remains a separate consumer of Drive files.

## Findings from the live pilot

| Finding | Status |
| --- | --- |
| Reopened Windows publisher storage failed guarded staging | Fixed and pushed in ef3d683; regression reproduced before fix, passed after, and native live publication succeeded. |
| Old retry message remained after successful automatic sync | Fixed and pushed in 41a1be9; persisted errors now follow live journal state. |
| Paused Drive panel missed local changes | Fixed in 41a1be9; registry changes now refresh publication state, with subscription cleanup tests. |
| Background startup did not request low WebView memory use | Previously tested fix committed and pushed as 8409929. |
| Numeric-version-changing rename leaves old path marked Missing | Still open. The correct family and cloud ID persist, but the project warning is confusing. Preserve genuine missing-file detection when correcting it. |

See [live pilot v2](ContextDock_Drive_Pilot_v2.md) for exact scenarios, hashes, timing, build identity and limitations. The final pilot displayed two synced documents; local recycling remained off. Small-file automatic verification took about 13–17 seconds in one simultaneous-arrival sample, including a safe consistency retry. This does not establish a fixed sync interval or ChatGPT freshness guarantee.

## Remaining gates

1. Confirm updated-file retrieval in a fresh ChatGPT web conversation and record an observed upper-bound delay separately from Drive API/connector visibility.
2. Resolve the numeric-rename Missing warning, then verify ordinary moves, filename-version changes and genuinely missing latest files together.
3. Exercise actual live-Google outage/revoked-or-expired authorization and interrupted-transfer recovery. Saved credentials and paused state survived native restarts, but that does not establish these failure cases.
4. Test opt-in recycling through the complete live Google verification chain using disposable files only. Keep real projects opted out.
5. Check actual battery-saver transitions, longer battery/resource behavior and large-project queues. Existing resource observations remain limited as described in audit v3; no battery-life percentage is claimed.
6. Complete reviewed-profile migration, new-head CI and production packaging/installation acceptance before merge/release. Older binaries cannot open schema 18; do not treat a local pilot build as rollback or release acceptance.

## Checks

323 Rust tests passed, eight opt-in integrations remained ignored, and all-target Clippy passed with warnings denied. After UI follow-up, 198 frontend tests, ESLint, TypeScript/production build and isolated native release build passed. The latest code is pushed; the branch remains a draft. Neither the real profile nor source files were deleted, recycled or migrated during this pilot.
