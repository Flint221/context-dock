# ContextDock shared overview and indexing verification

The project overview is mounted once above Files & tracking, Cloud sync and Ask AI. Saved selection, local tracking and cloud coverage therefore remain visible on every tab. Switching tabs preserves the existing panels, subscriptions and unsaved-selection protection.

## Investigation

A read-only copy of the running application's registry, WAL and SHM shows the reported sequence for `RAP_Tech_Doc_v1.docx` (13,778,302 bytes): an identity-verification error at 21:57:00 UTC, a manual sync request seven seconds later and successful verification of the same uploaded File at 21:57:08 UTC. The other five selected documents had already completed. No replacement upload was necessary. The original HTTP response was not logged, so the exact missing or conflicting field in that historical response cannot be recovered from this evidence.

The old attachment code compared every ownership attribute immediately, treating absent attributes exactly like conflicting attributes. The [OpenAI create-file response schema](https://developers.openai.com/api/reference/resources/vector_stores/subresources/files/methods/create) permits optional/null attributes. A mocked response with omitted attributes reproduces the false mismatch; retrieving the same association with its complete attributes resolves it. This is consistent with the recorded second-sync recovery, rather than evidence that the document exceeded a size limit. No live re-upload or remote mutation was used for this investigation.

## Behavior

Attachment verifies the exact journaled File ID and vector-store ID. Present installation, project, document, revision and SHA-256 attributes must match; conflicting values fail immediately and identify the field. Missing attributes trigger a read of the same association. If still incomplete, the job remains indexing and checks again after 15 seconds, with its state saved across restart. Processing and verification share the existing 120-poll budget. Persistent incompleteness produces a filename-specific error with explicit retry guidance. Other due files continue syncing.

Normal reconciliation also retrieves incomplete list entries and schedules verification of selected jobs. An incomplete response never authorizes completed status, search, metadata replacement or deletion. Destructive reconciliation and removal retain strict ownership checks. Uploaded Files are reused; there is no automatic attribute repair or duplicate upload to work around an identity conflict. Existing failed jobs still require the normal explicit retry, while newly pending verification resumes automatically.

The settings text now matches the existing implementation: 16 MiB raw snapshot, 4 MiB extracted text, 8 MiB DOCX body XML and 128 MiB total expanded DOCX package, alongside ZIP entry/compression bounds, the configured soft upload limit and provider limits. This release corrects stale copy; it does not raise these bounds again.

## Acceptance

Automated coverage exercises missing create-response attributes, delayed verification of a 13 MiB illustrated Word package, other files progressing, restart and incomplete list responses, bounded polling, partial attributes, every immutable identity conflict and refusal of deletion with incomplete ownership. Source bytes and local version history remain intact. The tab regression checks the same overview element and saved counts on all three tabs.

After installation, verify the overview on all tabs and sync a newly selected large Word document once. It should remain indexing while processing/verification is pending and finish without a second Sync click. A genuine mismatch must still show an actionable red error. The original provider response cannot be replayed and installed authenticated acceptance is separate from mocked regression checks. See [release verification](verification.md) for completed checks and installer details.
