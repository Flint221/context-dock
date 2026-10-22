-- Run inside the registry migration transaction with foreign keys disabled.
CREATE TABLE document_versions_new (
 id TEXT PRIMARY KEY,
 document_id TEXT NOT NULL REFERENCES documents(id),
 number INTEGER NOT NULL CHECK(number > 0),
 source_name TEXT NOT NULL,
 source_relative TEXT NOT NULL,
 captured_at TEXT NOT NULL,
 modified_at TEXT,
 size INTEGER NOT NULL,
 sha256 TEXT NOT NULL,
 snapshot_relative TEXT NOT NULL UNIQUE,
 archive_relative TEXT,
 reason TEXT NOT NULL,
 UNIQUE(document_id, number)
);
INSERT INTO document_versions_new(id,document_id,number,source_name,source_relative,captured_at,modified_at,size,sha256,snapshot_relative,archive_relative,reason) SELECT id,document_id,number,source_name,source_relative,captured_at,modified_at,size,sha256,snapshot_relative,archive_relative,reason FROM document_versions;
DROP TABLE document_versions;
ALTER TABLE document_versions_new RENAME TO document_versions;
CREATE INDEX version_content_lookup ON document_versions(document_id, sha256);
ALTER TABLE document_versions ADD COLUMN version_key TEXT;
