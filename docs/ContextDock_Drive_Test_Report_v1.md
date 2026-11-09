# ContextDock Google Drive replacement test

Document version: 1. Tested November 8, 2026, in America/Chicago (UTC timestamps fall on November 9). This records an isolated acceptance test for the direction proposed in [app audit v2](ContextDock_App_Audit_v2.md).

## Result

The requested isolated test passed: one visible cloud file, verified replacement under the same Google Drive file ID, and a fresh ChatGPT web conversation reading the updated contents through the connected Google Drive source.

Google Drive is the selected route to evaluate for latest-context publication. This result supports that direction; it does not implement it in ContextDock. No application code, settings, database, installed version, tracking behavior, or user project documents were changed during this test. The earlier Space upload/readback success was reported by the user and was not repeated here.

## Test resources

The folder lookup found no existing ContextDock folder directly under My Drive, so the test created the root and a dedicated project subfolder. No real project folder was used.

| Resource | Name or ID |
| --- | --- |
| Root | [ContextDock](https://drive.google.com/drive/folders/1kMrZMpFI8Al1GIC0yGv7KqpMjwVrIg6o) |
| Root folder ID | `1kMrZMpFI8Al1GIC0yGv7KqpMjwVrIg6o` |
| Isolated project | [ContextDock / Drive API Test](https://drive.google.com/drive/folders/1TZ0yzrXg8x5U0u7hr7Hxaw7HfcdXbScT) |
| Project folder ID | `1TZ0yzrXg8x5U0u7hr7Hxaw7HfcdXbScT` |
| Cloud file | [ContextDock_Drive_Test_v2.md](https://drive.google.com/file/d/1eLuPShhWIXYZ7V2gfQxY4UnmxIQ1geyZ/view) |
| Cloud file ID before and after replacement | `1eLuPShhWIXYZ7V2gfQxY4UnmxIQ1geyZ` |
| Local fixtures | [v1](ContextDock_Drive_Test_v1.md), [v2](ContextDock_Drive_Test_v2.md) |

The samples contain disposable test text and random verification markers, with no credentials, personal data, or project content. The cloud file remained unshared. No public-link permissions were enabled.

## Drive API verification

The connected Google Drive tools performed provider API operations directly, independently of ContextDock. The test uploaded the local v1 Markdown file, verified its metadata and readable contents, then used a file update with the same ID to replace both its bytes and filename with v2. The update did not create a second file. This follows the provider's [files.update API](https://developers.google.com/workspace/drive/api/reference/rest/v3/files/update).

| Check | Observed result |
| --- | --- |
| v1 creation | `2026-11-09T00:43:42.653Z`; name `ContextDock_Drive_Test_v1.md`; 311 bytes |
| v1 content readback | Declared version 1; marker `CDRIVE-V1-4101137C9D354AEC`; value `copper meadow` |
| v2 modification | `2026-11-09T00:44:08.934Z`; name `ContextDock_Drive_Test_v2.md`; 364 bytes |
| Identity | Same cloud file ID and same project parent folder |
| Raw v2 download | Completed at `2026-11-09T00:45:51.549Z`; all 364 bytes matched the local v2 fixture |
| Folder listing after replacement | Exactly one visible entry, the v2 file with the original ID |
| Final folder check | Still one entry at approximately `2026-11-09T00:46:45Z` |

SHA-256 of local v1:

```text
2ebfb4aebf7ae624f3c2dd3869c0ee824fc53d3e03785fe0a30c799e43ce9f6e
```

SHA-256 of both local v2 and the complete raw Drive download:

```text
c4ea5e1ca84c8f4e120001a2193a23f2c347cd68ffadec3cf1f2246413cd0ed9
```

The raw download was compared byte for byte as well as hashed. Matching metadata or a filename alone was not treated as content verification. Temporary authenticated download URLs are intentionally omitted from this record.

## Fresh ChatGPT connected-file readback

A new regular ChatGPT web conversation was opened. Through **Add from library**, the source filter was set to Google Drive and the picker searched for `ContextDock_Drive_Test`. It returned one matching v2 item. That connected file was selected and added to the chat; the local fixture was not uploaded to ChatGPT.

The prompt was submitted at approximately `2026-11-09T00:46:18Z`:

> Read only the connected Google Drive file attached to this message. Report its declared version, exact verification marker, and sample value. If you cannot read the contents, say so; do not infer the contents from the filename.

The expected marker and sample value were not included in the prompt. ChatGPT returned:

| Field | Returned value |
| --- | --- |
| File | `ContextDock_Drive_Test_v2.md` |
| Declared version | `2` |
| Exact verification marker | `CDRIVE-V2-38FB2436F5B34EFD` |
| Sample value | `silver orchard` |

These match the downloaded v2 contents. The unpredictable marker provides evidence beyond inferring a version from the filename.

[Readback conversation](https://chatgpt.com/c/6af117d4-05b4-83e9-a09c-7352f5a50cc1) (requires the user's ChatGPT account). A screenshot was saved locally as `ContextDock_Drive_Readback_v1.jpg` in the Codex visualization workspace. It is excluded from the repository because its sidebar includes unrelated chat titles.

## Observed delay

The v2 update completed at **6:44:08.934 PM Central**. The connected-file picker found the v2 filename on its first search, around 6:46:03 PM. The blind content request was submitted around 6:46:18 PM. The completed, correct response was observed before the clock check at **6:46:54 PM Central**.

Therefore updated content was readable by ChatGPT within **2 minutes 46 seconds** of the Drive update. This is a conservative upper bound that includes navigation and response generation. There was no observed stale-content response or failed search. The test did not continuously poll from the update, so it does not establish the earliest availability time, a fixed sync interval, or a guaranteed freshness bound.

## What this establishes for the proposed app

Keep the core tracking and version-family behavior described in audit v2. For optional cloud publication, updating one Drive item per document identity is a viable route: v1 can become v2 without changing the link or producing another visible cloud file. ChatGPT can select that connected item and read its updated contents in a new conversation.

The implementation plan should retain project folder IDs, cloud file IDs, stable document identity, and last verified content hashes. It should update the existing item, verify the matching generation, then mark it synced. Failed uploads or verification should remain queued. Local moves and filename changes should update the mapping rather than create another cloud item. Recycling older local versions remains optional and gated on verified replacement.

This single small Markdown test does not validate app-owned OAuth setup, restart-safe mappings, interrupted uploads, retries, concurrent updates, DOCX behavior, ambiguous version families, automatic ChatGPT refresh in an already open conversation, or old-file recycling. No cloud file was moved to Trash, no older local file was recycled, and no retention policy was changed. The visible folder count does not imply that Drive's internal revision history was removed.

The test folders and one cloud v2 sample remain available for inspection. Both local fixture versions remain in the repository. ContextDock implementation remains unchanged pending the next implementation decision.
