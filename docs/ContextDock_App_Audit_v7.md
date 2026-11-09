# ContextDock live Drive and recycling acceptance

This revision closes the disposable recycling gate from [audit v6](ContextDock_App_Audit_v6.md), which is retained. The live test passed against source commit `c67c32e6ce4fd524f3c3971e9c9933d393e0c311` on November 9, 2026. No application source changes were needed during this acceptance run. [PR 16](https://github.com/Flint221/context-dock/pull/16) remains draft; no merge, production installation or database reset has occurred.

## Completed background acceptance

The existing pilot was closed before the test. The test used its isolated credential-vault target, a new disposable project/registry and a dedicated Google Drive subfolder. It completed in 33.35 seconds without desktop interaction.

| Check | Observed result |
| --- | --- |
| Expired access-token refresh | Real Google refresh succeeded and the refreshed expiry persisted. |
| Markdown publication | v1, then v2, then v3 used the same Drive file ID. Final downloaded bytes matched v3. |
| Word publication | v1 and v2 used the same Drive file ID. Final downloaded bytes matched v2. |
| Interrupted verification | A deliberate retryable interruption followed an actual remote write. Reopening the registry and retrying recovered the pending publication under the original cloud ID. |
| Changed older local file | Deliberately changed Markdown v1 was refused for recycling and remained in place with its changed contents. It was subsequently excluded from the disposable project's selection. |
| Unchanged older local files | Markdown v2 and Word v1 reached the actual Windows Recycle Bin. Both durable receipts were present, and separate read-only SHA-256 checks of the returned physical bin paths matched the tracked bytes. |
| Newest local files | Markdown v3 and Word v2 stayed in the disposable project directory. |
| Database/recovery state | Integrity check returned `ok`; both recycling intents were complete with no errors or unresolved recovery intents. |
| Final preferences | Publication, automatic publication and recycling are off in the disposable project. The original pilot's recycling preference remains off. |

This verifies the complete native Google publication-to-Windows-Recycle-Bin chain for these two disposable version families. The native Shell adapter also checked file identity before accepting each Recycle Bin receipt. It does not use a permanent-delete fallback.

## Independent Drive readback

The connected Google Drive plugin independently listed exactly two visible files in the dedicated [test folder](https://drive.google.com/drive/folders/11q4_d-56gqzesIQnNXjTyEbq7jPdEvZi): one file per document family. It fetched the final readable contents of both files successfully.

| Cloud file | Preserved Drive ID | Final readable contents |
| --- | --- | --- |
| `ContextDock_Context_v3.md` | `1GgPVgkY60-aZhnEUozngozPm_Icgf_pZ` | `disposable recovered Markdown` |
| `ContextDock_Notes_v2.docx` | `1oz2dAuj5gvOTAp-z4ZhUXjVOVEif96Zn` | `disposable second Word` |

Native verification downloaded the stored files and matched these SHA-256 values:

- Markdown v3: `06c8f0214dcc1d1da22fd8bdfec0002face005b240f97ffe1944915b966d2e51`.
- Word v2: `6e513f862550f74be9297a2a3c1c7f31b04b100558c9940147f261f9b477439a`.

The live fixture and report remain under `ContextDock_Pilot_v1/ContextDock_Live_Check_v1` in the existing local pilot artifact directory. `ContextDock_Recycle_Acceptance_v2.json` contains the independent read-only registry/settings/receipt/local-file checks. The test folder and its two harmless cloud files are retained as evidence. User project directories and the production registry were not modified.

## Remaining release gates

The successful test covers real token refresh and real remote writes with an injected verification interruption. It does not establish physical network-outage recovery, a genuinely revoked grant/reconnection or actual battery-saver transitions. Those native acceptance checks still require the final desktop handoff. The previously reported ChatGPT v8 readback remains separate evidence; this run used the connected Drive plugin and did not open a fresh ChatGPT web conversation or measure its indexing delay.

Current-head CI must pass before merging. Then update local main, execute the scoped [fresh-profile plan](ContextDock_Local_Reset_v2.md), install and verify production 0.11.0, and connect the four consolidated projects. Production recycling will start off even though the disposable integration has now passed. Larger-project resource behavior and battery-life gains remain unmeasured.
