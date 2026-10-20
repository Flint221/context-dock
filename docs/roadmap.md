# Development roadmap

## Completed: Milestone 1 — Windows application foundation

Tauri/React desktop shell, existing-folder registration, independent metadata discovery, SQLite persistence, dashboard/projects/settings, themes and Windows NSIS/Actions distribution.

## Implemented: Milestone 2 — Automatic document tracking and safe archival

Opt-in per-project native watchers; selected independent initial histories; deterministic filename/date matching and explicit review; SHA-256 duplicate prevention; immutable per-document snapshots; separately authorized Old Context Files archival; persisted activity and recoverable operation journals; startup reconciliation; version history and verified location actions. Existing IDs/preferences migrate in place. See the manual testing and verification records for validation and remaining distribution checks.

## Implemented: Milestone 3 — Local diff and history comparison

SHA-256-verified Markdown and bounded DOCX body extraction; exact Myers diffs for any historical pair in one family; raw bytes versus normalized text; unified/split display; paged regions, context expansion and change navigation; clipboard summaries/changed text; actionable errors and modal lifecycle coverage. No schema migration, capture changes, comparison cache or AI interpretation. See the Milestone 3 manual guide and verification record for limits and remaining interactive upgrade/Word checks.

## Implemented: Milestone 4 — Optional OpenAI indexing

Windows Credential Manager; transactional schema 003; explicit per-project selection/preview/retention consent; one owned vector store per project; verified original snapshot uploads; current-only/full-history sync; durable queue, processing polling, bounded retries and uncertain-operation reconciliation; direct retrieval tests with provenance; separate association/store/File cleanup and measured usage. Local tracking, archival and diffs remain independent and available offline. Version 0.4.0 retains application identity and per-user installer/data paths. Live authorized OpenAI testing and installed upgrade acceptance are user-run tasks in the Milestone 4 guide; AI interpretation is added separately in Milestone 5.

## Implemented: Milestone 5 — On-demand AI analysis, baselines and evolution

Version 0.5.0 adds explicitly approved Responses requests for exact version comparison, baseline alignment, ordered evolution and cross-document consistency. Immutable approved baseline revisions, snapshot/diff evidence, validated quotations, local report/decision persistence, exact cache reuse, usage/price previews, bounded one-request plans and restart/cancellation uncertainty are implemented. Schema 004 preserves the existing system. All 104 Rust tests and 41 frontend tests pass with TypeScript, ESLint, rustfmt, Clippy and local NSIS packaging. The mandatory Milestone 5 guide covers live user acceptance and upgrade checks; authenticated model testing and installed upgrade were not performed by the agent. Interpretation remains separate from authoritative local evidence.

## Milestone 6 — Production hardening and updates

Signed installers and signed update delivery; clean-machine/upgrade/uninstall coverage; migration backups and diagnostics; accessibility/scale review; cancellation and streaming pagination; storage usage and explicitly authorized retention management; broader filesystem compatibility; architecture-specific installers and offline prerequisite packaging. No automatic destructive retention or update agent is implemented now.
