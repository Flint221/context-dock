# Milestone 3 manual testing — ContextDock 0.3.0

Use Windows 11 and a new disposable folder. These tests need no Rust or TypeScript tools. Local diffs are factual text comparisons, not AI summaries. Nothing is uploaded. Internal capture numbers are separate from filename suffixes.

## Install the build

1. Close ContextDock. Optional: with the app closed, back up `%LOCALAPPDATA%\com.contextdock.desktop` as a whole, including SQLite sidecars and snapshots.
2. Open the Milestone 3 PR's **Checks → Windows installer → Details**, open its successful run, and download **ContextDock-Windows-x64-setup** under **Artifacts**. Unzip it. Confirm the selected run is for the Milestone 3 commit, not an older build.
3. Run `ContextDock_0.3.0_x64-setup.exe` using the same Windows account and existing per-user installation. Do not uninstall or remove app data first. No new application identifier or database migration is introduced in 0.3.0.
4. Alternatively, use the locally built installer at `src-tauri\target\release\bundle\nsis\ContextDock_0.3.0_x64-setup.exe` in the repository.
5. Launch ContextDock and confirm **Settings → Version 0.3.0 · Milestone 3**. Existing projects, history and theme should remain. The installer is unsigned; Windows policy/SmartScreen may require your usual review. No automatic updater or public release is created.

Failure/recovery: if the old version still appears, close all ContextDock windows and install the intended artifact again. Do not reset the registry. Actual installation/upgrade and Microsoft Word behavior require this manual check; automated service tests do not establish them.

## Exact sample contents

