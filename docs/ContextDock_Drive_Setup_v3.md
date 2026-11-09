# ContextDock 0.11.0 setup

Use the installed **ContextDock**, rather than the retired pilot. Version 0.11.0 is installed with a fresh registry and no projects. The Desktop OAuth client has already been imported; the expected initial Google connection status is **Ready to connect**. The previous pilot sign-in does not connect this production profile.

This guide supersedes the onboarding steps in [setup v2](ContextDock_Drive_Setup_v2.md), which remains as the historical OAuth/pilot guide. See the [release audit](ContextDock_App_Audit_v8.md) and [fresh-profile record](ContextDock_Local_Reset_v3.md) for completed checks and retained backups.

## Connect Google Drive once

1. Open ContextDock from its desktop shortcut or Start menu, then open **Settings**.
2. Under **Google Drive connection**, click **Sign in with Google**. Complete consent with the Google account containing your ContextDock Drive folder, then return to the app. Confirm **Connected** and the intended email address.
3. Click **Test Drive connection**. This verifies access without uploading documents.
4. Click **Choose existing ContextDock folder** and select the existing root named `ContextDock` in My Drive. This root will contain the four project subfolders.

If Settings instead says **Setup needed**, use **Choose OAuth client JSON** and select `C:\Users\Mihir\Downloads\ContextDock_OAuth_Client_v1.json`, then sign in. Keep the JSON outside the repository. If Google reports that access is limited to approved testers, confirm that the signing-in account is listed in the OAuth project's test users, as described in setup v2.

## Add the four consolidated projects

Start with one project so its local selection and cloud destination can be checked before repeating the process.

1. On **Projects**, choose **Add project**, give it a recognizable name and browse to its existing consolidated local folder. Register the working project folder, rather than an old archive or a backup.
2. Open **Files & tracking** → **Choose files and folders**. Select the consolidated Markdown and Word documents you want to maintain, exclude unrelated folders and click **Save changes**.
3. Enable **Monitor selected files**. Check **Latest documents** for the expected document families. Numbered files such as `CattleTools_Context_v1.md` and `CattleTools_Context_v2.md` belong to one family when their naming matches.
4. Open **Drive publishing** and enable **Publish this project to Google Drive**.
5. Click **Choose existing Drive project folder** if that project already has a destination, such as `ContextDock/Cattle Tools`. Otherwise click **Create Drive folders** to create the project destination under the selected root. Select the project subfolder, not the root itself, as this project's destination. The folder choice authorizes an existing folder; it does not automatically adopt every pre-existing cloud file as a tracked document.
6. Click **Publish latest**. Wait for each document to report **synced** here or **Up to date** in the file list. Check that the intended Drive project folder contains the expected latest files.
7. Under **Optional local cleanup**, leave **Recycle unchanged older local versions** unchecked during initial setup.
8. Repeat for the other three projects. No old database or archive import is needed.

Choosing a destination can start automatic publication once the project is enabled and its files are ready. Review the local selection before enabling publication. If a conflict or duplicate needs attention, use the displayed error rather than repeatedly creating folders or resetting the profile.

## Normal use

Keep editing the local documents. Stable saves publish automatically after a short settling period; the five-minute metadata reconciliation is a fallback for missed filesystem events, not the normal upload interval. Successful replacements update the same Drive file ID and filename. The newest working file stays local.

Keep ContextDock running in the tray. Windows startup registration is already enabled. Offline work remains queued and retries after reconnection. A manual pause or Windows energy saver can hold cloud writes while local tracking continues. New or ambiguous documents may require a decision under Pending approvals.

Drive publication is one-way from local documents. Editing a published cloud copy can produce a conflict that needs review. If a whole local project folder moves, use its **Reassign folder** control instead of adding a duplicate project.

Recycling can be enabled later per project. It requires a fresh verified Drive replacement and an unchanged, lower numbered local version of the same family. Changed or uncertain older files are kept. See the [recycling safeguards](ContextDock_Drive_Recycling_v1.md), including the safety-folder path used when restoring from Windows Recycle Bin.

Your existing ChatGPT Drive connection remains a separate consumer of these Drive files. The desktop app does not need Ask AI, vector-store indexing or a ChatGPT API key.
