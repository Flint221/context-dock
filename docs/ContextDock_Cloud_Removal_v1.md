# ContextDock cloud index removal recovery

## Observed cause

A stable, read-only copy of the running application database and WAL passed SQLite quick_check. CC Weather Tool had successfully indexed both filename revisions. The v1 job was no longer desired in current-version mode, while v2 was desired and indexed. Removal verification for v1 then failed and persisted a terminal next_at value. This blocked further automatic work. Folder reassignment itself was complete.

The installed executable reported version 0.10.0. Its notification error is separate from cloud cleanup; notification and folder reconnection repairs are included in later releases.

## Behavior

After an acknowledged association deletion, a still-present association now schedules bounded exponential retry with a minimum five-second delay and at most six scheduled retries. The next attempt verifies ownership again. Invalid deletion receipts, foreign identities and ambiguous ownership remain blocked. The exact legacy terminal error is eligible to resume under existing authorization. Ordinary upload pause, project pause, folder and credential guards remain enforced. Explicit saved-selection removal retains its existing ability to complete while uploads are paused.

Counts include desired jobs only, so an obsolete association is not presented as another selected indexed version. Unresolved project errors prevent ready messaging and chat submission. No uploaded File or local history is deleted by this association cleanup.

OpenAI documents that vector-store file removal is eventually consistent for search results: https://developers.openai.com/api/docs/guides/retrieval . This does not prove the current remote state of this account. The investigation made no live API calls and changed no original database, source, snapshot, archive or backup.

## Verification and installed acceptance

Mock-provider regressions cover delayed removal, restart recovery, bounded exhaustion, invalid receipts, saved-selection cleanup, current-version counts and preserved uploads/history. A UI regression checks that cleanup errors block ready messaging and questions.

Exit the old app from its system tray before installing 0.10.4. With the project folder available, Cloud sync enabled and uploads unpaused, the exact historical stopped removal can resume automatically. If retries exhaust, inspect the displayed error and use Sync now to explicitly retry. Confirm the current desired revision remains indexed and the obsolete association is removed. This installed live-account acceptance remains unperformed.
