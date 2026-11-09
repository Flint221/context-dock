# ContextDock latest-file publication

This records step 2 of the six-step Drive implementation. It extends [the native connection setup](ContextDock_Drive_Setup_v1.md); it does not replace the separate [provider acceptance test](ContextDock_Drive_Test_Report_v1.md) with a claim of live app acceptance.

## Current behavior

The project Cloud tab now includes Google Drive publication. Enabling it saves consent and account binding without uploading. **Publish latest** sends the current stable selected document for each tracked family. Publication remains manual at this stage; automatic scheduling and recovery are step 3. Existing OpenAI and archival interfaces remain until the dedicated simplification step.

Schema 14 adds an account root mapping, a project folder mapping, one cloud file mapping per document identity, last verified hashes/revisions, and pending publication intents. It preserves existing registrations, source IDs, snapshots, preferences and cloud settings. Google tokens and upload session URLs never enter these tables.

## Destination setup

The app requests the narrow `drive.file` scope. Files created through a different connector are not automatically authorized for this desktop client. The current [Google native Picker guide](https://developers.google.com/workspace/drive/picker/guides/desktop-mobile-picker) describes a system-browser picker integrated with desktop OAuth. This app uses that route; the Google Picker API must also be enabled in the client project for this optional selection flow.

After connecting, **Choose existing ContextDock folder** in Settings opens the Google picker for one folder. The backend verifies the returned selection, account, name, folder type and location directly under My Drive before recording it. It rejects selecting a different account or replacing a root already used by mapped project folders.

Alternatively, **Create Drive folders** in the project panel explicitly authorizes creating the root and project folder. It first reuses an accessible matching folder. Multiple matching names require review. If no root is accessible, ordinary publication stops until the user chooses an existing folder or explicitly creates one. A restricted search returning no result is not treated as proof that no similarly named folder exists elsewhere in the account.

Project publication reuses the saved root and project IDs. It never silently replaces a previously verified folder or file that is now missing or inaccessible.

## Replacement and verification

1. Identify the tracked document's current stable source and confirm saved path selection, project readiness and account binding.
2. Reserve a provider file ID and save it locally before any remote file creation. Folder IDs follow the same ordering. [Google documents pre-generated IDs](https://developers.google.com/workspace/drive/api/guides/create-file) for safe retry of supported file/folder creation.
3. Stage a bounded temporary copy, compare its SHA-256 and size with the tracked version, and save the pending generation before uploading.
4. Use [resumable upload](https://developers.google.com/workspace/drive/api/guides/manage-uploads) to create or update the mapped Drive item. Metadata includes the new filename and app-private document identity markers. Original DOCX bytes remain DOCX; Markdown remains Markdown.
5. Retrieve metadata, stream the complete cloud download, compare its SHA-256, and retrieve metadata again. Matching file ID, parent, identity markers, filename, size and stable provider version are required.
6. Save and verify the upload receipt before clearing its temporary intent. A newer local save can retain the verified prior receipt for recovery while leaving the document queued. It cannot report that stale generation as synced.

Renames and moves retain the tracked document mapping; increasing a filename version updates the same Drive file. Identical bytes do not justify another cloud copy. A lost upload response is recovered by checking the reserved file ID and pending content, including after reopening the database. A new upload session may be opened on retry, but it targets that same item.

Every native outbound request rechecks the connected account and project authorization. Writes also check global upload/battery pause and the expected local generation. HTTP does not hold the main registry mutex. Upload and readback use file streams and fixed-size buffers rather than loading the whole document into RAM.

Cloud content changed outside ContextDock, mismatched identity markers, missing previously verified items, incomplete verification and unsaved database intents stop publication. No cloud file or local source is deleted by this step. Conditional request headers are supplied when a provider ETag is available; concurrent multi-machine publication is not established by this test suite.

## Validation and remaining acceptance

Sixteen publication tests cover same-ID replacement, simultaneous versions, rename/move identity, exclusions, lost responses and restart recovery, corrupted readback, newer saves during upload, foreign identity, external edits, missing remote items, destination reuse/ambiguity, explicit root setup, account binding, migration from schema 13, and ignored database writes. Native OAuth coverage also checks the picker URL and returned folder identity. Five publication-interface tests cover consent without upload, explicit folder setup, failed replacement feedback and verified success reporting.

The full frontend suite passes with 177 tests. The full Rust suite passes with 295 tests and six existing opt-in integrations ignored. TypeScript checks, ESLint, the production frontend build, Rust formatting and Clippy with warnings denied pass. Passing these automated checks does not establish a live Google OAuth grant, picker interaction, or app-owned upload.

The user confirmed there is no existing ContextDock OAuth client. Live app sign-in and the end-to-end Google Drive/ChatGPT pilot remain pending that configuration. The adapter, durable intents, recovery checks and interface are implemented; automatic scheduling, full interface retirement, resource measurements and optional recycling remain subsequent work. Existing historical files, chats and cloud resources are retained.
