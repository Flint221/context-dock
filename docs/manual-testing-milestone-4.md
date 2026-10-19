# Milestone 4 — installed Windows cloud-indexing acceptance

These are **user-run live tests**, not results claimed by the developer. Automated coverage uses a mocked provider and temporary local files only. Use disposable content, your own OpenAI API project/key, and an account where you can inspect and delete the resulting resources. Do not paste a key into Codex, ChatGPT, screenshots or this repository. API billing is separate from ChatGPT; small test files are not a guarantee of free usage.

## Upgrade and prepare

1. Close ContextDock 0.3.0. If keeping a backup, copy the entire closed `%LOCALAPPDATA%\com.contextdock.desktop` directory, including its database, journals and snapshot folders, to a separate safe folder. Do not copy only the database while the app is running.
2. Obtain the successful Milestone 4 branch/PR run's **ContextDock-Windows-x64-setup** artifact from [Windows installer Actions](https://github.com/Flint221/context-dock/actions/workflows/windows.yml). Extract its ZIP and run `ContextDock_0.4.0_x64-setup.exe` under the same Windows user and default per-user installation. The installer is unsigned; follow your organization's SmartScreen policy. No uninstall, data reset, tag or public release is necessary.
3. Launch from Start. Settings/About must show **0.4.0 · Milestone 4**. Existing projects, themes, histories, snapshots and local tracking/archival preferences must remain. Cloud indexing must initially be off. Schema 003 adds cloud records; do not launch 0.3.0 against the migrated database or overwrite it with an old backup while running.
4. In Documents create a fresh folder `ContextDock-Cloud-Test`. In Notepad choose **All files**, UTF-8, and create the following two files. Confirm they are not secretly `.md.txt`. Use this disposable folder, not your real projects.

**Master_Context_v1.md**

```markdown
# Cloud Test Project

## Project Goal
Build a simple inventory management application.

## Features
- Add inventory items
- View inventory items

## Status
Planning
```

**Technical_Documentation_v1.md**

```markdown
# Technical Documentation

## Architecture
- React frontend
- Rust backend
- SQLite database

## Storage
All data is stored locally.
```

5. **Add project** → name `ContextDock Cloud Test` → Browse to that folder → Add project. In Document tracking select both initial files and enable automatic local tracking; leave **automatic archival off** for these tests. Both logical families must have captured v1 with View history available before any cloud consent. If necessary use Refresh documents and wait for local capture. Do not enable cloud until both v1 snapshots exist.
6. Keep the correct OpenAI organization/project selected in your browser. The vector-store dashboard is under **Storage → Vector stores**; use its index ID rather than just a display name. Files uploaded by ContextDock have opaque `cd_… .md`/`.docx` names. Map them with View Cloud Index's File ID, version UUID and family name, or the association attributes (`contextdock`, `project`, `document`, `version`, `version_number`, `sha256`, `is_current`). There is no local absolute path in the remote filename/attributes. File **contents** can themselves contain sensitive text and DOCX embeds/metadata: the whole original snapshot is sent.

## 1. OpenAI connection

