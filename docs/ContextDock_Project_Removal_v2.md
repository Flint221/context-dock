# ContextDock project removal and actual registry reset v2

October 22, 2026. This supersedes the earlier removal verification in ContextDock_Project_Removal_v1.md. The user reported that Cattle Tool still appeared with database-malformed errors and explicitly authorized removing it to start fresh while retaining real documents and backups.

## Established cause

The installed 0.9.0 process held database file ID 3659174698719871, WAL ID 1407374885577110 and SHM ID 562949955445143 on the same volume. Its main file was 573,440 bytes, WAL 259,592 bytes and SHM 32,768 bytes. Main-file SHA-256 exactly matched the earlier post-removal readback, proving that the earlier tombstone write remained present in the main file.

On disposable analysis copies, main-only reading showed schema 9 and Cattle Tool removed_at set. Reading that same main file with its actual WAL showed schema 10 and Cattle Tool removed_at NULL. Both views still reported corruption. The WAL therefore overrode the earlier main-file-only registration removal. Checking or changing the main file without the actual companion WAL did not establish the application's effective state.

A second verified complication was app-data redirection. A fresh launch through the Codex Computer Use helper opened a different healthy database beneath Packages/OpenAI.Codex_2p2nqsd0c76g0/LocalCache/Local/com.contextdock.desktop, while the original user-launched process had opened the unredirected app-data database. Native handle identities and resolved paths distinguished these objects. This explains why ordinary-path checks or a helper-launched empty app were insufficient evidence for the original application's state. The initiating cause of the pre-existing database corruption remains unknown.

## Preservation and reset

The app exited before the reset. Raw copies of the actual main/WAL/SHM were preserved both while running and after exit, by Windows file ID. Their closed-copy hashes matched the earlier running copies. Evidence and recovery material are retained in ContextDock_Registry_Reset_v2, including Running_Copy, Closed_Copy, analysis copies, manifests, exact-file helper sources and reset readback. Because the work ran in Codex's packaged context, the physical evidence folder is C:/Users/Mihir/AppData/Local/Packages/OpenAI.Codex_2p2nqsd0c76g0/LocalCache/Local/ContextDock_Registry_Reset_v2. Earlier backups were not deleted or overwritten.

A fresh registry was constructed separately from migrations 001 through 010 with a new cloud installation identity and no project/cloud resource bindings. Before changing the actual objects, the reset acquired the actual runtime lock and exclusive handles for all three recorded files and compared their complete bytes against the closed backups. The old WAL and SHM were cleared, and the actual main file was replaced with the validated fresh registry using its exact file ID. Writes were flushed and complete readback verified. Rollback to the preserved bytes was specified for any write/readback failure and was not needed. No original project directory, document, archive, snapshot or existing backup was deleted.

The exact-object post-reset copy passed integrity_check, returned no foreign-key errors, had schema 10 and zero projects, documents and cloud jobs. Its main file was 327,680 bytes; its old WAL and SHM were both zero bytes. All 90 inventoried source/archive/snapshot files retained their exact sizes and SHA-256 hashes. No credentials were retrieved and no cloud requests were made.

## Application acceptance and remaining scope

A helper launch displayed zero projects and no database errors, but its redirected database identity was identified and not treated as verification of the original file object. A normal Explorer launch check was then attempted. The user stopped Computer Use with Escape; no subsequent Computer Use calls were made. The user independently reported: "I cheked the app its 0 projects now". The final claim of visible zero projects therefore includes direct user acceptance, alongside the exact-object reset and integrity verification.

No folder reassignment was performed. Re-adding project folders can start fresh registration, history capture and cloud associations. Prior history and snapshots remain preserved as backups/orphaned snapshot bytes, rather than being claimed as recovered metadata in the new registry. This is an explicitly authorized one-time operational reset, not an automatic reset feature, corruption repair or installer change. Application code and the 0.9.0 installer are unchanged; only this report updates the repository.
