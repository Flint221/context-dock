# ContextDock external-drive reconnection

Version 0.10.3 fixes persistent folder loss after reconnecting an external drive. Previously, `check_folders` only checked ready roots, and `require_ready` immediately rejected roots already marked `needs_reassignment`. Restarting preserved that state even when the original directory returned.

Lost roots are now checked during workspace loading and the native monitor's existing approximately two-second health interval. A root reconnects only when its saved path is readable, resolves to the actual directory without a link or junction and has the same saved filesystem identity. Windows identity uses volume serial number and file index. No other drives or locations are searched. A replacement directory, changed drive letter, moved folder or missing saved identity still requires explicit reassignment. Identical filenames/content alone cannot bypass the identity check.

Recovery clears the root-loss error, increments the folder revision to invalidate stale reviews and returns the watcher to starting/off according to the saved tracking choice. The monitor rebinds automatically and reconciles files when tracking is enabled. Opening/refreshing a project scans its saved folder even after loss; **Refresh folders** on Dashboard and the Projects list checks and scans all current registrations. **Check saved folder** in a project's recovery panel retries its saved path without opening a folder picker and explains continued absence or an identity conflict. These controls do not change registration paths, enable tracking/cloud sync or reset projects.

Source IDs, captured snapshots, chats, path exclusions, cloud store IDs and pause/settings are retained. Files changed or deleted while disconnected follow existing reconciliation, missing-file and approval rules. Metadata refresh does not itself capture unchanged documents or upload anything; resumed monitoring/cloud workers follow saved policies. An unreadable root never authorizes deletion or archival.

## Acceptance checks

1. With a disposable tracked project on an external drive, disconnect the drive. Verify loss is reported and project work remains blocked. Reconnect it with the same drive letter. The project should recover automatically while running, including from the tray, or on reopening the app.
2. Use **Refresh folders** to check all project registrations at once. Verify registered paths and document IDs remain unchanged, including projects with tracking off. A drive that remains disconnected should still show unavailable.
3. In a project's recovery panel, choose **Check saved folder**. It should refresh the old location without a folder picker. If it is still missing, inaccessible or replaced, a red actionable message should remain and Reassign folder should be available.
4. Edit an already approved document after reconnecting. The watcher should resume and saved sync policy should apply. Verify exclusions, upload pause and cloud IDs are preserved. Existing snapshots should remain readable.
5. Place a different directory at the saved path, even with identical document names/content. It must remain blocked until reassignment. A different drive letter also requires reassignment because the saved path changed.

Automated tests simulate disconnect/reconnect by moving a disposable directory away and back while preserving its filesystem identity. They verify restart recovery, multi-project refresh, preserved history/selections/cloud identity, rejection of replacement or unknown identities and live native watcher recovery/capture. Physical cable removal, remount behavior of each external-drive filesystem and installed UI acceptance remain manual checks. Implementation and tests do not modify actual project files or application databases.
