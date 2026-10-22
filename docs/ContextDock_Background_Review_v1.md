# ContextDock background operation and approval review

Version 0.10.0 adds close-to-tray operation, opt-in hidden Windows startup, a global Pending approvals inbox and grouped Windows notifications. Migration 012 adds review receipts and notification watermarks; existing project IDs, selections, snapshots, chats, archives and cloud identities are preserved. No startup registration is created automatically during upgrade or ordinary launch.

## Controls and consent

X first resolves unsaved selections and then hides the existing window. Tracking and cloud workers continue in Rust. The tray opens the app, opens Pending approvals with a count, pauses/resumes uploads, opens Settings or fully exits. Shortcut launches restore the existing instance. Startup launches use `--background` and do not steal focus. Tray failure leaves the window accessible and makes X exit rather than hide into an unavailable tray. Native close requests arriving before frontend readiness are queued.

Tray Exit also resolves unsaved selections. The native runtime stops monitoring and subsequent cloud requests, waits off the UI thread for the current journaled operation and then exits. A finishing-work overlay prevents more interactions while waiting. OS shutdown may interrupt requests; the existing durable operation journal reconciles interrupted outcomes next launch. Explicit Exit does not depend on a healthy registry read.

Upload pause is durable and independent of per-project policies. It blocks new uploads, store creation, attachment and metadata mutation, including the next guarded request in an already-running operation. Existing association polling and confirmed opt-out/cloud cleanup continue. Resuming does not enable any disabled or individually paused project. Local sources and cloud data are retained by pause.

Startup uses only the current user's Run registration, with a quoted executable path and `--background`. Registration and Windows' StartupApproved override are reported separately. ContextDock never writes the override or silently re-enables Windows-disabled startup. The standard startup-settings link lets the user manage that state. The installer removes this installation's owned startup entry on explicit uninstall; update/overwrite mode preserves it. Explicit uninstall/reinstall removes startup opt-in. Notification identity and protocol registration use the existing application identifier.

## Pending approvals

The inbox projects the existing `new_paths` discovery records, with excluded parent folders filtered out. Groups show relative path, type, size and availability. Batches are reviewed per project; nothing starts selected, and explicit selection plus preview confirmation is required. Approval removes only the selected file exclusions through the existing saved-selection transaction. Ignore keeps exclusions and records the decision while retiring those pending rows. Sources, snapshots and history are not deleted. Ignored files can be reselected through the folder tree later.

The review token binds selected paths, current pending metadata, project/folder state, selection revision and cloud-policy revision. Missing files cannot be approved. Stale selection, changed metadata, excluded parents, reassignment and removed projects require a fresh review. The decision receipt and pending-row updates commit with selection changes. Capture failures after a successful approval stay actionable as tracking errors rather than falsely reporting an unsaved approval. Tracking/cloud/automatic settings are never enabled by review. Existing high-confidence authored-version inheritance remains in place; ambiguous or unrelated new files stay pending.

## Notifications

Version 0.10.1 repairs first-send `ToastNotifier.Setting` failures (`0x80070490`), registers the per-user notification identity/logo, adds **Send test notification** in Settings and backs off failed/disabled submissions without marking them delivered. See [the installed test walkthrough](ContextDock_Tray_Testing_v1.md).

The native worker groups changes for 30 seconds and limits notifications to one per project/category every five minutes. Delivered signatures are persisted, avoiding unchanged reminders after restart. Resolved problems clear their watermark so a recurrence can notify again. Approval and problem notifications default on; routine success notifications are omitted. The foreground window uses in-app counts instead of toasts. Notifications contain project names and counts, without document filenames/content, and Windows suppression remains effective.

Windows toast XML uses protocol activation for narrowly validated `contextdock://review` or `contextdock://project` routes with UUID project IDs. Activation only navigates; it cannot approve, delete, upload or open arbitrary filesystem paths. Installed protocol activation routes to the existing instance through single-instance handling; cold launch reads the same route from arguments. Actual notification delivery and protocol registration depend on the installed application's Windows identity and shortcuts.

## Verification and installed acceptance

Automated tests use disposable registries, sources, native windows and uniquely named startup entries. They cover migration/history preservation, explicit batch consent, stale reviews, missing/excluded files, ignored decisions after restart, recognized versions, notification grouping/watermarks, pause/resume, confirmed removal while paused and native hidden monitoring/reopen/Exit. The Windows library test executable explicitly activates Common Controls 6 because Wry's native-control imports require it; packaged applications already include their normal Tauri resources.

Before claiming installed acceptance, verify this sequence on the delivered installer:

1. Open a project, change a folder checkbox and click X. Keep editing must remain available; failed Save must not hide the window. Save/Discard then hides; clicking the tray or shortcut restores the same application.
2. Edit an approved file while hidden, then create a new unrelated file. The edit follows saved sync policy; the new file remains excluded and appears in Pending approvals. After the grouping interval, click the Windows notification and review the correct project. Cancel must not approve; confirmation must affect only selected files.
3. Disable each notification switch, reverse it and verify saved readback. Check Windows notification suppression. Ignore a pending file, restart and confirm it stays excluded without a repeated notification. Check an excluded nested folder remains quiet.
4. Enable startup and check ContextDock in Windows Startup apps. Disable it in Windows and reopen ContextDock: the app must report that override and leave it disabled. Enable it in Windows, sign out/in and confirm hidden startup with a working tray; disable the app's registration and confirm no launch on the next sign-in.
5. Pause uploads during pending work: no subsequent upload/attachment should start, local tracking continues and approved removals can finish. Restart and verify the pause persists. Resume and confirm each project's original policy remains intact.
6. Exit with active work: the finishing overlay appears, no new requests start and pending work resumes after reopening. Check drive loss, offline recovery and tray failure messages. Verify overwrite/update preserves preferences and explicit uninstall removes only this installation's startup registration. No installed upgrade, sign-in cycle or authenticated provider notification acceptance is implied by fixture tests.

See [release verification](verification.md) for actual check counts, artifact details and remaining manual acceptance boundaries.
