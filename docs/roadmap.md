# Development roadmap

## Completed: Milestone 1 — Windows application foundation

Tauri/React desktop shell, existing-folder registration, independent metadata discovery, SQLite persistence, dashboard/projects/settings, themes and Windows NSIS/Actions distribution.

## Implemented: Milestone 2 — Automatic document tracking and safe archival

Opt-in per-project native watchers; selected independent initial histories; deterministic filename/date matching and explicit review; SHA-256 duplicate prevention; immutable per-document snapshots; separately authorized Old Context Files archival; persisted activity and recoverable operation journals; startup reconciliation; version history and verified location actions. Existing IDs/preferences migrate in place. See the manual testing and verification records for validation and remaining distribution checks.

## Milestone 3 — Local diff and history comparison

Add document-specific content extraction and local comparison between preserved versions. Define Markdown/text and DOCX extraction boundaries, unsupported binary changes, accessible comparison navigation and long-history performance. Do not conflate capture reasons/metadata with content differences.

## Milestone 4 — Optional OpenAI indexing

Add explicitly opted-in per-project cloud indexing, disclosure of uploaded content, secure credential storage, vector-store associations, upload/delete policy, retries and usage feedback. Offline management and local snapshots remain available.

## Milestone 5 — On-demand AI analysis

Add user-triggered comparisons, document/version-specific baselines, cached analysis and goal-drift assessment with provenance, uncertainty and cost controls. Generated interpretation must remain distinguishable from local evidence.

## Milestone 6 — Production hardening and updates

Signed installers and signed update delivery; clean-machine/upgrade/uninstall coverage; migration backups and diagnostics; accessibility/scale review; cancellation and streaming pagination; storage usage and explicitly authorized retention management; broader filesystem compatibility; architecture-specific installers and offline prerequisite packaging. No automatic destructive retention or update agent is implemented now.
