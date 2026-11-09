CREATE TABLE IF NOT EXISTS drive_cleanup (
 project_id TEXT PRIMARY KEY REFERENCES projects(id),
 error TEXT
);
CREATE TABLE IF NOT EXISTS drive_recycle (
 id TEXT PRIMARY KEY,
 project_id TEXT NOT NULL REFERENCES projects(id),
 document_id TEXT NOT NULL REFERENCES documents(id),
 source_id TEXT NOT NULL REFERENCES source_files(id),
 root TEXT NOT NULL,
 relative TEXT NOT NULL,
 safety_relative TEXT NOT NULL,
 sha256 TEXT NOT NULL,
 file_identity TEXT NOT NULL,
 generation TEXT NOT NULL,
 state TEXT NOT NULL DEFAULT 'prepared',
 bin_receipt TEXT,
 error TEXT,
 created_at TEXT NOT NULL,
 UNIQUE(source_id,sha256)
);