**Prepare/files:** Keep the two captured samples unchanged. In your own [OpenAI API keys page](https://platform.openai.com/api-keys), create a disposable key with Files and Vector Stores read/write access in the intended API project. Have billing/quotas available as required by your account.

**Click:** Settings → OpenAI connection → enter the key in the password field → **Save / replace key** → **Test connection**. Close/reopen ContextDock, return to Settings and confirm a saved key is indicated without its value being revealed. Windows Credential Manager → Windows Credentials → Generic Credentials should contain `ContextDock.OpenAI`.

**Operation/result:** Only an authenticated `GET /v1/vector_stores?limit=1` is needed for Test. Expect `connected`, a cleared password field, and no new remote index/File. Listing success does not prove upload permissions or available billing.

**Failure/recovery:** A useful key/permission/network error, not secret text, should appear. Correct the key or API project/permissions and Test again. No local source, history or tracking setting should change. Never share the key to troubleshoot.

## 2. Create the project's dedicated index without uploading

**Prepare/files:** Return to the disposable project with two captured v1 families.

**Click:** Cloud indexing → select both eligible families → keep **Current version only** → check **Automatically sync newly captured versions** → Review cloud changes. Expect two snapshots, their combined raw size, additions, zero removals and proposed store name. Read the retention/cost disclosure, check consent and **Confirm cloud configuration**. This saves the plan but does not start initial uploads. Click **Resume cloud sync** to create only the index before initial sync.

**Operation/result:** One `POST /v1/vector_stores`, with installation/project/operation ownership metadata. Expect a persisted `vs_…` ID and ready state. In OpenAI Storage → Vector stores verify that exact ID/name exists, initially without attached files. Confirm another project did not acquire this store.

**Failure/recovery:** No ID, multiple newly created stores, or uploaded files before Start indicates failure. Use **Retry / Reconcile / Verify key** for an interrupted request; it lists owned operations before creating anything. Do not repeatedly approve a new index. Only if the previous request has ended and the correct dashboard/listing confirms no copy exists, use **Retry confirmed absent index creation**, type `RETRY ABSENT OPERATION` and check the attestation. An existing matching copy must instead be recovered.

## 3. Initial upload and asynchronous indexing

**Prepare/files:** Verify both families remain selected. The two sources remain unchanged; local versions are still v1.

**Click:** **Start Initial Sync**. Open **View Cloud Index / Test Search** and inspect both rows. Refresh cloud status if needed.

**Operation/result:** For each snapshot: verified local SHA-256 → multipart `POST /v1/files` with `purpose=assistants` → attach its File ID to this project's vector store → poll the association. This purpose/header vocabulary is API compatibility terminology; no Assistants/Threads/Runs or model API is called. Both rows must eventually say **indexed**, with 2 indexed/0 pending/0 failed after settling. An uploaded or in-progress File must not count as indexed. OpenAI should show two completed associations and two uploaded Files. Raw byte totals and parsed/indexed usage may differ.

**Failure/recovery:** A tiny local scan normally takes seconds, but remote processing may take seconds to minutes; there is no guaranteed cloud completion time. Processing polls about every 15 seconds. Persistent errors need Retry/Reconcile, permission/billing checks or a smaller/supported file. Local controls and navigation should remain usable. Do not equate a successful upload with successful processing or guaranteed free use.

## 4. Retrieval verification

**Prepare/files:** Wait for the initial two indexed rows, then open View Cloud Index / Test Search.

**Click:** Enter exactly `What technology does the project use?` → **Search cloud index**.

**Operation/result:** `POST /v1/vector_stores/{id}/search` with project/installation filters and current-version filtering. Expect an excerpt from Technical Documentation mentioning React, Rust and SQLite, with its real File ID, family, captured v1, version UUID and provider score when supplied. This is retrieval, not an AI-generated answer; ranking/exact excerpt wording is provider-dependent. Only returned identity/provenance/score metadata is persisted, not the query or full excerpt.

**Failure/recovery:** Another project's text, invented source identity or a generated answer is a failure. An empty result alone can be provider ranking/processing delay: wait, Reconcile and retry `React Rust SQLite`. A provenance mismatch must be refused, not silently attributed. Compare returned IDs to the dashboard.

## 5. Capture and automatically index a new version

**Prepare/files:** Keep the filename **Master_Context_v1.md**. Add this exact line beneath `- View inventory items` and save UTF-8:

```markdown
- Export inventory to CSV
```

**Click:** Wait for local tracking or Refresh documents; Master Context history must show captured **v2**. With the separate automatic cloud sync option still enabled, wait for its cloud row to index. Search `Export inventory to CSV` after completion.

**Operation/result:** One newly committed snapshot upload/attachment; local Technical Documentation remains v1. Remote Master Context v2 replaces v1 in current-only mode **after v2 is completed**. Both source filename and local v1 snapshot remain intact. A `_v1.md` filename can correctly hold captured v2, v3, etc.; filenames do not determine capture sequence.

**Failure/recovery:** No local v2 is a local tracking/save issue; resolve it before cloud sync. A cloud failure must retain the old indexed association. Retry/Reconcile uses the same uploaded File after failed processing and retries only its failed association; it does not create another snapshot or duplicate successful File.

## 6. Current-only policy and replacement safety

**Prepare/files:** Let Test 5 settle with Master Context v2 and Technical Documentation v1.

**Click:** Refresh cloud status; inspect View Cloud Index and the same remote vector-store ID. Search `Export inventory to CSV` again.

**Operation/result:** Exactly **two active completed associations**: Master Context v2 and Technical Documentation v1. Historical Master Context v1 should be `removed` locally as an association, with its uploaded File **retained** remotely. Local Master Context still has both versions. There may be a brief overlap during safe replacement; assess counts after completion.

**Failure/recovery:** Old association removed before the new one completed, missing local history, or permanently duplicated active versions indicates failure. Reconcile to compare actual remote state. Do not delete retained Files merely to make association counts match; these are different resources.

## 7. Full captured history and explicit policy change

**Prepare/files:** No edits. Record current association/File IDs.

**Click:** Version policy → **Full captured history** → Review cloud changes → verify one addition (Master Context v1) → consent → Confirm. After settling, inspect the same vector store. Optionally switch back to Current version only using another preview/confirmation; expect one removal, then return to Full history for Test 8.

**Operation/result:** Three completed associations: Master Context v1/v2 and Technical Documentation v1. A retained uploaded File can be reattached without uploading its bytes again. Each association has family/version/hash/current metadata; only the real current version is marked current. The dashboard name is not the identity boundary.

**Failure/recovery:** Silent history uploads before confirmation, foreign families or duplicate mappings indicate failure. If a capture occurs while the confirmation is open, a stale-preview rejection is expected: Cancel and Review again. Reconcile discrepancies; local history is never changed by policy changes.

## 8. Pause cloud synchronization while local tracking continues

**Prepare/files:** Click **Pause cloud sync** and wait for any already in-flight request to end. Record remote IDs/counts. Append `- Scan item barcodes` under Features in Master_Context_v1.md and save.

**Click:** Refresh documents if needed; open local View history and Compare Versions. Do not click Sync Now or Resume yet.

**Operation/result:** Local capture advances to Master Context **v3** and remains comparable offline. No subsequent background cloud request/upload should begin while paused; the remote index remains at Test 7's state. An already in-flight request may finish, so record counts after it settles.

**Failure/recovery:** Frozen local tracking or new remote activity after pause settled is a failure. Keep cloud paused, inspect activity and verify no second running installation/other tool is editing the same remote store.

## 9. Resume and restart reconciliation

**Prepare/files:** Keep the Test 8 edit; cloud still lacks v3.

**Click:** **Resume cloud sync**. Wait for v3 indexing, close/reopen the application, reopen this project and use **Retry / Reconcile / Verify key** if state needs attention.

**Operation/result:** Durable pending work resumes with the same project store and existing File mappings. Full history now has Master Context v1/v2/v3 and Technical Documentation v1 (four associations). Startup reconciles active projects; it does not blindly recreate stores or reupload completed snapshots.

**Failure/recovery:** Duplicate stores/Files or lost selection on restart indicates failure. For an interrupted upload, Reconcile looks up its unique opaque filename/size and verifies downloaded SHA-256. If no copy is found, stop/check the correct dashboard and only then use **Retry confirmed missing upload** with its explicit absence confirmation. Never reset uncertainty while an earlier request is still running.

## 10. Offline local behavior and recovery

**Prepare/files:** Temporarily disconnect internet (leave local tracking enabled). Append `- Export inventory to JSON` to Master_Context_v1.md and save.

**Click:** Navigate Dashboard → Projects → project, Refresh documents, View history, Compare captured versions and Settings. Expect local Master Context **v4**. Reconnect internet; use Retry/Reconcile if automatic bounded retries have stopped.

**Operation/result:** Local registration, history, snapshots and diffs work. Cloud shows a network/retry/uncertain error and retains work instead of claiming indexed success. After reconnection it reconciles and eventually indexes v4 in the same store (five associations in full history).

**Failure/recovery:** Local data loss, global frozen controls, or Indexed while offline with no completed remote copy is a failure. Network failures before transmission can retry with backoff; uncertain mutations first reconcile. Restore connectivity, test the saved key if connection status failed, then Retry/Reconcile. No guaranteed completion time is promised.

## 11. Duplicate prevention and usage

**Prepare/files:** Wait until Test 10 settles. Record store ID, association count, uploaded File IDs and local history counts.

**Click:** Click **Sync Now** three times sequentially, allowing each request to unlock. Do not edit any files. Open Settings → Cloud usage and the linked OpenAI usage/billing pages.

**Operation/result:** Read reconciliation is permitted; no new File upload/store creation is needed. Counts and IDs remain stable. The UI shows local raw bytes separately from last measured OpenAI index bytes. OpenAI's account usage/billing is authoritative; the app does not estimate dollars or enforce an account-wide spending cap. A per-file soft upload limit is configurable via reviewed project configuration.

**Failure/recovery:** Growing identical File/version mappings, fabricated billing totals or a claim that uploads are always free is a failure. Pause cloud sync, record IDs and use Reconcile before cleanup. Consult [current pricing](https://developers.openai.com/api/docs/pricing), [usage](https://platform.openai.com/usage) and [billing](https://platform.openai.com/settings/organization/billing/overview); account allowances can be shared and prices can change.

## 12. Project isolation

**Prepare/files:** Create `Documents\ContextDock-Cloud-Test-B` containing only `Other_Project.md`, UTF-8, with:

```markdown
# Other Project
Build a bicycle repair appointment scheduler.
Unique marker: bicycle-scheduler-only.
```

**Click:** Register as `ContextDock Cloud Test B`, enable local tracking for that one file, separately select/consent to cloud indexing and Start Initial Sync. Record its **different** `vs_…` ID. Test Search there for `bicycle-scheduler-only`; inspect both dashboards by ID.

**Operation/result:** A second explicitly owned store contains only B's selected snapshot; A retains only A's selected families. Search provenance must belong to the chosen project. Separate opt-in must be required even with an already connected global key.

**Failure/recovery:** Reused store IDs or cross-project returned content/IDs is a failure. Pause both, preserve evidence and inspect ownership metadata. Do not delete a foreign/unrecognized association through ContextDock; the app must refuse destructive ownership mismatches.

## 13. Safe authentication and permission failure

**Prepare/files:** Pause both projects. Keep your valid key privately available for re-entry. Use only the disposable API key/account: revoke the test key in OpenAI, then click Settings → Test connection. Alternatively save a nonfunctional test string of at least 20 printable characters and Test; this should fail authentication without creating resources.

**Click:** Verify useful error feedback and local history/diff access. Replace with your own valid key, Save and Test, return to each project and click **Retry / Reconcile / Verify key** before resuming it.

**Operation/result:** Invalid/revoked keys fail a read-only request. No key value should appear in UI messages, activity, files or browser storage. A replacement key cannot silently reuse a mapping until the remote store's installation/project/operation ownership is verified. A key from a different OpenAI API project may legitimately have no access; it must not create a replacement index automatically.

**Failure/recovery:** Leaked key text, silent resource deletion or local data loss is a failure. Use the correct API project's permissions and credential. Rate/quota errors may require billing changes; Retry-After/backoff is bounded and terminal failures require an explicit retry. If credentials cannot access an old store, recover access in OpenAI rather than deleting unrelated data.

## 14. Explicit remote cleanup with local history preserved

**Prepare/files:** Reconnect/test the authorized key and verify each disposable index. Wait for active uploads, then turn automatic cloud sync off via reviewed configuration to avoid capturing additional test edits during cleanup. Do not unregister the project before remote cleanup; if already removed, re-register the same folder to recover its identity and mappings.

**Click:** In A's **View Cloud Index**, choose one known owned version → **Remove association** → Confirm. It should become removed/excluded, but its uploaded File should remain. Click that removed row's **Delete uploaded File**; read the separate warning, verify no other tools/threads/batches/resources reference it, check the attestation, type `DELETE file-…` using the exact shown ID, and confirm. For all other versions, return to the panel → **Delete project cloud index**, type `DELETE vs_…`, confirm and wait for status **deleted**. Reopen View Cloud Index to delete each remaining retained uploaded File separately using the same File-ID confirmation. Repeat for B.

**Operation/result:** Association DELETE is distinct from vector-store DELETE and File DELETE. ContextDock verifies store metadata/all known associations before deleting an index. It checks every accessible store for File references, verifies the owned filename/size and downloaded SHA-256, and requires user confirmation about references that the API cannot enumerate globally. Foreign references, insufficient list permissions or mismatched bytes block File deletion. Local sources, v1–v4 history, snapshots, optional archives and comparisons remain intact. Confirm the two disposable stores and their explicitly purged Files are gone in the correct OpenAI project. Provider dashboards/search/deletion views may be eventually consistent; storage/billing changes may lag and retention follows OpenAI's policy, not an immediate wipe guarantee.

**Failure/recovery:** Local deletion, deletion without confirmation, or silently retaining a supposedly deleted resource is failure. A partial delete must retain IDs for retry. Refresh/reconcile once remote state settles; a missing owned store can be acknowledged by the explicit delete control, then **Prepare a new index** requires a new consent review if you intend to recreate it. File ownership/reference checks cannot be bypassed merely to make cleanup green. Inspect/delete genuinely orphaned disposable resources in OpenAI yourself only after confirming their IDs and ownership.

**Finish:** Settings → **Disconnect OpenAI** removes the Credential Manager key, not remote objects. Revoke the disposable key at OpenAI. Removing the disposable registrations or closing/uninstalling ContextDock does not delete local source folders, snapshots, history or remote data. Manually remove only the disposable source folders after retaining anything you want to inspect. Do not clear the user's real application data.

## Acceptance checklist

- [ ] Upgrade retained existing registrations, themes, tracking/archival preferences and byte-exact history; cloud started off.
- [ ] Key was entered only in the installed app, stored in Windows Credential Manager and never revealed in UI/SQLite/browser storage/logs.
- [ ] Connection Test created/uploaded nothing; separate per-project selection and consent were required.
- [ ] Each project has its own persisted owned store ID; only tracked selected snapshots were sent.
- [ ] Both initial samples became Indexed only after provider processing completed; retrieval returned real provenance.
- [ ] Captured versions advanced independently of filenames; current-only kept old indexed content until replacement completed.
- [ ] Full history/policy changes used explicit previews; pause/offline use did not block local history/diffs.
- [ ] Resume/restart/repeated Sync did not duplicate successful resources; uncertain operations reconciled before retry.
- [ ] Missing/foreign resources and wrong/revoked credentials produced actionable errors without unsafe recreation/deletion.
- [ ] Raw vs indexed bytes, potential costs and authoritative account usage/billing were clear.
- [ ] Association, store and File deletion were separately understood/confirmed; local data survived cleanup.
- [ ] Both test projects' remote resources were cleaned up, disposable key revoked, and remaining live/API/upgrade observations recorded accurately.

Official references: [Retrieval](https://developers.openai.com/api/docs/guides/retrieval), [File upload](https://developers.openai.com/api/reference/resources/files/methods/create), [direct search](https://developers.openai.com/api/reference/resources/vector_stores/methods/search), [association deletion](https://developers.openai.com/api/reference/resources/vector_stores/subresources/files/methods/delete), [File deletion](https://developers.openai.com/api/reference/resources/files/methods/delete), [data controls](https://developers.openai.com/api/docs/guides/your-data). Consult the current API docs and account policy if provider behavior changes.
