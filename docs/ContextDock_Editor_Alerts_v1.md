# ContextDock temporary editor alerts

Version 0.10.10 keeps expected editor locks and active-save waits visible in the application without generating Windows problem notifications. The existing cloud readiness message and automatic recovery after the document becomes readable are retained. A batch containing only sharing/lock violations or capture quiet-window waits is labeled as a temporary tracking wait. A mixed batch retains its normal attention status so an independent error is not hidden behind the editor lock.

The desktop notification signature filters only this explicit tracking wait and the known cloud SOURCE_BUSY message. Failed-job notification counts exclude jobs with that same known temporary source error. Other tracking errors, source/folder disappearance, authentication errors and independent upload failures still produce problem notifications. A previously queued project alert is retired when only a temporary wait remains. Grouped unavailable-drive notifications, approvals and notification preferences retain their existing behavior.

Regressions cover multiple locked sources, mixed lock/integrity errors, source disappearance, authentication and independent failed jobs. The Windows DOCX editor-handle lifecycle test checks that the in-app tracking/cloud waits remain present, the desktop-alert decision is quiet, and closing the editor resumes sync without extra versions or uploads. Actual Word/Windows toast behavior remains an installed acceptance check.

The installer is prepared separately. The running application must stay untouched until the user confirms they have quit it and authorizes installation.
