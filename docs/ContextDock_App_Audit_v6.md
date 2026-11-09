# ContextDock app audit and release gates

This revision follows [audit v5](ContextDock_App_Audit_v5.md), which is retained. The 0.11.0 application remains on draft [PR 16](https://github.com/Flint221/context-dock/pull/16). Production is still installed at 0.10.15. No merge, installation or production database reset has been performed.

## Completed fixes and checks

The numeric-version rename repair and user-reported ChatGPT v8 readback remain accepted as recorded in v5. Pending approvals now show the actual saved Google Drive publication policy. Review tokens include the settings that affect publication, so a confirmation cannot silently use a policy changed after its preview.

Drive metadata responses are bounded to 1 MiB. Interrupted response bodies and incomplete JSON become retryable failures. A wire-level test verifies recovery through the same cloud file ID after the server accepts the replacement but its response is truncated. Other malformed metadata remains an actionable error.

Local validation passed: 330 Rust library tests, 198 frontend tests, TypeScript, ESLint, rustfmt and all-target Clippy with warnings denied. The pilot-feature test target also compiles and passes Clippy. Eight integrations were ignored by the default Rust suite; this result does not establish live recycling or actual outage acceptance.

The production Windows x64 NSIS installer was built and preserved as `ContextDock_Installer_v1.exe`, 4,424,985 bytes, SHA-256 `F316DBC87446D15D5074479191A611DF3C2BE985C7ED934524FD60C4F1BDD0AD`. A separate pilot v9 executable was prepared with the same production fixes. Neither executable has been installed or launched for final acceptance. Exact-head CI remains a merge gate.

## Required recycling acceptance before merge

The user requires working recycling before merging. The fresh production setup will still start with recycling off; users can enable it per project after reviewing the behavior.

An explicit Windows/pilot-only integration test now uses a fresh disposable registry and project folder. It must verify real Google token refresh, Markdown and Word replacement under preserved Drive IDs, recovery after an injected interruption following a real remote write, and actual Windows Recycle Bin receipts for unchanged older files. Changed older files and newest local files must stay in place. This is a prepared test, not a completed result.

The interruption test reopens the registry and retries against actual Google Drive; it does not simulate a literal process crash or physical network outage. Default test runs cannot invoke this integration. Its explicit opt-in requires the marked pilot profile and a new versioned fixture directory. Publication and recycling are disabled for the disposable project after success. Existing project folders and their preferences are not used for this test.

## Remaining work

- Run and inspect the complete live Google-to-Recycle-Bin integration before merging.
- Complete native acceptance for actual offline/reconnect, revoked authorization/reconnection and battery-saver transitions. Do not infer these results from token refresh, pause/resume or injected interruptions.
- Confirm current-head CI and installed-app behavior before completing the release. Resource measurements remain observations, not a battery-life improvement claim; longer and larger-project behavior has not been established.
- After acceptance, merge the reviewed branch, update local main and install the prepared production build with a fresh local registry. Re-add the four consolidated projects without migrating their old registrations.

The user is working on the computer. Background development and packaging are authorized; desktop interaction and final installation checks wait for an available handoff. The running pilot must close before the isolated live credential test so that two processes cannot race its credential state.

## Fresh-profile scope

Read-only inspection of the production profile found schema 10 with no active project registrations, plus 231 old internal snapshot files totaling 718,500,889 bytes. This cache is app-owned history, distinct from actual consolidated project documents. The authorized fresh start includes the active SQLite database/journals and obsolete internal snapshot/staging cache. Existing backups and user project directories stay outside the reset scope. See [reset plan v2](ContextDock_Local_Reset_v2.md); it records a plan, not an executed reset.
