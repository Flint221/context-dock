# ContextDock database investigation v1

Investigation date: October 22, 2026 (America/Chicago). Reviewed branch: `codex/new-file-selection`, implementation/test HEAD `fa5b2c85ff73b2460950cfa440fa309a83112e76`. Remote HEAD matched. The worktree was clean at entry. This report adds no application behavior changes.

## Decision

Do not install or launch the build against the existing registry and do not start live cloud verification yet. The available database/WAL pair is incomplete or mismatched. Copy-only salvage preserves useful records but is not a validated operational registry. Replacing it with the empty schema-1 base, discarding the WAL, rebuilding registrations or resyncing from a partial reconstruction could lose history or change existing cloud resources.

No original database, WAL, SHM, snapshot, staging file, registered source or cloud resource was intentionally modified. No SQLite connection opened the original files in this investigation. Filesystem SHA-256 inventories taken before and after the copy-only analysis match, including SHM. No ContextDock process was found at entry. No GUI, installation, uninstall, credential retrieval or provider request was performed.

## Preservation and reproduction

Fresh evidence copies are under `%LOCALAPPDATA%\ContextDock_Investigation_v1`. Original_Copy and Prior_Backup_Copy preserve database/WAL/SHM/snapshots/staging. The previously supplied backup and the current source have identical relative-file inventories and SHA-256 values. All 60 selected files were copied and verified. This is independent of any claim that an earlier read-only SQLite connection could not change SHM: this investigation checks actual bytes and uses SQLite only on derived copies.

Original evidence fingerprints:

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| contextdock.db | 40,960 | d75d67823ee14f556c9597b685738bce5a73684f0872513701c3b272bd6369a2 |
| contextdock.db-wal | 4,523,792 | d790dd7c77112b07653af66221d02247b52ab2272144e695454c6e2d79842f56 |
| contextdock.db-shm | 32,768 | 5d72801d45299448e9d77572af53eb4350b215600b13b987cd7a1cdc1708448f |

Python SQLite 3.53.1 reproduces schema version 9 and malformed schema/integrity errors with the copied WAL. These probes omit the copied SHM and let SQLite rebuild it on the disposable copy, ruling out stale SHM as a sufficient explanation. Base-only immutable probes return schema 1 and integrity ok, but all three base tables (projects, documents, preferences) are empty. This base contains no usable recent project registry.

An independently downloaded official SQLite 3.54.1 ARM64 CLI reports the same malformed error on the committed reconstruction. The tool ZIP SHA3-256 matched the official download listing; provenance is recorded locally. This rules out a Python-only binding problem as the explanation tested here.

## Physical evidence and limits on causation

The WAL uses 4,096-byte pages and contains 1,098 complete frame slots. Its header checksum and the rolling checksums/salts of frames 1 through 1,075 validate. The final valid commit is frame 1,075 and declares 149 database pages; the copied SHM independently reports mxFrame 1,075, nPage 149 and backfill 0. The remaining 23 slots have older-generation salts. Such retained WAL tail slots are not part of the current committed state; their existence alone is not corruption.

The base has only ten pages. Of the 149 pages required by the last committed state, 65 are absent from both the base and the current valid WAL generation. Current page 1 points to schema leaf pages 26, 28, 58 and 89; all four are among the missing pages. Rebuilding the current page view from the preserved base plus valid committed frames still fails schema reading. This explains the malformed-schema result without relying on a guessed SQL migration defect.

A separate chronological overlay probe was retained locally, but its early schema-1 successes do not establish a recoverable healthy historical registry: those views inherit the same inadequate base. No healthy complete recent transaction state was established from this pair.

The evidence is consistent with a stale, truncated or replaced base paired with a later WAL, or another loss of checkpointed pages. It does not identify the responsible process, action or hardware event. No attribution to SQLite, a particular installer, another chat or a filesystem failure is proven. Schema 10 is absent from the preserved registry, so this investigation does not show that the branch's migration 010 executed against these files.

