# Milestone 6 — folder reassignment and missing-file recovery (0.6.0)

Use disposable folders and documents. Automated tests use temporary SQLite/source stores and mocked cloud APIs; the installed checks below remain user acceptance. No key or cloud request is needed for tests 1–9.

## Prepare and upgrade

Close ContextDock before running `ContextDock_0.6.0_x64-setup.exe` from the intended branch's successful Windows installer workflow or the local `src-tauri/target/release/bundle/nsis/` output. Keep the existing per-user installation and application-data directory. Schema 006 preserves registrations, themes, snapshots, histories, cloud settings/mappings and saved chats. Do not downgrade to 0.5.4 after migration. The installer remains unsigned.

Create a new disposable folder, for example `%USERPROFILE%\ContextDock-Folder-Test`, outside app data and any other registered project. Add `Master.md` containing `# Master` and `Original project notes.`; add `docs\Technical.md` containing `# Technical` and `Uses React and SQLite.`. Register it as **Folder Recovery Test**, enable tracking for both files, keep archival off, and verify two independent v1 histories. Use a new path if the example was registered before, since removal retains history.

## 1. Move while running

Finish saves and close the editors. In Explorer create another disposable parent and move the **whole Folder-Test folder** into it. After the current operation finishes, allow a few seconds for health checking. Dashboard/project cards should show **Folder lost · Reassign**; opening the project shows **Folder lost — please reassign it** and the old path. Tracking/cloud work panels must be blocked. Navigate to another project, Dashboard and Settings; the app must remain usable.

## 2. Restart before fixing the path

Close ContextDock normally, reopen it and open Folder Recovery Test. The same reminder and project name must remain. **Refresh documents** must not create a new registration, erase histories, clear the reminder or start captures from another folder. App shutdown must continue to work. Tracking-off projects must also detect root loss.

## 3. Picker cancellation and review

Click **Reassign folder → Choose folder**, then cancel the native picker: registration and reminder stay unchanged. Select the moved project folder itself on a second attempt. The review should show two known/unchanged documents, zero missing, and the new canonical location. The final button stays disabled until the confirmation checkbox is checked. Cancel the review once: the old registration remains and no files move.

Selecting only the new parent lists known files as missing and may show additional nested documents. If no known files are found, the review explicitly warns that this could be the wrong folder. Cancel and select the actual project root. An empty replacement can be explicitly acknowledged, but cannot recreate missing documents.

## 4. Confirm the moved folder

Choose the correct moved folder again, review, check confirmation and **Confirm reassignment**. Rapidly double-click: one reassignment should run, and the controls stay guarded through backend readback. Expect the same project name, two original families/v1 histories, saved snapshots, tracking preference and theme. The old root is not copied/recreated. No document bytes or captures change solely because the root moved. Opted-in tracking installs a new watcher; **Refresh documents** can trigger immediate reconciliation.

## 5. Capture after reassignment and restart

Edit the moved `Master.md` to `Updated project notes.`, save and close. Wait or refresh. Only Master advances to v2; Technical stays v1. Repeated unchanged refreshes add no duplicate versions. Restart and verify the reassigned location and independent histories persist. Compare Master's v1/v2 to verify old snapshots remain usable. If source archival was separately enabled, its existing safety/consent rules still apply after recovery.

## 6. Partial move or deleted file

Close the app. Move the whole project root again, but keep `docs\Technical.md` separately in a verified disposable holding folder. Relaunch and review reassignment: Technical's relative path must be listed as missing, while Master is found. Explicitly acknowledge and confirm the partial move. The recovered project must show missing-file counts and Technical's missing status, retaining its family/history instead of deleting it. Restore that exact file to `docs\Technical.md`, then **Refresh documents**: availability returns with no extra content version for unchanged bytes.

If an entire nested source folder is moved within the root, refresh and review any ambiguous document matches. Exact-path observations and conservative unique-hash relocation keep the existing matching rules; the application never guesses between live duplicate copies or multiple families.

## 7. Original folder returns

Move the root away, wait for the warning, then return the same directory to its original registered location. The recovery reminder must remain until explicit reassignment. Select that original folder and confirm to recover it. A different directory created at the old saved path must also require reassignment instead of silently becoming the project. The initial upgrade can record only the identity of a root currently accessible to 0.6.0; it cannot reconstruct a replacement made before an identity was ever stored.

## 8. Invalid destinations and changed review

Try another registered project's folder, its child or parent, a retained removed registration, app data, a file path, unavailable folder or junction/link. The action must fail visibly and preserve the previous association. Choose a valid disposable folder, leave the review open, then edit/rename one of its expected documents or move the selected directory. Confirmation must refuse stale/unavailable review data. Choose the folder again and review the updated results. Locked/unreadable expected documents or unreadable subdirectories must fail the review with an actionable error; release locks/restore access and retry.

## 9. Missing sources and pending operations

Remove one current disposable source while keeping the root. Refresh: the document must say missing, and its saved history remains. An unreadable subdirectory produces partial/unavailable status rather than claiming all its files were deleted. Release the lock/restore the source/access and refresh. Existing pending capture/archive conflicts remain listed with retry actions; recovery must never overwrite a conflicting source/archive. Cross-volume copies can invalidate old archive file-identity evidence and must leave such operations pending. Do not remove journals or snapshots to force success.

## 10. Optional cloud/chat acceptance

Only use your own account and disposable content if cloud behavior is being accepted. Index the two samples and save a local chat first. Root loss must stop new sync/questions; reassignment must reuse the same cloud mappings, not create a second bucket. Questions wait for local reconciliation and changed current-version indexing. A missing selected current source blocks new questions/sync and displays the local readiness error, while saved chats remain readable after root recovery. Restore the file and refresh to resume. Explicit cloud cleanup/project removal remains available during folder loss; an already-sent request may finish. Never enter a key in this guide/chat/repository.

## Finish

Check light/dark themes and Windows scaling, including long Unicode paths, the confirmation checkbox and paged missing-file lists. Close/reopen once more and confirm all recovered associations and histories. Record any numbered-test failure with version, expected/actual state and redacted error. Clean any explicitly created disposable cloud resources first, then remove only the disposable registration/folders. Keep the real app-data directory intact; removal retains local history and is not a storage purge.
