# ContextDock cloud removal verification

Version 0.10.9 fixes automatic sync stopping after the provider acknowledged deletion of an obsolete vector-store association but still returned it on the immediate verification read. Previously these valid pending responses consumed the six-attempt failure budget, leaving a permanent error until Sync now was pressed. Current-version indexing counts could already be complete while this cleanup remained stuck.

A successful deletion acknowledgement is now persisted on the existing job as removal_pending. Later attempts verify ownership and poll that same association instead of issuing repeated DELETE requests. Valid pending reads schedule another check after thirty seconds across worker ticks and restarts without consuming the separate provider-failure budget. Real authentication, ownership, malformed receipt and network failures retain existing safeguards. Uploaded Files, local snapshots and document history are retained. Recognized legacy removal-wait errors that had exhausted their attempts are resumed automatically.

The overview shows Finishing sync and the cloud page explains that current versions are indexed while previous entries await verified removal. This is waiting progress rather than a failed upload. Chat remains disabled until the acknowledged entries are verified absent. Counts continue to represent the desired indexed versions.

Regressions cover polling beyond twelve verification waits, recovery of both legacy messages, thirty-second scheduling, restart, exactly one DELETE after acknowledgement, retained uploads and history, eventual absence and UI/chat behavior. Provider delay is simulated; the user's actual cloud visibility delay is not claimed to be reproduced live.
