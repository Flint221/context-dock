# Development roadmap

Milestone 1 establishes project registrations, independent document observations, local persistence, desktop navigation and Windows distribution. Subsequent work is intentionally unimplemented.

## Milestone 2 — Automatic tracking and safe archival

Introduce opt-in filesystem observation, debouncing, explicit archive ownership and immutable snapshots per logical document. Define atomic capture, exclusions, recovery from interrupted writes and safe retention before modifying anything. Reconcile renamed exports using explicit matching and hashes. Preserve original source files.

## Milestone 3 — Local diff and version history

Provide document-specific history and local comparison between immutable versions. Define Markdown/text extraction and DOCX extraction boundaries. Clearly separate source metadata, captures, diffs and unsupported binary changes. Support usable navigation across long histories.

## Milestone 4 — Optional OpenAI indexing

Add opt-in, per-project cloud indexing with explicit disclosure of what leaves the machine. Store credentials securely outside SQLite/repositories. Introduce vector-store associations, upload state, deletion policy, retries and usage feedback. Offline project management remains available.

## Milestone 5 — On-demand AI analysis

Add user-triggered comparisons, explicit project baselines, cached analysis and goal-drift assessment. Link findings to particular documents and immutable versions. Show provenance, uncertainty, cost controls and the distinction between local evidence and generated interpretation.

## Milestone 6 — Production hardening and updates

Add signed installers, signed update delivery, migration backup/recovery, scale testing, accessibility review, startup diagnostics, support export controls, archive retention safety, scan cancellation and installer upgrade/uninstall testing. Revisit architecture-specific installers and fully offline prerequisite packaging. Automatic updates are not present in Milestone 1.
