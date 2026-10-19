# Milestone 2 — Windows manual testing walkthrough

Test ContextDock 0.2.0 with disposable Markdown files. Follow the tests in order so version counts match. Never substitute an important project for the test folder.

## Install and prepare

1. Close all ContextDock windows. From a successful [Milestone 2 Windows installer workflow run](https://github.com/Flint221/context-dock/actions/workflows/windows.yml), download **ContextDock-Windows-x64-setup**, extract the ZIP, and run `ContextDock_0.2.0_x64-setup.exe`. Use the same Windows account as 0.1.0. Do not delete application data to upgrade. This unsigned installer may show a SmartScreen/unknown-publisher warning; verify its origin and follow your Windows/organization policy.
2. Launch ContextDock. **Settings** must show **Version 0.2.0 · Milestone 2**. Existing project names and theme should survive; leave real projects' tracking/archival off during this exercise.
3. In Explorer enable **View → Show → File name extensions**. Create `Documents\ContextDock-Test`, using a path you have **never registered before**. If used before, choose `Documents\ContextDock-Test-2` or another unique name. Unregistering retains history, so reusing a path can restore earlier test versions.
4. Create only `Master_Context_v1.md` inside it. In Notepad select **Save as type: All files**, use the exact `.md` filename rather than `.md.txt`, and UTF-8 encoding. Paste:

```markdown
# Master Context
Goal: build a small local tool.
Status: first draft.
```

If Documents is redirected to OneDrive and tracking refuses a cloud/reparse-point folder, use a new fully local `%USERPROFILE%\ContextDock-Test` instead. The app refuses reparse-point traversal rather than guessing. Every filename below belongs in your chosen disposable root unless stated otherwise.

Optional PowerShell setup for the **initial file only**:

```powershell
$testFolderName = 'ContextDock-Test' # Choose a new name if previously registered.
$testRoot = Join-Path ([Environment]::GetFolderPath('MyDocuments')) $testFolderName
if (Test-Path -LiteralPath $testRoot) {
    throw 'Folder already exists. Choose a NEW disposable name; nothing was changed.'
}
New-Item -ItemType Directory -Path $testRoot -ErrorAction Stop | Out-Null
@'
# Master Context
Goal: build a small local tool.
Status: first draft.
'@ | Set-Content -LiteralPath (Join-Path $testRoot 'Master_Context_v1.md') -Encoding UTF8 -ErrorAction Stop
Write-Output "Created disposable project at $testRoot"
```

The script never selects a registered project and refuses to overwrite an existing folder. Never retarget it at a real project. For a cloud-placeholder error choose a fresh local root outside OneDrive and app data.

5. In ContextDock click **Add project**, name it **ContextDock Test**, use **Browse** to select this folder, and save. Confirm the file appears in discovery with tracking off and no captured versions.

**Inspecting results:** Use **Document tracking → Document families → View history**. Internal version numbers count distinct captured bytes, not filename suffixes. **Recent tracking activity** persists capture/review/failure events. **Pending operations** explains errors. Small files normally settle in a few seconds; allow about 10 seconds, then use **Scan for changes** if needed. Locked files retry with bounded delays; close the editor and scan again. A visible conflict is preferable to an unsafe move.

## Test 1 — Initial capture

**File/location/content:** `Master_Context_v1.md` in the disposable root, with the first-draft text above.

**Click:** **Enable tracking…**. Keep this file selected, leave **Also enable automatic archival** **unchecked**, then choose **Capture selected and enable**.

**Expected:** **Master Context**, **Current v1**, **1 version**. History shows an **initial tracking** capture and a present snapshot. Source stays unchanged in the root. No `Old Context Files` folder is needed yet.

**Failure:** Missing/corrupt snapshot, an initial number above v1 in a genuinely fresh project, source unexpectedly moved, or unrelated files automatically grouped. If old history was restored, use a new unregistered root instead of deleting saved history.

## Test 2 — Updated filename and explicit archival

**Click first:** Close history and check the separate **Enable automatic archival of replaced source files** checkbox. Merely enabling it must not move the initial document.

**Create in the root:** `Master_Context_v2.md`:

```markdown
# Master Context
Goal: build a small local tool.
Status: second draft with a clearer plan.
```

Save/close Notepad, then wait or click **Scan for changes**.

**Expected:** The same Master Context family becomes **Current v2**, **2 versions**. The new `_v2.md` stays active in the root. The old `_v1.md` moves into **Old Context Files** only after both versions are preserved and verified.

**Archive:** `Old Context Files\Master Context - <document UUID>\v1 - Master_Context_v1 - <operation UUID>.md` contains **Status: first draft.** UUIDs vary; original filenames remain in history metadata.

**Failure:** New source moved/deleted, old bytes lost, archive overwritten, an unrelated family merged, or duplicate versions from one stable save. A conflict must stay visible without forcing a destructive cleanup.

## Test 3 — Same filename overwrite

**Edit the existing active** `Master_Context_v2.md` in the root, replacing its contents with:

```markdown
# Master Context
Goal: build a small local tool with an offline workflow.
Status: third captured revision, same filename.
```

Save/close; wait or **Scan for changes**, then **View history**.

**Expected:** **Current v3**, **3 versions**, despite the filename still ending `_v2`. Current source stays in the root with third-revision text.

**Archive:** v1 remains; a new **v2** copy preserves **Status: second draft with a clearer plan.** Both earlier snapshots remain available.

**Failure:** Active source moved, second-draft bytes lost, earlier archives changed, or multiple new versions for identical bytes.

## Test 4 — Independent document

**Create in the root:** `Technical_Documentation_v1.md`:

```markdown
# Technical Documentation
Architecture: React interface and Rust backend.
Storage: local SQLite and immutable snapshots.
```

Save/close; wait or **Scan for changes**.

**Expected:** A separate **Technical Documentation** family starts at **Current v1**, **1 version**. Master Context stays v3 with three versions. New technical source remains in the root.

**Archive:** Existing Master Context v1/v2 archives stay unchanged. No archive is needed for the technical document's initial version.

**Failure:** Technical content joins Master Context, starts at v4, or moves Master Context's active file.

## Test 5 — Ambiguous matching and explicit confirmation

Pause first so setup does not accidentally create a high-confidence replacement.

1. Click **Disable tracking**.
2. Create these two files in the root:

`Review_Notes_v1.md`:

```markdown
# Review Notes
Workstream: interface review.
```

`Review_Notes_2026-10-01_v2.md`:

```markdown
# Review Notes
Workstream: backend review.
```

3. Click **Resume tracking**; scan if needed. Both same-stem incoming files should appear in **Needs Review**. For **each**, choose **New independent document**. Each starts its own **v1**, distinguished by its active filename. Neither initial source moves.
4. Now create `Review_Notes_v3.md` in the root:

```markdown
# Review Notes
Workstream: interface review, updated checklist.
```

5. Wait/scan. This new file must remain in **Needs Review** because two families could match. Confirm both previous sources are still present.
6. In its **Assign to document family** dropdown, select **Review Notes — Review_Notes_v1.md**, then choose **Assign as version**.

**Expected:** Only the chosen interface family becomes **v2**, with two versions and active `Review_Notes_v3.md`. Backend Review Notes stays **v1**, with its dated `_v2` filename. Master Context stays v3; Technical Documentation stays v1. The manual directory/name association is remembered.

**Archive:** Nothing moves while a match is pending. After confirmation, the chosen family's old `Review_Notes_v1.md` can move to its own v1 archive. The backend dated file stays in the root. UUIDs distinguish same-named family folders.

**Failure:** Any move before confirmation, wrong family updated, initial histories merged, or review notes mixed into Master Context. **Ignore for tracking** is also available for a pending file and must leave it untouched; selecting that alternative leaves its existing families at their previous counts.

## Test 6 — Application closed

Close ContextDock with its window X and let it exit. There is no tray service. Edit the active `Master_Context_v2.md` in the root:

```markdown
# Master Context
Goal: build a small local tool with an offline workflow.
Status: fourth revision, saved while ContextDock was closed.
```

Save/close Notepad. Relaunch ContextDock, open **ContextDock Test**, and inspect history. Scan if a folder was temporarily unavailable.

**Expected:** Startup reconciliation captures **Current v4**, **4 versions**, with a startup capture reason. Other histories retain independent counts. Current `_v2.md` stays in the root.

**Archive:** Existing v1/v2 remain; **v3** preserves **third captured revision, same filename.**

**Failure:** Missed change after startup/manual scan, duplicate v4/v5 entries for the same bytes, missing old snapshots, or moved current source.

## Test 7 — Duplicate prevention

**Files/content:** Do not modify anything.

**Click:** **Scan for changes** three times, letting each finish; optionally also **Refresh documents** and restart once. Inspect history.

**Expected:** Master Context stays **v4 / 4 versions**. Technical Documentation stays v1. Interface Review Notes stays v2; backend Review Notes stays v1. No extra archives appear.

**Failure:** Counts rise or archive copies multiply without byte changes. Adding/removing a newline changes actual bytes and is not this unchanged-content test.

## Test 8 — Archive safety and exclusion

**Files/content:** Make no active-source changes. Open the disposable **Old Context Files** in Explorer and inspect preserved files in Notepad without saving. In ContextDock inspect discovery/history and click **Scan for changes**.

**Expected:** Archives never become active discoveries or new families. Master Context stays **v4 / 4 versions**; current `_v2.md` remains in the root.

**Archive:** Master Context has **v1 first draft**, **v2 second draft**, and **v3 third revision**. The manually assigned interface Review Notes family has its own v1 archive. Any unrelated pre-existing files remain intact. Immutable snapshots are independent copies.

**Optional disposable-only check:** Edit one archive copy and scan. No new active version should appear. Its immutable snapshot must stay unchanged; **Archive location** should report a hash conflict when opening that edited copy. Do not edit the snapshot.

**Failure:** Archives recursively tracked, current source disappears, unrelated archive overwritten, or editing an archive changes its snapshot.

## Test 9 — Tracking disabled and resumed

Click **Disable tracking**. Confirm **Tracking off**, **Resume tracking**, and retained history. Edit active `Master_Context_v2.md` in the root:

```markdown
# Master Context
Goal: build a small local tool with an offline workflow.
Status: fifth revision, saved while tracking was paused.
```

Save/close; wait several seconds and click **Scan for changes** while tracking is off.

**Expected while off:** Metadata may refresh; Master Context stays **v4 / 4 versions**. No snapshot/archive is created; old v1–v3 archives remain.

Click **Resume tracking**, leaving the earlier archival setting enabled.

**Expected on resume:** **Current v5**, **5 versions**. The current source stays in place with fifth-revision text. A **v4** archive copy preserves **fourth revision, saved while ContextDock was closed.** Other histories are unchanged.

**Failure:** Capture while disabled, lost history, missed catch-up, duplicate v5, or moved current source. If archival is separately disabled, v5 should still capture while old snapshot bytes remain recoverable and sources are not moved.

## Test 10 — Snapshot recovery/inspection

**Files/content:** Make no changes. The original v1 must still contain:

```markdown
# Master Context
Goal: build a small local tool.
Status: first draft.
```

**Click:** Master Context → **View history** → v1 → **Version metadata**. Inspect the original source name, reason, capture time, size and SHA-256. Choose **Snapshot location**, then open that read-only file in Notepad without editing. **Copy snapshot path** should copy its location. The on-disk snapshot filename is a UUID; the original filename is retained in the UI.

**Expected:** Integrity is checked before Explorer opens; contents match the first draft. Current source still contains fifth-revision text. Master Context stays **Current v5 / 5 versions**. Archive contents remain as in Test 9. Inspection does not perform rollback or create new archives.

**Failure:** Old snapshot has current bytes, preserved metadata is missing, unexpected integrity failure, snapshot overwritten, or inspection changes source/count. Availability labels indicate file presence; opening the location performs actual hash verification.

## Final checklist

- [ ] Version 0.2.0 and upgraded registrations/preferences confirmed; real projects stayed opted out.
- [ ] Selected initial files became independent v1 histories without moves.
- [ ] Tracking and archival required separate opt-ins.
- [ ] Renamed exports kept a family; in-place edits left current sources in place.
- [ ] Unrelated documents kept independent sequences.
- [ ] Ambiguous matching waited for a decision without moving files.
- [ ] Restart/resume caught missed changes exactly once.
- [ ] Unchanged refreshes created no duplicate versions or archives.
- [ ] Old Context Files was excluded and preserved prior bytes.
- [ ] Immutable snapshots remained inspectable independently of archive copies.
- [ ] Disabling tracking retained history and stopped captures.
- [ ] Errors/pending operations were visible rather than causing overwrite/deletion.

## Safely remove the disposable test project

1. Close editors/history dialogs. Disable tracking for **ContextDock Test**.
2. Verify its registered path is the disposable root. Click **Remove**, then **Remove registration**. The actual folder, its archives, saved snapshot bytes and history metadata must remain.
3. If you no longer want its disposable source/archive files, manually delete **only that verified test folder** in Explorer after unregistration. Do not delete another project's files or the whole ContextDock app-data directory.
4. This does not purge stored snapshots/history. Registering the same path restores its retained identity/history with tracking off. Use a new path for another fresh v1 exercise; Milestone 2 has no automatic snapshot purge.

Report a failure with the app version, numbered test, expected/actual counts and exact activity/pending-operation error. Do not post real documents or your application database publicly.
