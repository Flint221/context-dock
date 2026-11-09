# ContextDock app audit and rename acceptance

This revision supersedes the open rename and ChatGPT-readback findings in [audit v4](ContextDock_App_Audit_v4.md), which is retained unchanged. The 0.11.0 preview remains on draft [PR 16](https://github.com/Flint221/context-dock/pull/16), with schema 18. No production installer or real-profile migration was applied.

## Verified changes

Commit 9c99828 fixes unchanged numeric-version renames in the latest-file workflow. The matcher preserves the source and family identities when one selected, higher-numbered filename with the same stem/type/directory has verified identical bytes. New filename-version metadata is still recorded. Previously captured stale paths can be retired from the current-file UI without deleting their source/version metadata or changing cloud receipts. Ambiguous, changed, excluded and in-flight cases retain their existing safeguards. Reappearing paths become visible again; genuine missing latest files still warn.

The rebuilt native pilot automatically repaired its original v6 stale-path warning. A separate v1-to-v2 rename then preserved the exact source UUID, family UUID, Drive ID and content hash. Independent connected-Drive listing/readback confirmed one cloud file for that test family. The latest native UI showed three families and no missing-file banner. Original Markdown v8 and Word v4 files and receipts were unchanged, and recycling remained off.

The user also supplied a successful ChatGPT connected-Drive readback for v8. Version 8, marker CDPILOT-A6F3E619DB99426C-MD8 and phrase violet harbor matched all test evidence. This closes readback acceptance as a user-reported result; precise freshness timing was not recorded and no fixed interval is claimed.

See [rename repair evidence](ContextDock_Rename_Repair_v1.md) for matching boundaries, retained metadata, live IDs/hashes, build identity and the distinction between observed native/Drive results and user-reported ChatGPT results. [Pilot v2](ContextDock_Drive_Pilot_v2.md) preserves the earlier upload, simultaneous-version, move, pause and restart evidence.

## Checks

328 Rust tests passed with eight opt-in integrations ignored. All-target Clippy with warnings denied, TypeScript/production frontend build and isolated native release build passed. Frontend source was unchanged in this repair; the prior 198-test frontend suite remains its validation baseline. The code and documentation are committed and pushed on the draft branch.

## Remaining gates

- Correct Pending approvals confirmation wording: it currently labels the legacy cloud-sync flag off even while saved Drive publication is enabled. The live test followed the saved Drive policy, but the text can mislead users.
- Exercise actual live-Google outages, expired/revoked authorization and interrupted transfers. Pause/resume and saved credentials across restart have passed; they do not establish those failure cases.
- Verify the complete live-Google-to-Recycle-Bin chain on disposable files before enabling cleanup for real projects.
- Check actual battery-saver transitions and longer/large-project resource behavior. No battery-life improvement percentage is established.
- Complete current-head CI, production packaging and reviewed-profile upgrade/installation acceptance before merge/release. Preserve all prior audit limits and the forward-only schema-18 migration boundary.
