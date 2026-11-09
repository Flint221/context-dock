-- Metadata and recovery intents only. OAuth tokens and upload session URLs
-- belong in the native vault / bounded native memory, never in this journal.
CREATE TABLE IF NOT EXISTS drive_accounts (
 account_id TEXT PRIMARY KEY,
 root_id TEXT,
 create_root INTEGER NOT NULL DEFAULT 0 CHECK(create_root IN (0,1)),
 root_ready INTEGER NOT NULL DEFAULT 0 CHECK(root_ready IN (0,1))
);
CREATE TABLE IF NOT EXISTS drive_projects (
 project_id TEXT PRIMARY KEY REFERENCES projects(id),
 account_id TEXT NOT NULL REFERENCES drive_accounts(account_id),
 folder_id TEXT,
 folder_ready INTEGER NOT NULL DEFAULT 0 CHECK(folder_ready IN (0,1)),
 enabled INTEGER NOT NULL DEFAULT 0 CHECK(enabled IN (0,1)),
 automatic INTEGER NOT NULL DEFAULT 0 CHECK(automatic IN (0,1)),
 recycle INTEGER NOT NULL DEFAULT 0 CHECK(recycle IN (0,1)),
 requested INTEGER NOT NULL DEFAULT 0 CHECK(requested IN (0,1)),
 attempts INTEGER NOT NULL DEFAULT 0,
 next_at INTEGER NOT NULL DEFAULT 0,
 error TEXT
);
CREATE TABLE IF NOT EXISTS drive_documents (
 document_id TEXT PRIMARY KEY REFERENCES documents(id),
 project_id TEXT NOT NULL REFERENCES drive_projects(project_id),
 file_id TEXT NOT NULL,
 verified_hash TEXT,
 verified_name TEXT,
 verified_size INTEGER,
 verified_revision TEXT,
 verified_at TEXT,
 pending_hash TEXT,
 pending_name TEXT,
 pending_size INTEGER,
 pending_version TEXT,
 pending_source TEXT,
 pending_relative TEXT,
 pending_stage TEXT,
 pending_generation TEXT,
 state TEXT NOT NULL DEFAULT 'queued',
 attempts INTEGER NOT NULL DEFAULT 0,
 next_at INTEGER NOT NULL DEFAULT 0,
 error TEXT,
 UNIQUE(project_id,file_id)
);
CREATE INDEX IF NOT EXISTS drive_documents_due ON drive_documents(state,next_at);
