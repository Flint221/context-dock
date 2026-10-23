# ContextDock exFAT archive copies and attention details

## Installed investigation

Version 0.10.4 was running. A stable read-only copy of its actual open database/WAL passed quick_check. Weather Tool v1 was marked removed and v2 indexed/completed, with no project cloud error. CottonTrax had eleven desired indexed jobs and four obsolete removed jobs, with no remaining cloud error. Its transient removal-verification notices were followed by successful removal activity.

The two remaining CottonTrax issues were blocked archive copy operations for CottonTrax_Master_Context_v3.md and CottonTrax_Team_Tasks_v2.md. Both used the same source for old and new content: these preserve prior edit snapshots, not moves of the current source. The D: volume reports healthy exFAT. Archive copy publication previously required a hard link, unsupported on that filesystem, causing Windows error 1.

## Repair

On Windows archive copies now publish a flushed, hash-verified staging file through the existing handle-based no-overwrite rename. Before publication the operation journal durably records the staging filesystem identity in the copy plan's previously unused old_identity field. Recovery accepts a completed publication only with a matching receipt and hash; legacy hard-link publications remain recoverable. An unavailable journal prevents publication. Collisions remain blocked and sources/snapshots are never removed. Non-Windows publication retains its prior hard-link behavior.

The project overview separately counts failed archive/file operations and names their destinations. Pending operations also show their destination paths. No schema migration is required.

## Verification and acceptance

An explicit Windows regression published/recovered an archive in a disposable directory on the actual exFAT drive, verified preservation of the snapshot and refused a destination without ownership evidence. Other regressions cover interrupted staging/publication, journal failure, collisions and existing archival behavior. UI checks identify both failed destinations and the retry action.

The user's original application database, files, snapshots, archives and backups were read only during this investigation. No cloud API calls or cleanup were performed by the investigation. After installing 0.10.5, open CottonTrax > Files & tracking and use Retry pending operations. The two saved copy operations should complete; confirm the overview attention count returns to zero. The old staging files from unsuccessful attempts are retained conservatively. Installed retry against the real project is not claimed as completed.
