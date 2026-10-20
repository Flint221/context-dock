# ContextDock 0.5.4 — simple cloud chat acceptance

Use a disposable project with a tracked Markdown document containing `React frontend` and `SQLite database`. Existing projects/history remain available. Install 0.5.4 with the app closed. Schema 4→5 retains snapshots, archives, cloud mappings/settings and old baselines/reports; do not downgrade to 0.5.0 after migration.

1. Save and test your own key in Settings. Open the disposable project. With Cloud sync off, confirm history/diffs still work and Ask project explains why Send is unavailable. No baseline form should appear. Folder actions are under Project folder and options.
2. Turn on Cloud sync. Review the named documents and disclosure, then Cancel: the switch must stay off and no initial upload should begin. Turn it on again and confirm Enable Cloud sync and AI. One confirmation saves selection, enables automatic version sync and starts initial sync. No separate Start button is needed.
3. Wait for Documents ready for questions. Ask `Which database does this project use?` and Send. Expect a generated answer grounded in the vector-store passages, a document name/version under Sources, and a saved chat titled from the first question. Each Send may incur retrieval/model charges; tests by the agent use mocks only.
4. Ask `What about the frontend?` in the same chat. Expect recent conversational context, a fresh retrieval of current versions and a saved second turn. It should not re-upload complete local snapshots for this question.
5. Choose New chat and send a different question. Switch between the two saved chats. Restart ContextDock and reopen the project: both conversations/answers should remain. The database is `%LOCALAPPDATA%\com.contextdock.desktop\contextdock.db`; no separate JSON file is required.
6. Turn Cloud sync off. Cancel once to verify the original state remains; then confirm. New uploads and questions must stop, while both saved chats and local version history remain readable. An already-sent request may finish. Off does not delete cloud files.
7. With sync off, Delete chat → Delete local chat. Only the selected conversation disappears. The other chat, document history and OpenAI files/store must remain. Restart to verify deletion persists.
8. Re-enable sync, edit the working document to use a different database and wait for local capture. Questions must wait while the latest version is pending; use Sync now if necessary. After indexing, ask again and verify the answer cites the new capture number. Full-history indexing must still restrict ordinary chat to current versions.
9. Disconnect the network or use a test account with rejected requests. Send must unlock after failure, retain the question/error in history when dispatch began and never automatically repeat a paid model call. An interrupted request becomes uncertain on restart; inspect provider usage before manually sending again.

Chat uses up to five provenance-checked search passages, recent completed turns within a bounded context budget, 2,000-byte questions and 2,048 maximum output tokens. Older messages remain saved even when outside model context. It is a question-and-answer interface, not an exhaustive comparison, autonomous tool, source editor or rollback feature. Sources validate which retrieved files were supplied, not the truth of every generated assertion.

## Major toggle acceptance

For project Cloud sync, advanced automatic sync/pause/resume, tracking/archival and Settings → Disconnect OpenAI: open the confirmation and Cancel first. The saved state must remain unchanged. Confirm next and rapidly double-click: one operation should run, the spinner should remain through backend verification and the controls should unlock afterward. Success shows a checkmark only after readback; sync request confirmation must not claim that indexing is complete. A failed status read must show an error/unknown state and offer status refresh instead of allowing questions from stale state. Disconnect OpenAI removes the key only after confirmation and keeps local/cloud documents; do not use it unless you are ready to re-enter your own key. No test requires sharing the key with the coding agent.

## Streaming and keyboard acceptance (0.5.4)

- Confirm Cloud sync is a full-width pill with its thumb inside the track in both states, light/dark themes and Windows scaling.
- With the current files indexed, type a first question and press Enter. Exactly one question is sent. Shift+Enter inserts a newline; IME composition Enter does not submit.
- After retrieval, the answer appears progressively. Controls stay guarded until final validation/save. Open Sources: only filenames and captured versions appear, without passage previews.
- Ask a follow-up such as “And which frontend?” in the same chat. Up to six recent completed exchanges (12 KB maximum) provide context. Start a new chat to reset this context; older history is still saved.
- Interrupt connectivity during generation. Partial text must disappear, a failed/uncertain turn must remain, and no paid automatic retry must occur. A subsequent explicit Send is a separate request.
- Restart and check that only completed answers and their source metadata appear as finished replies. Existing saved sources without filename metadata fall back to their document family name.

## Cloud erasure and project removal (0.5.4)

Use a disposable project/account scope for destructive acceptance; other stores/files must remain intact.

1. Index two selected documents, edit one and sync again. Note the project bucket and uploaded Files in OpenAI's dashboard.
2. Uncheck Cloud sync. Confirm the dialog names the cloud project and says it deletes both bucket and uploaded Files, including older versions. Cancel first: nothing is deleted.
3. Confirm. Controls show a spinner; repeated clicks do not dispatch duplicates. Success waits for deletion and absence verification. Check that both the bucket and project uploads are gone, while source files, version history and saved chats remain.
4. Enable Cloud sync again. After confirmation, a fresh bucket and uploads are created; questions unlock after indexing. Existing local capture history is reused.
5. Disconnect the network during cleanup. The project must say Cloud deletion incomplete, never a clean Off/success. Restore connectivity, retry Delete cloud data, and verify cleanup completes. Close/relaunch during cleanup: it remains pending for an explicit retry and retains remote identifiers.
6. Select **Remove project** beside Refresh documents. The dialog defaults to deleting cloud data. Confirm only on the disposable project; registration removal follows cloud verification, and the folder remains intact.
7. On a second disposable project choose **Keep cloud data in OpenAI** when removing. The project disappears locally, while its bucket/uploads remain by explicit choice. Re-register the same folder: history/mappings return with tracking off; Delete cloud data can clean it later.
8. A file referenced by another accessible bucket must block deletion with an error. Foreign/mismatched files must never be purged. Remove the deliberate shared reference before retrying.
9. Global Disconnect OpenAI must refuse while active projects still have cloud resources, explaining how to clean them first. Never remove the key needed to finish a failed cleanup.

The old Delete project cloud index workflow queued bucket deletion but separately retained uploaded Files. The new **Delete project cloud data** workflow verifies deletion of both, and records counts in Recent cloud activity. UI-level success is not a live-account acceptance substitute.
