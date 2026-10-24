# ContextDock grouped unavailable-folder alerts

A shared drive outage now produces one notification listing affected project names instead of one toast per project. The existing thirty-second persistent batching window collects projects observed on consecutive monitor ticks. The signature uses sorted project IDs, so query ordering does not create repeat alerts. Successful submission alone marks delivery; disabled/failed submission retains retry behavior. The existing five-minute burst throttle remains.

Unavailable-folder projects are excluded from individual problem toasts while their folder is unavailable; their in-app scan/tracking/cloud diagnostics remain unchanged. Available projects retain their individual tracking/cloud problem alerts. Approval notifications and preferences are unchanged. Clicking the summary opens the first affected project for review. Reconnection of all affected projects clears the group watermark, allowing a later disconnect to notify again.

The regression verifies staggered detection, a single four-project summary, valid internal routing, restart persistence, stable ordering and rearming after recovery. Existing queue tests cover unsuccessful delivery and retry. Actual unplugging with the installed build remains a user acceptance check. No user source, application database, startup preference or live cloud resource is modified by this change.
