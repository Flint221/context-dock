# ContextDock Google Drive setup and live acceptance

This supersedes the active setup instructions in v1. ContextDock now has its native connection, publication, recovery, simplified interface and optional recycling code. Automated and disposable native checks have passed; its app-owned Google connection still needs a Desktop OAuth client. The earlier chat connector's credentials cannot authorize the desktop app.

## Create the application client

1. Open [Google Cloud Console](https://console.cloud.google.com/) in the Google account that should own this application setup. Create a project named ContextDock, or select a suitable existing project. Keep all following settings in that project.
2. In APIs & Services → Library, enable **Google Drive API** and **Google Picker API**. Google's [desktop/mobile Picker guide](https://developers.google.com/workspace/drive/picker/guides/desktop-mobile-picker) requires Picker enablement for the browser-based folder chooser. This native integration does not need a JavaScript API key.
3. In Google Auth Platform, configure the application's branding as ContextDock, with your support/developer contact email. For a personal-account pilot, use an External audience in Testing and add the Google Drive account as a test user. Organizational accounts can have administrator restrictions; use an audience/account allowed by that organization.
4. In Data Access, declare only `https://www.googleapis.com/auth/drive.file`. This grants files created by ContextDock or explicitly authorized through the picker. The app does not request whole-Drive access, ChatGPT credentials or an OpenAI API key.
5. In Google Auth Platform → Clients, create an OAuth client with application type **Desktop app**, named ContextDock. Download its JSON to a private local folder outside this repository. Google's [Desktop OAuth instructions](https://developers.google.com/identity/protocols/oauth2/native-app) describe this client type, PKCE and the local-loopback redirect. Do not create a web/server client or manually substitute an API key.
6. Give the assistant only the local path to that downloaded JSON. Do not paste its contents into chat or commit it. Creation and consent take place in your Google account; no client or grant has already been created on your behalf.

## Connect the isolated pilot

Use the separate ContextDock Pilot build/profile prepared for acceptance, keeping the installed application closed or separate. The pilot uses a distinct application identity, marked development data directory and Windows Credential Manager target; it cannot open the installed profile. Its disposable project contains harmless Markdown and Word files. Native startup registration is disabled in the pilot.

In Settings, import the Desktop OAuth JSON, then choose Sign in with Google. Complete the Google account and consent screens yourself in the system browser. Return to the app, verify the displayed account and run Test Drive connection. Restart the pilot and verify that the same account remains connected without exposing tokens to the interface or SQLite.

Choose the existing root folder named ContextDock directly in My Drive through Settings. Under the pilot project's Drive publishing tab, enable publication and choose its existing matching project subfolder. If it has no project folder, use Create Drive folders explicitly. A narrow `drive.file` search cannot establish that an unselected folder is absent; the app must not create a second folder on that assumption. Once remote files are mapped, switching to a different folder is refused to preserve their identities.

## Live pilot sequence

Keep Optional local cleanup off until publication acceptance passes.

1. Publish the disposable Markdown and Word families. Verify one Drive file per family, original binary file type, exact downloaded bytes and persisted project/file IDs.
2. Save a new numbered version with an unpredictable harmless marker. Verify that the same cloud file ID receives its new filename and bytes, with one visible file for that family.
3. Test simultaneous numbered arrivals, a move, a rename, offline recovery, interrupted publication/restart, expired/revoked authorization and pause/resume. Check cloud contents and local receipts rather than relying only on a success message.
4. In a fresh ChatGPT web conversation, select the updated connected-Drive file and ask for the new marker. Record provider-write and successful readback times. Report an observed upper bound when the actual visibility time is unknown. This is separate from the earlier connected-tool test and is not proof of a fixed sync interval.
5. With live publication accepted, enable recycling only for the disposable project. Verify that its unchanged lower numeric version reaches the Windows Recycle Bin, its newest working file stays local, and a changed older file is retained. Check actual battery-saver transitions while publication is queued. Longer battery discharge and large-project measurements remain separate from the short AC-powered resource samples.

Drive publication is one-way from local files. Cloud edits trigger a conflict rather than an automatic import or silent replacement. Failed verification retains older files. Restoring an item from Windows Recycle Bin returns it to `.contextdock/recycle/<operation ID>/<original filename>`; move it back to its desired original project location when needed. Interrupted cleanup restores known safety files when their original paths are free; uncertain or occupied paths require review.

The branch remains a draft until these live checks are recorded. The installer has not been applied to the real profile, existing saved material has not been deleted and recycling has not been enabled for real projects.
