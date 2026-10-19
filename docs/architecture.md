# Architecture

ContextDock separates UI, typed IPC, native commands, registry persistence, and filesystem observation. The application identifier `com.contextdock.desktop` is stable across releases.

## Components

- `src/App.tsx`: navigation, selected project, operation feedback, and service orchestration.
- `src/components/`: dashboard, project cards/details, document table, settings, accessible native HTML modal, and registration form. Document table rendering is paged in batches of 100; search is deferred to keep input responsive.
- `src/services/registry.ts`: typed Tauri invocation facade and native directory picker. `src/types.ts` mirrors serialized Rust payloads.
- `src-tauri/src/lib.rs`: narrowly scoped commands, application startup, supported app-data path resolution, and shared registry state.
- `registry.rs`: validation, versioned SQLite migrations, project operations, scan reconciliation, and snapshots.
- `scanner.rs`: canonical root validation and recursive read-only traversal. It returns observations and errors, with no database responsibility.
- `models.rs`: serialized project/document snapshots, UTC time and Windows identity-key helpers.
- `migrations/001_registry.sql`: registry schema, uniqueness constraints, foreign keys and indexes.
- `.github/workflows/windows.yml`: Windows validation, reproducible dependency installs, NSIS artifact creation, and isolated tag-release publishing.

The frontend can select a folder and call project-level operations. It has no filesystem plugin, shell plugin, database plugin, content-reading command, or arbitrary file-write API. The only plugin permission is `dialog:allow-open`, scoped to the main local window. Explorer opens only an existing registered project ID, resolved and validated in Rust. User-provided paths never become shell command strings. The CSP permits packaged assets and Tauri IPC, without remote scripts or network services.

All database work and scans run through `spawn_blocking`, away from Tauri's UI thread. A mutex serializes native mutations and reconciliations so refresh/removal cannot race. SQLite also enforces canonical-folder and project/relative-path uniqueness. Snapshots may wait for an active scan; the frontend keeps navigation and progress feedback responsive. There is no background watcher. Scans are currently collected in memory; cancellation, streaming batches and larger-scale registry pagination are future hardening work.

## Persistence and identities

```text
Project (UUID)
  └── Logical document (UUID; Milestone 1 documents table)
        └── Document version (future; no snapshots created in Milestone 1)
```

`projects` stores the display name, canonical root, normalized unique path key, creation time, scan status/time and scan warning. Display-name edits never rename the filesystem directory. Unregistering cascades only to registry documents through SQLite foreign keys.

`documents` stores an independent logical document ID with its current observed source location and metadata: project ID, name, relative path, normalized identity key, kind, size, modification time, last successfully observed time, and availability. Absolute paths are derived from the registered canonical root and scanner-produced relative path. The `(project_id, path_key)` constraint distinguishes equal filenames in different directories. Repeated scans upsert observations without replacing UUIDs.

Milestone 1 associates each logical document with one source path. Path changes currently create a new identity and retain the old missing observation. Paths are not the final long-term document matching strategy. A later source-locations table can separate multiple filesystem observations from a logical identity while retaining current document UUIDs. Explicit user matching, content hashes and similarity can reconcile exports with different filenames without merging every Markdown file in a project into one history.

A future `document_versions` table should have its own UUID, a required `document_id` foreign key, capture UTC time, immutable archive location and content hash. Version ordering/uniqueness belongs within each document, never at project scope. The source modification timestamp is an observation, not a capture timestamp. Baseline selection should reference a particular document/version. Hashing, archive writes and captured versions are deliberately absent now. Vector-store associations and cached analysis should be introduced with the features that need them rather than preallocated speculative tables.

`preferences` currently stores theme selection. Migrations run transactionally under `PRAGMA user_version`; launch is idempotent, and a database created by a newer schema is rejected rather than downgraded. Foreign keys are enabled on every connection. WAL and a busy timeout protect persistence. Metadata survives restarts and installation upgrades; the stable app identifier must not change.

## Observation semantics

1. Registration validates an absolute folder in Rust, canonicalizes it, verifies directory access, rejects a duplicate, persists its identity and performs an initial scan.
2. Opening a project or manual refresh runs a recursive metadata scan. Child symlinks and reparse points are excluded, including junctions. Standard dependency/output/archive folders are pruned.
3. Available files are upserted in a transaction. Missing-file status is assigned only after a complete traversal. Partial scans assign unavailable status to previously observed but unconfirmed files; a root failure makes the project unavailable.
4. `last_observed` advances only for files actually observed. Missing/unavailable records retain their previous size and modification timestamp. Reappearing files at the same relative path recover their UUID.
5. Dashboard counts include retained observations, with an available count shown separately. Recency uses observed file modification metadata. No live filesystem monitoring is implied.

The scanner reads directory entries and metadata, without opening document contents or writing into project roots. Reparse points are excluded to prevent recursive loops and unrelated traversal. As with ordinary filesystem traversal, concurrent external directory replacement is not an adversarial sandbox guarantee; handle-based traversal would be additional hardening if threat requirements expand.

## Validation and extending the application

Rust tests use temporary roots/databases, including a Windows junction fixture and explicit ACL denial restored by a scope guard. React interaction tests mock only the typed native facade. Formatting, TypeScript, ESLint, tests, clippy and production packaging run in Windows CI.

New modules can add sidebar entries and screen components while retaining the service boundary. Add native services behind project/document IDs rather than exposing arbitrary filesystem APIs. New persistent concepts require numbered transactional migrations and reconnection tests. Milestone 2 must define explicit safe archive ownership and opt-in tracking before adding writes; the Milestone 1 discovery code should remain a read-only observation service.

## Official references reviewed

- [Tauri Rust commands](https://v2.tauri.app/develop/calling-rust/)
- [Tauri capabilities](https://v2.tauri.app/security/capabilities/)
- [Tauri dialog plugin](https://v2.tauri.app/plugin/dialog/)
- [Tauri Windows installers](https://v2.tauri.app/distribute/windows-installer/)
- [Tailwind Vite installation](https://tailwindcss.com/docs/installation/using-vite)
- [GitHub Actions security](https://docs.github.com/en/actions/reference/security/secure-use)
