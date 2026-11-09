# ContextDock publication and local cleanup recovery

This supplements and supersedes the active recovery instructions in v1 for the 0.11.0 preview. The stable-save scheduler, durable remote identities, retries and account-scoped resumption remain as described there. Schema 18 also requires an explicit existing-folder selection or creation choice before a new project destination is created.

Older-file cleanup has an independent recovery journal. Turning recycling off or disconnecting Drive does not hide an unfinished safety move. Its warning remains visible outside the collapsed Optional local cleanup controls. Publication can remain verified while that local recovery requires attention.

Tray problem counts and grouped notifications use current Drive failures and unresolved local cleanup in the latest-file workflow. Retained OpenAI/index errors do not create stale alerts. Paused publication and ignored/excluded families do not produce new Drive-failure alerts; unresolved safety moves remain actionable even when publication is off.

The latest-document view also displays unresolved cleanup warnings and project-level authorization/destination blocks. A missing destination choice is Needs attention rather than an indefinite Queued label. A cleanup warning does not invalidate a separately verified remote-content receipt.

Open Optional local cleanup → Retry interrupted cleanup after fixing the indicated path or access problem. This action works without a Google connection, never publishes a document and does not enable recycling. It retries only that project's retained local recovery intents. The publication/cleanup gate prevents concurrent source moves during the check.

If the original path is free and the safety file's physical identity/hash still match, the file is restored without replacement. If the path is occupied, keep the replacement: move it to another desired location, then retry recovery. Available safety bytes remain retained until the conflict is resolved. If the original and safety locations are both absent, inspect the Windows Recycle Bin; the app does not infer a successful deletion or repeat an uncertain destructive operation. A restored file and an occupied-path replacement are both preserved by the regression test.

Warnings clear only after the requested recovery confirms that no unresolved intent remains. Choosing recycling off is not a resolution receipt. Recycled files restored through the Windows Recycle Bin return to the safety-folder path documented in ContextDock_Drive_Recycling_v1.md; recovery can then verify and return the retained file to its original location when safe.

The interface test verifies recovery while disconnected, saved-state readback and absence of publication. The backend test exercises an occupied original path, failed recovery, preservation of the replacement, explicit retry after moving that replacement, and successful restoration of the unchanged safety file. These extend the existing crash, opt-out, bin-failure and newest-file preservation checks. Live Google acceptance still requires the new Desktop OAuth client; this local action does not replace it.
