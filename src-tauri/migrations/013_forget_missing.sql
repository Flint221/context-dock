-- Snapshot deletion is journaled with explicit local-history removal.
CREATE TABLE IF NOT EXISTS forgotten_snapshots (
 snapshot_relative TEXT PRIMARY KEY, sha256 TEXT NOT NULL,
 project_id TEXT NOT NULL, error TEXT
);
