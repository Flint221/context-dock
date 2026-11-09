# ContextDock version-rename repair

This closes the numeric-version rename finding in [audit v4](ContextDock_App_Audit_v4.md) and records the user's subsequent ChatGPT readback. Code commit: 9c99828. Tests and native acceptance used the isolated pilot profile; the installed app and real projects were not migrated.

## Corrected behavior

Previously, the relocation matcher required the incoming filename to have the same author-version key. An unchanged file renamed from v8 to v9 therefore joined the correct family as a new numbered export but left its former source path marked Missing.

In the latest-file workflow, one unambiguous higher numeric version with the same normalized stem, directory, type and verified bytes can now relocate the missing active source while keeping its source UUID. The new filename still creates the appropriate version metadata even when the file timestamp and content hash are unchanged. A filename decrease retains the existing review requirement.

Already-captured stale paths are repaired separately. An absent older path is marked with a retained renamed disposition only when there is exactly one selected, available, higher-version successor in the same established family with matching stored and freshly read SHA-256. Reciprocal ambiguity, exclusions, pending family reviews, unfinished journal work, pending Drive generations and recycle references prevent retirement. The repair does not change the active source, version metadata, cloud ID or source bytes. Missing renamed records leave the current-file list, folder tree and missing-file warning; they remain retained in the database. If an old path reappears, its existing record becomes visible and is checked normally again.

Historical repair infers a logical relocation from absence, family/version and uniquely matching content; it does not prove a past operating-system rename event. An identical higher-version copy followed by removal of the older path can meet the same conditions. Changed or ambiguous cases remain visible for review. A truly missing latest file without a verified successor remains Missing.

## Automated checks

Five new regression tests cover:

- Forward numeric renames, preserved family/version metadata, and a subsequently missing latest file.
- Previously captured stale records and the old path reappearing with its original source/family identity.
- Changed bytes, ambiguous successors and excluded paths retaining missing status.
- An interrupted Drive publication deferring retirement until the pending intent resolves.
- A numeric rename keeping one cloud file and the same document/Drive IDs.

The complete Rust suite passed 328 tests with eight opt-in integrations ignored. All-target Clippy passed with warnings denied. TypeScript/production frontend build and the isolated native release build passed; frontend source was unchanged in this repair, with its prior 198-test suite retained as the frontend baseline.

## Actual native and live Drive acceptance

Pilot v8, built from 9c99828, automatically retired the existing Moved/ContextDock_Context_v6.md stale record on startup. Its source row and version metadata were retained, tracking_error cleared, and the Files & tracking view showed Available with no missing-file banner. The existing Markdown v8 and Word v4 retained their verified names, contents and Drive IDs.

A separate harmless ContextDock_Rename_v1.md fixture was approved in the native Pending approvals interface and published. Renaming it to ContextDock_Rename_v2.md produced the following verified result:

| Evidence | Before and after |
| --- | --- |
| Source UUID | `d05c3810-8a8f-4aa0-939c-d623a26d5f09` |
| Family UUID | `4119d0a4-165a-49a6-9aa4-d37bead29668` |
| Drive file ID | `1e4lquHTh1N3s6AVU-86lSzY57OHl9Fcf` |
| SHA-256 | `83fe031f81e899fe0412bbcfe59e7408b5ab4563d71c0d9460dfb6a963d26d3f` |

The rename occurred at 11:58:01 PM America/Chicago on November 8; exact remote verification completed at 11:58:14 PM. The native latest list showed three document families with no missing-file banner. A separate connected-Drive folder listing showed three cloud files total, one per family, with only v2 for the rename fixture. Connected-Drive content fetch confirmed its original test marker. Local read-only observations are preserved as ContextDock_Drive_Acceptance_v5 and v6 JSON artifacts, with WAL visibility.

Pilot executable SHA-256: `C709D97E6D8E2A34A8CE8E172094D3E79BF56B70D814EE33126A432D8CC6E7DE`. Local recycling stayed off. No source was recycled or permanently deleted, no production installer was applied, and no release was published.

## ChatGPT readback

The user supplied a successful ChatGPT connected-Drive response for ContextDock_Context_v8.md. Its reported fields exactly matched the local/native/Drive test evidence:

| Field | Value |
| --- | --- |
| Version | `8` |
| Marker | `CDPILOT-A6F3E619DB99426C-MD8` |
| Phrase | `violet harbor` |

This is user-reported ChatGPT web acceptance matched against independent Drive and local receipts, not an assistant-observed browser session. No precise readback delay was captured, so it does not establish a refresh interval or freshness guarantee. The v8 readback file was left unchanged during the separate rename test.

## Remaining work

Live outage/expired-or-revoked-authorization/interrupted-transfer acceptance, full live-Google-to-recycling verification, battery-saver/large-project checks, reviewed-profile migration and production packaging remain separate gates. A separate UI wording issue was observed: the Pending approvals confirmation still reports a legacy cloud-sync flag as off while Drive publication is enabled. Its actual saved Drive policy was respected; the confirmation wording needs correction before real-user upgrade acceptance. New branch-head CI must be assessed independently.