SQLite explains the dependence on the database/WAL pair and the nonessential nature of SHM in its [WAL format documentation](https://sqlite.org/walformat.html). Its [corruption guide](https://sqlite.org/howtocorrupt.html) discusses mismatched journals, replacement/restore and other possible causes; these are hypotheses here, not diagnoses of the initiating event.

## Copy-only salvage

Official CLI `.recover --ignore-freelist` was run only on derived images. It produced a lost_and_found container with 1,514 records, including secondary-index entries; this must not be described as 1,514 recovered application rows. Its integrity check is ok as a new salvage container, not as proof that ContextDock has been restored. No recovered container was installed or connected to a cloud worker.

Surviving records include seven project records, 43 document-family records, 34 source records, 49 complete version metadata records, seven cloud-project records and 22 cloud-job records. Missing schema pages prevent treating inferred table identities and row completeness as a fully validated database. Other table roots and data leaves are missing too, so chats, exclusions, connection/installation state, operation history and other tables cannot be assumed complete or empty.

All 57 snapshot files (21,585,474 bytes) are preserved. Surviving primary/unique index records identify 57 snapshot paths, version IDs and sequence numbers. Forty-nine full version records and surviving hash-index leaves jointly supply expected hashes for 56 snapshots; all 56 match preserved file bytes. Eight full version metadata rows are missing. One snapshot has no independently recovered expected hash, so its historical integrity is unverified even though its bytes are preserved.

The initial exploratory snapshot audit used the wrong inferred hash-index mapping; it is retained as evidence and superseded by `ContextDock_All_Snapshot_References_v2.json`, which explains the correction. No operational reconstruction uses the exploratory mapping.

Older-generation tail pages were explored in an additional explicitly quarantined image only. They provide historical fragments but are unsafe to splice into the current database: page numbers may have been reused and some fragments do not match the current inferred table shapes. They were not merged into the committed salvage or original files.

SQLite describes `.recover` as salvage with possible missing/altered/resurrected records and invalid relationships in its [recovery documentation](https://sqlite.org/recovery.html). A successful recovery command is not sufficient evidence for resuming automatic cloud activity.

## Cotton Trax evidence

Recovered records contain `CottonTrax_Team_Tasks_v1.md`, rather than the unsuffixed filename in the handoff. It has an available/tracked source, a tracked document family, a current snapshot and a completed capture operation. Its recovered version metadata and snapshot bytes verify. It has no surviving legacy cloud_documents row and no cloud_job record. Cotton Trax has nine saved indexed jobs. These are local database observations, not a live check of OpenAI's current index.

This supports the original split-selection diagnosis: a captured local file could be absent from the old cloud selection. Missing saved path-exclusion pages prevent independently confirming the entire project's folder selection from this damaged registry. The count of nine saved jobs does not by itself establish which other documents should currently be uploaded or removed.

## Branch review and independent validation

Reviewed the branch history from `746a10f`, the migration and the affected registry, tracking, folder-selection, cloud scheduling/queue/recovery/authorization and UI code, plus targeted tests.

- Migration 010 creates new_paths and enables auto_sync for already-enabled cloud projects. It does not drop history, replace installation/store/File identities or delete indexed resources. Existing paused state remains meaningful. It changes the supported schema to 10.
- Saved folder rules drive initial capture and cloud desired versions. The legacy cloud document rows remain stored for compatibility but no longer authorize eligibility. Legacy version exclusions no longer override saved folder selection.
- Discovery after the initial scan adds explicit file exclusions and New markers. Save admits new paths; tracking-off blocks capture. High-confidence inheritance uses the existing matcher, checks excluded parents and ambiguous batches, binds the identified family to staged bytes, and otherwise retains exclusions.
- Scheduler/queue logic notices missing selected snapshots, reuses existing job/File identities and preserves uncertain upload recovery. Upload/attach guards recheck tracking, saved path and folder revisions, connection scope and project permission.
- Saved folder deselection uses the existing verified path-removal flow. Selection save drains the cloud gate before registry locking, verifies saved rules and cleanup intents and retains local snapshots/history. Missing active sources or inconsistent captured families block reconciliation rather than authorizing silent index pruning.
- The visible cloud document checklist and automatic-sync checkbox are removed. Folder-tree draft/save/discard behavior remains; the application page order is not changed by this branch.

Fresh checks at the reviewed HEAD: **197 native tests passed, one intentionally ignored child-fixture entry; 131 frontend tests passed across 15 files**. TypeScript, ESLint, rustfmt and all-target Clippy with warnings denied passed. The simulated nine-to-ten CottonTrax test passes, including repeated sync without duplicate uploads. Migration, missing-source/index preservation, ambiguous inheritance, parent exclusions, paused tracking and cleanup/reselection tests pass. These are disposable-fixture/mocked-provider results; no installed or live-provider acceptance is claimed.

The existing installer is 4,490,009 bytes and timestamps after the final production-source commit and before the last three test-only commits. The installed executable differs from the release executable. Both retain product version 0.8.0, so that version string alone cannot distinguish them. No fresh installer was built or installed in this investigation. A distinct version should identify the next release after recovery and acceptance work.

## Next safe step

Locate another complete recent database backup or a Windows File History copy, preserving any companion WAL. Compare its identifiers, snapshots and relationships on copies before considering replacement. The two currently known backups contain the same incomplete pair. A further backup location was requested from the user.

If no complete copy exists, prepare an explicitly incomplete offline reconstruction with a per-table missing-data inventory and validate it against preserved snapshots and, if authorized, read-only cloud metadata. Do not manufacture missing history dates, silently reset installation identity, overwrite the original, or start automatic sync. Any proposed replacement or cloud mutation beyond the intended selected-document sync needs a concrete loss/risk assessment and user approval first.
