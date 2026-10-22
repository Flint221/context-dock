# ContextDock cloud cleanup v1

October 22, 2026 (America/Chicago). This records the user's explicit instruction to delete ContextDock cloud data so documents can be uploaded again, while preserving files on disk. It supersedes the earlier investigation's hold on cloud mutations for this scoped cleanup only. It does not authorize deleting unrelated OpenAI resources or treating the damaged local registry as repaired.

## Verified outcome

Deleted and independently verified absent through resource GETs and fresh paginated listings:

- Three ContextDock vector stores: PV-GIN, Cotton Trax and Cattle Tool.
- Seventeen ContextDock uploaded Files and their seventeen index associations.
- Zero resources with the verified ContextDock installation identity remain in the accessible vector-store/File inventories.

Two unrelated Course Assistant vector stores and fourteen course PDFs remain. Their resource IDs, store names/metadata and File names/sizes/purposes were independently checked against the pre-cleanup inventory and match. No other cloud application data was targeted.

Forty known existing local source/archive files were SHA-256 inventoried before and after cleanup; all are unchanged. The original database, WAL, SHM, all 57 snapshots and staging inventory also exactly match the investigation's entry fingerprints. No source file, archive or snapshot was deleted, moved or rewritten. Both earlier evidence copies and the original malformed registry are retained. This does not claim that every arbitrary file in every project directory was rehashed; the cleanup script performed only cloud GET/DELETE operations and wrote task evidence under its separate investigation directory.

## Ownership and execution checks

A single installation identity was independently reconstructed consistently from all 22 surviving cloud-job remote filenames, with project/version/job UUID components checked against the saved job records. Seven recovered cloud-project records provided known project and store-operation identities. The saved Windows credential was used in transient memory only; it was not printed, persisted to a report, placed in command arguments or changed in the Windows vault.

Before the first mutation, the cleanup fetched complete paginated cloud inventories. Target stores had matching installation/project/operation metadata; known saved store IDs were checked against their operation identity. Target Files had the app's exact structured filenames, known project IDs and assistants purpose. Each target store association carried the expected installation/project attributes and pointed to a verified target File. No target File was associated with either unrelated vector store.

All target resources were re-read before deletion. Uploaded File identities were checked by ID, filename, size and purpose; stores by ID and metadata. Each deletion intent and result was written to a task-local receipt. Each deleted File/store returned a matching deletion acknowledgement and was subsequently verified absent. Fresh inventories confirmed zero matching ContextDock resources and preservation of the unrelated resources. No ContextDock process was running at cleanup entry. No GUI, application launch, installation, new upload or replacement index creation occurred.

Both uploaded Files and vector stores were removed. Merely removing a vector-store association does not delete its underlying uploaded File; see the official [vector-store File deletion reference](https://developers.openai.com/api/reference/resources/vector_stores/subresources/files/methods/delete), [File deletion reference](https://developers.openai.com/api/reference/resources/files/methods/delete) and [vector-store deletion reference](https://developers.openai.com/api/reference/resources/vector_stores/methods/delete). The OpenAI Docs skill was used to verify these operations.

## Evidence and remaining work

Private resource inventories and per-resource receipts are retained under `%LOCALAPPDATA%\ContextDock_Investigation_v1` in the versioned ContextDock Cloud Local Inventory, Delete Plan, Delete Receipts, Delete Result, Source Preservation and Unrelated Cloud Preservation files. Credentials are not included. The task-local API helper is outside the repository and contains no key.

The application database is still malformed and has not been reset or replaced. Cloud cleanup cannot restore its missing pages or local history metadata. Old cloud IDs in the preserved database now refer to deleted resources and must not be used as evidence that cloud data still exists. A fresh operational registry or separately approved recovery remains necessary before re-registering projects and re-uploading. Keep the damaged registry and snapshot evidence so a later history recovery remains possible.

No application code changed, so the preceding investigation's passing 197 native/131 frontend checks remain the latest code validation; no new test or installer claim is made by this operational cleanup report.
