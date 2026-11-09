# ContextDock fresh production profile plan

Status: authorized, not executed. This plan is for the 0.11.0 Drive publication cutover after live acceptance and merge. [Reset v1](ContextDock_Local_Reset_v1.md) describes a different historical operation and remains unchanged.

## Exact scope

The production profile is `C:\Users\Mihir\AppData\Local\com.contextdock.desktop`. Read-only SQLite inspection, including its WAL, found schema 10 and no active project registrations. Its internal snapshots directory contains 231 files totaling 718,500,889 bytes; staging was empty. These observations must be rechecked at execution.

With ContextDock closed and its runtime lock held exclusively, reset only the active `contextdock.db`, `contextdock.db-wal`, `contextdock.db-shm` and obsolete internal `snapshots`/`staging` content. Resolve and inspect each exact target before mutation, reject reparse points and ensure all targets remain inside this profile. Keep a versioned copy of the small active database and journals before reset. Such a copy does not preserve discarded snapshot payloads and must not be described as a full-history backup.

Do not modify the four consolidated project directories, external archive directories, previous backups or Google Drive content. The user is consolidating actual documents separately; no data merging or archive restoration is needed. Do not recursively remove the entire application data or installation directory.

## Cutover and verification

1. Complete live recycling, recovery and native acceptance, then require green CI for the reviewed head before merging PR 16. Update local main and verify it contains the reviewed production source.
2. Close ContextDock, obtain the exclusive runtime lock and recheck the exact reset scope. Preserve a small versioned database/journal copy and remove only the authorized active registry/cache targets.
3. Install the verified 0.11.0 production NSIS installer in current-user mode. `/S` without `/R` installs silently without launching the application; check the installer exit code and installed executable version.
4. Release the runtime lock and launch the installed production application when the desktop is available. Verify schema 18, database integrity, zero project registrations and the initial disabled publication/recycling state. Check the visible startup flow before handing it back.
5. Connect production Google Drive and select the intended ContextDock root through the app's supported flow. Pilot credentials use a separate Windows credential-vault target and must not be copied into the registry or repository.
6. Add the four consolidated projects and review folder selection. Verify a latest-file replacement in one project before enabling automatic publication more broadly. Recycling remains off initially even though the disposable live test must pass before merge.

Record the actual reset, installed version and acceptance results in the next versioned document. Until then, this document does not establish that any cutover step has run.
