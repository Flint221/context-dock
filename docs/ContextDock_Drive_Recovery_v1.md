# ContextDock automatic publication and recovery

This records step 3, following [verified latest-file publication](ContextDock_Drive_Publication_v1.md). Schema 15 adds durable scheduling metadata without changing the schema-14 cloud identities or pending uploads.

## Scheduling

Enabling project publication now enables automatic publication. The watcher identifies a stable tracked save, then the Drive scheduler waits another three seconds for the current document generation to settle. A burst of saves updates the observed generation instead of publishing every intermediate state. The manual Publish latest action remains available for an explicit check or retry.

Native file events wake the worker after reconciliation. A five-minute metadata reconciliation recovers missed watcher events. The Drive worker also checks for database changes while idle, with a five-second wake ceiling; unchanged documents do not trigger staging, hashing, token refresh or Google requests. The worker processes one document per turn and rotates projects so a busy project cannot monopolize it.

An existing schema-14 project that was configured for manual publication retains that setting until the user explicitly enables automatic publication through the current controls. Failed work from a manual publication remains eligible for retry because that request and its document intent are persisted.

## Recovery and stopping

- Upload intent, file ID, expected content and generation remain in SQLite across a restart. A completed upload whose acknowledgement was lost is verified under that ID before a new generation is attempted.
- Network and provider failures receive increasing retry delays. Provider delay guidance is incorporated into the bounded backoff. Temporary editor locks and changing local saves remain Waiting for file rather than becoming permanent failures.
- Authorization failures stop background requests until the bound account is reconnected or explicitly retried. Successful reauthorization clears the account's authorization block and resumes its queued documents. It does not switch a project to another account.
- A conflicted document waits for attention while unrelated eligible families can continue. Missing or conflicting destination folders stop that project's publication without creating a replacement folder silently.
- Pause publication remains available during a manual upload. The currently sent request can finish; guards stop subsequent requests and preserve pending work. Pausing, tracking opt-out, account disconnection and removed-project state are checked before further outbound work.
- Global upload pause and battery-saver policy prevent new writes while local monitoring continues. Closing to the tray keeps the worker alive. Exit cancels pending sign-in, stops the worker from sending another request and waits off the UI thread for the current journaled operation to finish.

## Validation

Nine scheduler tests cover settling, unchanged idle checks, persistent backoff and file identity after restart, manual-request recovery, pause persistence, authorization blocking and account-scoped resumption, independent conflicted families, schema-14 pending-operation migration, provider delay and editor-wait classification. The publication interface also verifies pausing an in-flight operation without claiming an unverified upload succeeded.

The disposable native Windows tray test passed explicitly: a blank fixture window closed to the tray, tracked fixture edits while hidden, reopened and exited with the Drive worker registered. The full frontend suite passed with 179 tests and the full Rust suite with 304 tests. TypeScript checks, ESLint, production frontend build, Rust formatting and Clippy with warnings denied passed. Six opt-in integrations remain ignored by default even though the tray test was run separately.

Live Google authorization, picker operation and app-owned cloud transfer still require the new ContextDock Desktop OAuth client. Native scheduling and provider-independent failure tests do not replace that acceptance. No installed app, real project files, existing archives, saved chats or retained OpenAI resources were modified by these development tests. Interface simplification, the document/resource pilot and optional recycling remain steps 4–6.
