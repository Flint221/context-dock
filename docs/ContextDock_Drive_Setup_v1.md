# ContextDock Google Drive connection

This is step 1 of the latest-file publication work. The app now has its own native Google sign-in flow. The earlier connected-tool acceptance test remains separate evidence in [the Drive test report](ContextDock_Drive_Test_Report_v1.md).

## One-time Google setup

1. In your Google Cloud project, enable the Google Drive API.
2. Configure Google Auth Platform branding and audience for ContextDock. If the app is in testing, add the Google account used for the pilot as a test user.
3. Declare only `https://www.googleapis.com/auth/drive.file` for this workflow.
4. Create an OAuth client with application type **Desktop app**, named ContextDock. Download its client JSON to a private local location outside the repository.
5. In ContextDock Settings, choose **Choose OAuth client JSON**, select that file, then choose **Sign in with Google**.
6. Complete sign-in in your default browser. Return to ContextDock and confirm the displayed account. Use **Test Drive connection** to check access without uploading a file.

Google's [desktop OAuth documentation](https://developers.google.com/identity/protocols/oauth2/native-app) covers API enablement, Desktop client creation and the browser/loopback flow. Its [Drive scope documentation](https://developers.google.com/workspace/drive/api/guides/api-specific-auth) explains that `drive.file` permits files created by the app or explicitly opened/authorized for it. It does not grant automatic access to every folder created by the earlier test's separate connector. Reusing such a folder needs an explicit supported authorization path; the app must not silently claim it can see all of My Drive.

The OAuth client JSON is configuration for this application, not a ChatGPT subscription credential. Keep it out of chat and source control. No existing OAuth client or app-owned Google authorization has been supplied or verified in this development session yet.

## Connection behavior

- The native backend opens the system browser, binds a random port on `127.0.0.1`, and validates a fresh OAuth state and PKCE challenge. It does not embed Google's login page.
- Sign-in can be cancelled and expires after three minutes. Invalid or stale callbacks cannot complete it. Authorization codes and tokens are excluded from error messages.
- The imported client and saved sign-in stay in Windows Credential Manager under a separate ContextDock Google Drive entry. They are not returned to React, saved in SQLite, or placed in browser storage. The imported JSON's endpoint fields are ignored; requests use fixed Google endpoints.
- Initial connection requires offline file access and a successful Drive account lookup. Access tokens refresh before expiry; refresh-token rotation is saved before the new access token can be used.
- Disconnect clears the saved sign-in locally before requesting Google revocation. If Google is unreachable, the app still remains disconnected and reports that remote revocation was not confirmed. Cloud files and the OAuth client configuration are retained.
- Account status reads do not perform periodic network requests. Settings polls only while a sign-in is pending. This step adds no background upload worker.

## Verification boundary

Automated tests cover PKCE, callback validation and a real local HTTP callback, cancellation/timeouts, scope rejection, account verification, refresh-token rotation, vault failure, offline disconnect, client replacement guards, and frontend success/failure feedback. The complete existing frontend and Rust suites, formatting, lint, type checks and build are used as the per-step regression gates.

Step 1 results: 172 frontend tests passed; 278 Rust tests passed with six existing opt-in device/integration tests ignored. The new coverage includes 13 native OAuth tests and five connection-interface tests. TypeScript checks, ESLint, the production frontend build, Rust formatting, and Clippy with warnings denied passed. The six ignored tests do not establish native tray, startup, external-drive or live-cloud acceptance.

Live sign-in, Windows vault persistence across an actual app restart, and account revocation still require the application's own Desktop OAuth client and an interactive Google grant. Passing automated tests is not evidence those live acceptance checks have happened. Later publication and recycling steps must remain independently verified before their acceptance is marked complete.
