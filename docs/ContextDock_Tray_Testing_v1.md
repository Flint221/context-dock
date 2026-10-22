# ContextDock tray, startup and notification checks

Use the installed **0.10.1 or later** application. Before updating, right-click the ContextDock tray icon and choose **Exit ContextDock**. Install the update over the existing installation, then open it normally. Updating does not reset projects or change startup consent.

## Why 0.10.0 showed Element not found

The native worker checked `ToastNotifier.Setting` before its first `Show` call. Windows can return `0x80070490` for that permission query before it has initialized a new notification sender. A disposable Windows integration test reproduced that exact error. Registration alone did not remove the first-send exception; allowing the specific not-yet-initialized response through to `Show`, then checking the setting again, successfully submitted the notification and retained it in Notification Center.

Version 0.10.1 also registers the per-user display name, logo and stable protocol-only activation identity at installation and before delivery. It does not change Windows notification permissions. Other API failures remain errors with the failing stage identified. Failed or disabled submissions do not consume the notification watermark, retry no more than once per minute per project/category and do not block other projects. Resolved, disabled-category or successfully retried notification errors are cleared.

Microsoft describes the permission-query exception in [How can I tell whether the user has disabled toast notifications for my app?](https://devblogs.microsoft.com/oldnewthing/20200810-00/?p=104058). Registration follows the per-user identity pattern in [Windows App SDK notification utilities](https://github.com/microsoft/WindowsAppSDK/blob/main/dev/AppNotifications/AppNotificationUtility.cpp). Alerts use protocol activation and cannot approve or upload documents.

## 1. System tray

1. In Settings, enable **Keep running when closed**. Click the window's X. The window should disappear while ContextDock remains under the tray's hidden-icons arrow near the clock.
2. Click its tray icon to reopen the same window. Right-click it and check **Open ContextDock**, **Pending approvals**, **Pause uploads**, **Settings** and **Exit ContextDock**.
3. Change a folder checkbox without saving and click X. Resolve the unsaved-changes prompt before the window hides. Cancel must keep editing available.
4. Choose **Exit ContextDock** from the tray. The icon should disappear after current work finishes. Reopen from Start. Turning **Keep running when closed** off makes X fully exit instead.

## 2. Windows startup

1. Enable **Start with Windows, hidden in the tray** in ContextDock Settings.
2. Click **Open Windows startup settings** and confirm ContextDock appears and is enabled. Task Manager's Startup apps page is another place to inspect it.
3. Save your other work, then sign out and sign in again, or restart Windows. ContextDock should start hidden with its tray icon present. Clicking the icon should open it.
4. Disable ContextDock in Windows Startup apps and reopen ContextDock Settings. It should report the Windows override without changing it. Re-enable in Windows to reverse that choice.
5. Turn startup off in ContextDock. The registration should be removed, and it should not start on the next sign-in. Closing to the tray alone is not a startup test.

## 3. Notification delivery and clicking

1. In ContextDock Settings, click **Send test notification**. This explicit test works while the window is focused and does not depend on project files or cloud access.
2. Look for a ContextDock banner. Press **Windows + N** to inspect Notification Center if the banner is hidden. Windows Do not disturb may suppress banners. Enable ContextDock under **Windows Settings > System > Notifications** if the app reports notifications are disabled.
3. Click the test alert. It should open **Pending approvals**, including when ContextDock is hidden. It must not approve anything.

## 4. Actual new-file approvals

1. Use a disposable test project with tracking enabled. Cloud sync can stay off to test discovery and approvals without uploads. Leave **New files awaiting approval** enabled.
2. Close ContextDock to the tray. Create a genuinely new Markdown document in a selected project folder, for example `ContextDock_Approval_Test_v1.md`. Do not use a new version of an already approved document or an excluded folder for this check.
3. Allow discovery plus the approximately 30-second grouping interval. The tray Pending approvals count should rise and a grouped notification should appear. Alerts for the same project/category are limited to one every five minutes; an unchanged delivered group does not repeat after restart.
4. Click the notification. The correct project group should open. The new file must remain unselected and unapproved. Selecting it opens a confirmation step; Cancel must leave it pending. Confirm approval only when desired. Uploading also requires the project's cloud settings to allow it.
5. Create another unrelated test file and **Ignore** it in the inbox. Restart ContextDock and confirm it stays excluded. Turn the approval notification switch off and repeat with a new file: the inbox should still populate, but there should be no approval toast. Turn the switch back on to restore alerts.

For background upload acceptance, separately edit an already approved test document while hidden with cloud sync on. It should follow the saved sync policy. Pause uploads from the tray, make another edit and confirm no new upload starts; Resume should allow it again. Local tracking continues during pause.

Local automated checks can verify the first-send error, Windows notification history, disposable startup registration, tray lifecycle and approval consent. A real sign-in cycle, visible installed banners and clicking installed notifications still require these user checks.