Create `Documents\ContextDock-Diff-Test` in File Explorer. In Notepad, use **Save as type: All files**, UTF-8, and the exact filename `Master_Context.md` (enable Explorer's filename extensions so it is not `.md.txt`). Keep the same active filename for all three saves. Include the final newline after the last line and preserve blank lines. Wait for each capture before saving the next revision; do not save all three in rapid succession.

### Revision A — internal captured v1

```markdown
# Master Context

## Project Goal
Build a simple offline document manager.

## Features
- Register folders
- Discover Markdown files

## Status
Planning
```

### Revision B — internal captured v2

```markdown
# Master Context

## Project Goal
Build a local document manager with version history.

## Features
- Register folders
- Discover Markdown files
- Track document versions
- Archive old versions

## Status
Development
```

### Revision C — internal captured v3

```markdown
# Master Context

## Project Goal
Build a local-first document management application.

## Features
- Register folders
- Track document versions
- Archive old versions
- Compare historical documents

## Architecture
- React frontend
- Rust backend
- SQLite storage

## Status
Development
```

If you accidentally capture intermediate edits, use the actual capture dates and source metadata to identify A/B/C instead of assuming numbers 1/2/3. Every comparison is ordered by internal version number, irrespective of the order you choose in the selectors. A filename such as `_v2.md` may legitimately have internal captures v3 or v4.

## Test 1 — Initial setup and history

1. Preparation/content: save Revision A only in the new test folder.
2. Click **Add project**, name it **ContextDock Diff Test**, **Browse** to the new folder, then **Add project**. Open its detail screen.
3. Click **Enable tracking…**, select `Master_Context.md`, leave initial archival off, and click **Capture selected and enable**. Wait for the Master Context family to show one captured version.
4. Click **View history**. Expect v1, original filename, timestamp, hash and snapshot actions. A helpful second-version explanation replaces Compare Versions while there is only one capture. Close history.
5. Failure: duplicate tracking panels, a missing project toolbar, missing history, or controls stuck on Working. Recovery: close/reopen the detail view and use **Scan for changes** once; inspect visible tracking errors. For four small local files scans normally take seconds; locks, OneDrive hydration or large folder trees can delay them. Do not repeatedly click scan or delete app data.
6. To exercise archival too, now enable **Enable automatic archival of replaced source files**. It is separate consent. In-place saves of `Master_Context.md` keep that active filename and archive copies of prior snapshots under `Old Context Files`; the current source is not moved. With archival off, immutable historical snapshots still support every comparison below.

## Test 2 — Compare v1 and v2

1. Preparation/edit: replace the entire active `Master_Context.md` contents with Revision B and save once. Close Notepad or finish saving before scanning.
2. Wait for v2, or click **Scan for changes** once. Open **View history → Compare Versions**. Default selections should be v1 and v2. Click **Compare selected versions**.
3. Expect the old goal removed and the new goal added; `Track document versions` and `Archive old versions` added; Planning removed and Development added. With exact contents/final newlines, expect **4 lines added, 2 removed**. Bytes and text differ. No invented similarity score appears.
4. Failure: old/new reversed, missing edits, extra captures created by comparing, blank results or perpetual Comparing. Recovery: return to history, verify A/B captures and retry the pair. Use the displayed integrity/encoding/size error to correct the cause; never replace a historical snapshot with the current source.

## Test 3 — Compare v2 and v3

1. Preparation/edit: return to history and close it, replace the active file with Revision C, save, and wait for v3 before reopening history.
2. Click **Compare Versions**, confirm default v2/v3, then **Compare selected versions**.
3. Expect the goal replacement, removal of `Discover Markdown files`, addition of `Compare historical documents`, and the Architecture heading with React, Rust and SQLite. Development remains unchanged. With exact contents expect **7 added, 2 removed**.
4. Failure: changes from another family or incorrect additions/removals. Recovery: check selected capture numbers and original filename; retry from the correct family's history. Extra intermediate captures change the default pair; choose B/C explicitly.

## Test 4 — Nonconsecutive and reversed selection

1. Preparation/content: keep the three captures from Tests 1–3; no file edit is needed.
2. In comparison select v1 as Earlier and v3 as Later independently, then compare.
3. Expect the complete A→C interval, including goal, features, architecture and status. With exact contents expect **10 added, 3 removed**. A result can compare two historical versions even if neither is current; also select v1/v2 now that v3 exists.
4. Select v3 in Earlier and v1 in Later. Expect an ordering explanation. Compare: the result heading must still read v1 → v3 and additions/removals must match the forward result.
5. Failure: selection silently switches family or reverses additions. Recovery: go back to history and choose again. Selectors only list captured UUIDs from this family, never arbitrary paths.

## Test 5 — Unified view and context

1. Preparation/content: select A→C with no additional edits.
2. Choose **Display mode → Unified diff**. Each changed region shows related removals and additions together, with old/new line numbers, `−` and `+` markers and neutral context.
3. Click **Expand unchanged context**, then **Collapse unchanged context**. Default context includes up to three unchanged lines on each side. Long regions show 100 lines per page; **Next lines/Previous lines** expose all content without truncation. A missing final newline is explicitly marked if applicable.
4. Expect literal Markdown, including headings/list markers, rather than rendered HTML. Test dark/light themes after returning to Settings; the comparison should remain readable when reopened.
5. Failure: whitespace disappears, changed lines cannot be reached, or markers depend on color alone. Recovery: verify the selected pair, expand context and page through the current region; report the exact line/range if still absent.

## Test 6 — Side-by-side view

1. Preparation/content: use the same A→C result.
2. Choose **Side-by-side diff**. Expect Earlier v1 left, Later v3 right, related replacement lines aligned and blanks for single-sided insertions/deletions. Text wraps within each half.
3. Change display modes repeatedly; summary totals must stay the same. Use Next lines if a region exceeds a page.
4. Failure: left/right invert, counters change with display mode, text is clipped without a way to read it, or the app freezes. Recovery: switch to Unified, return to history and retry. Do not edit source files as a workaround.

## Test 7 — Change navigation and modal lifecycle

1. Preparation/content: use A→C, which has multiple separate changed regions.
2. Click **Next change** until the last region, then **Previous change** back. Expect `Change N of M`, the correct region brought into view, and navigation disabled at each endpoint. Paging within a region is independent of change navigation.
3. Click **Back to version history**, then **Compare Versions** three times. Expect one dialog, working selectors, and the Compare Versions button focused on return. Close history, navigate Dashboard → Projects → test project, and reopen it.
4. Failure: duplicated tracking panels/dialogs, stale Working controls, missing Refresh documents or data from a different project. Recovery: close/reopen the app and report the exact transition; keep all test history for diagnosis. Escape/Close dialog must remain available during calculation and errors.

## Test 8 — Identical and normalized-equivalent content

1. Preparation/content: keep at least two captured versions. Identical bytes in one family are deduplicated by tracking; saving identical text does not guarantee a new capture.
2. Open comparison and select the **same captured version in both selectors**, e.g. v2/v2. This supported self-comparison is the controlled identical fixture. Click compare.
3. Expect **No textual changes**, 0 added/removed/change blocks, **Text equivalent**, **File bytes identical** and disabled change navigation.
4. Optional normalization fixture: with the test file only, save the current text once using UTF-8 with BOM or change CRLF to LF in an editor that exposes this option. Wait for a new byte-distinct capture. Compare the immediately preceding capture against it. Expect equivalent text, zero changes, **File bytes differ**, and a normalization warning. Do not alter whitespace or remove the final newline in this fixture.
5. Failure: byte identity claimed for different package bytes, newline-style-only changes produce many replacements, or duplicate captures are required to reach equivalent state. Recovery: use same-version self-comparison first, then verify encoding/newline settings. Invalid UTF-8/UTF-16 is rejected with an explanation, not silently replaced characters. A missing final newline is a real textual change.

## Test 9 — Multiple independent families

1. Preparation/content: create `Technical_Documentation.md` in the test root with exactly `# Technical Documentation`, a blank line, then `Uses SQLite locally.` and a final newline. Wait for its independent v1.
2. Open its **View history**. Expect the one-version explanation; Master Context versions must not appear. Edit its last line to `Uses SQLite locally with immutable snapshots.`, save, wait for its v2, then compare.
3. Expect only the technical sentence replacement and this family's source names. Return to Master Context history: its own v1/v2/v3 remain separate. Capture numbers are family-scoped.
4. Failure: a selector lists the other family's files or combines histories. Recovery: close history, reopen the intended family, and stop testing association decisions until the mismatch is diagnosed. The Rust command independently rejects cross-family UUIDs even if a UI were to send them.

## Test 10 — Restart and historical independence

1. Preparation/content: finish saving all edits and record family/version counts. Close ContextDock normally, reopen and navigate to the test project.
2. Open Master Context history and compare A→C again, then A→B while C remains current.
3. Expect the same hashes, totals, text and original metadata. Comparisons use preserved snapshots, not current sources. Restart alone must not create extra versions of unchanged bytes.
4. Failure: history disappears, old pairs read current source text, or an unchanged startup creates extra captures. Recovery: verify the test folder and app version; retain the existing application-data directory. Never reset the database or copy current files into historical snapshot locations.

## Test 11 — Optional Microsoft Word DOCX

1. Preparation/content: in Word create `Word_Notes.docx` in the disposable root containing a heading `Meeting Notes` and paragraph `We will work offline.` Close Word, wait for its independent v1.
2. Add paragraph `Comparison uses preserved snapshots.`, save/close and wait for v2. Compare v1/v2; expect the new paragraph and readable paragraph boundaries. Optional: add a two-cell row `Owner` / `Mihir` and compare; cells use tab boundaries in consistent XML order.
3. Make only a formatting change (bold the first sentence), save/close and wait. If Word changes the package bytes, tracking captures another version. Compare the before/after formatting pair: expect **Text equivalent** but **File bytes differ**. If Word produces byte-identical output, no new capture is expected; do not force one by editing snapshots.
4. The extraction-policy disclosure must explain: body paragraphs and run text are included; hyperlink labels and displayed field results are included without execution; insertions/move-to included, deletions/move-from omitted. Headers/footers, notes, comments, images/charts, text boxes/drawings, embedded objects and field instructions are omitted. Formatting, generated numbering, hidden-text visibility and page layout are not compared. Merged cells follow XML order; nested tables report unsupported rather than inventing layout. No macros, fields, links or external resources are executed.
5. Failure: bold/run splits alone create sentence edits, corruption crashes the app, or omitted structures are presented as fully identical documents. Recovery: close Word to release locks, retry valid captures, review policy warnings and use Word for visual/formatting comparisons. Corrupt/oversized packages fail with a readable error and leave Back/Close available.

## Test 12 — File and history safety

1. Preparation/content: finish all edits, close Notepad/Word and wait until tracking settles. Record each family's version count, latest activity and archive filenames. Optionally hash the disposable source/archive files in PowerShell: `Get-ChildItem -LiteralPath "$HOME\Documents\ContextDock-Diff-Test" -File -Recurse | Get-FileHash -Algorithm SHA256`. Use the actual folder path if Documents is redirected into OneDrive.
2. Without saving any source, compare several pairs in both modes, navigate changes, and optionally click **Copy comparison summary** and **Copy changed text**. Paste into Notepad without saving into the watched folder.
3. Return to history/project and recheck counts, activity, filenames and hashes. Expect no new versions, archive moves, source edits or comparison journal entries. Clipboard summaries contain factual counts; changed text contains all changed blocks, not just the visible page. No export file is automatically written.
4. Failure: comparisons create versions, change bytes or trigger archival by themselves. Recovery: stop testing and preserve test files/data for diagnosis. Distinguish unrelated watcher captures caused by your editor/cloud software from a comparison action; do not modify immutable snapshots to investigate.

## Limits and safe retries

Each side supports a snapshot up to 16 MiB, extracted text up to 4 MiB, 20,000 lines and 64 KiB per line. Myers calculation has a 2-second complexity budget; exceeding it returns an error, not an approximate result. DOCX additionally permits 2048 ZIP entries, 128 MiB declared expanded package total, 32 MiB per entry, 200:1 compression ratio, 8 MiB body XML and XML nesting depth 128. ZIP64/multi-volume packages and nested tables are unsupported. One native comparison runs at a time. Back/Close can leave the view while its bounded worker finishes; a new request during that interval may say to retry shortly. No disk comparison cache exists.

Missing snapshots, hash mismatches, locks, invalid encodings, malformed XML/ZIP or size/complexity violations display the actual cause. Retry after releasing a lock or selecting another valid pair. Missing/corrupt history must be recovered from a trusted whole-store backup; the app never silently substitutes the active source. Do not experiment by corrupting the user's real snapshots.

## Acceptance checklist and cleanup

- [ ] Existing installation upgraded to 0.3.0 with projects/theme/history retained.
- [ ] A/B/C comparisons and nonconsecutive/reversed selection work with correct totals.
- [ ] Unified/split modes, context expansion, region navigation and paging work.
- [ ] Equivalent text and byte identity are distinguished; self-comparison works.
- [ ] Independent document histories cannot be mixed.
- [ ] Restart preserves results; no duplicate panels or dialogs appear.
- [ ] Errors clear loading and permit retry/Back/Close.
- [ ] Optional Word body, run formatting and table tests match the disclosed policy.
- [ ] Comparing and clipboard actions leave all source/archive/snapshot/history bytes intact.

Cleanup: disable tracking for **ContextDock Diff Test**, then **Remove → Remove registration**. This retains all source/archive/snapshot/history bytes. If desired, delete only your disposable `ContextDock-Diff-Test` folder in Explorer after verifying its full path. Do not delete `%LOCALAPPDATA%\com.contextdock.desktop`; it contains other projects and retained histories. Automatic snapshot/history purge is not part of this milestone.
