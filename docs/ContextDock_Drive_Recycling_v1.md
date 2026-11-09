# ContextDock optional older-file recycling

Step 6 adds a per-project choice that is off by default. It applies only after successful latest-file publication and a second, fresh remote metadata/download/metadata verification. A saved synced label alone cannot authorize cleanup. The newest local working file stays in place and is pinned against concurrent writes/deletion while older files are considered.

Eligible files must belong to the same tracked family, have the same normalized filename stem, carry a strictly lower numeric filename version, remain selected and available, and match their previously tracked SHA-256 and size. Unnumbered files, ambiguous families, pending matches, changed older files, changing newest files, excluded paths and unsupported volumes are retained. Default automatic recycling is limited to Windows fixed NTFS volumes.

## Guarded transfer and recovery

Schema 17 journals the original source/document IDs, root, original location, expected hash, physical file identity, current generation and safety location before any source move. Windows handle-based rename moves the verified older file without replacement into a unique directory under `.contextdock/recycle`, keeping its original filename. The source is never copied into a permanent ContextDock archive.

Immediately before transfer, the app rechecks publication/tracking settings, account and generation, global resource pause, stopping state and the recycling preference. Turning recycling off remains available during a manual publication. A failed check after the safety move restores the original file when its path is free and its identity/hash match. A bin transfer failure follows the same restoration path. Occupied original paths are never overwritten; the safety file is retained and an independent cleanup warning appears without invalidating a verified cloud publication.

The native transfer uses Microsoft's [ITransferSource::RecycleItem](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-itransfersource-recycleitem), which returns the recycled item. It does not call permanent deletion or fall back to DeleteItem/RemoveItem. The adapter denies writes during the Shell transfer and verifies the returned Recycle Bin file's physical identity and contents before recording success.

On startup, unfinished prepared/quarantined/bin-pending intents are restored if the known safety file is present and the original path is free. If the operation's final location cannot be established, it is marked for attention and no deletion is retried. A changed project root, changed safety file or original-path collision keeps available bytes for review. Source metadata and cloud IDs remain intact. A successfully recycled older source is hidden from the current-file list; it is not mistaken for a newly missing working file.

Restoring through Windows Recycle Bin returns an item to its safety-folder location. Move it from `.contextdock/recycle/<operation ID>/<original filename>` back to the desired project folder when needed. The journal retains the original location. This safety location is intentionally excluded from monitoring and automatic publication.

## Validation and acceptance

Eight new backend tests cover default opt-out, newest-file preservation, fresh cloud readback, changed older files, unsupported volumes, failed bin transfer, restart after a safety move, occupied-path recovery, post-quarantine opt-out, and a newest file edited after its upload. Interface tests cover explicit opt-in, saved-state verification and disabling recycling during publication; cleanup errors remain separate from publication errors.

Two opt-in native checks were explicitly run: a disposable file reached the actual Windows Recycle Bin with matching contents, and the complete guarded cleanup engine using simulated Drive retained its newest local file, recycled only the older fixture and recorded the verified bin receipt. This is actual Windows recycling acceptance, with a simulated cloud provider. No user's project files were recycled.

The full frontend suite passed with 188 tests and the full Rust suite with 320 tests; eight integrations are excluded from the default Rust run, including the two recycling checks run separately. ESLint, TypeScript/production frontend build, Rust formatting and Clippy with warnings denied passed. Live app-owned Google publication, folder-picker acceptance and the complete live-cloud-to-recycling chain still need ContextDock's new Desktop OAuth client and interactive consent. No production installer has been installed and recycling remains disabled for real projects.
