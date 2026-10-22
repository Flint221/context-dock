# ContextDock archive restore v1

October 22, 2026. The user requested restoration of older archived documents to their original main project directories, with original names/version labels, and required advance notice before any content-based manual classification. Metadata was sufficient for every archived file; no manual content classification was performed.

## Metadata basis

The preserved recovery database's archive-operation payloads, source-file records and complete version metadata established exact original relative paths for all 11 files in the three Old Context Files trees. Every operation was a recorded source move rather than an in-place snapshot copy. Archive bytes matched the recorded SHA-256 and the preserved immutable snapshot. All 11 source metadata records were also present and agreed with their version paths. The entire archive inventory matched the plan, and every original destination was absent.

Restored files:

| Project | Original filenames restored to its main directory |
| --- | --- |
| Cattle Tools | RAP_Ingestion_Context_v1.md; RAP_Ingestion_Context_v2.md; RAP_Master_Context_v4.md; RAP_Master_Context_v5.md; RAP_Raster_Analysis_v4.md; RAP_Raster_Analysis_v5.md |
| CottonTrax | CottonTrax_Architecture_v1.md; CottonTrax_Master_Context_v1.md; CottonTrax_Sync_MVP_v1.md |
| PV-GIN | PV-GIN_Dev_Doc_v1.docx; PV-GIN_Dev_Doc_v2.docx |

The existing project roots were used: Boyd/Cattle Tools, Lara/CottonTrax and Wyatt/PV-GIN under the user's D-drive AgriLife directory. No document was renamed based on its prose, given a guessed version number or exported over an existing file. Filenames requested by the user and established source metadata were preserved exactly.

## Execution and verification

ContextDock was not running. All relative destinations were checked to remain within the intended project root; reparse points were refused. Before moving, all 11 source files were backed up under `%LOCALAPPDATA%\ContextDock_Archive_Restore_v1\Archive_Backup` with their prior archive directory structure retained, and every backup hash verified. Read handles denied concurrent writes while allowing validation and rename. Windows non-overwriting rename returned each original source to its metadata-defined location. Intent and success receipts were persisted per file, with rollback specified if verification failed.

The first preflight used an incompatible sharing mode for validation reads and stopped before any move. Its empty receipt is retained. The corrected read-lock mode was then used; no document content changes were needed and no file was lost or overwritten.

Final results: 11 restored files, all restored hashes and all backup hashes match; all original archive-source paths are absent. The 20 Markdown/Word documents already present in the main directories retain their original hashes. Only the exact generated family directories from the plan and then the three Old Context Files roots were removed using empty-directory-only operations, after verification. Ten empty directories were removed; no directories containing files were recursively deleted. Existing database, snapshot and earlier backup stores were not rewritten, and no cloud request was sent.

The task-local plan, backup, main-file baseline, receipts and final verification are retained in ContextDock_Archive_Restore_v1. No application code or installer changed. This restores source organization; it does not repair the previously damaged registry or alter the application's archive naming format. Re-enabling automatic archival later can legitimately move verified older exports out of the main directory again.
