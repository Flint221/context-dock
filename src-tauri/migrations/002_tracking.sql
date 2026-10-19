ALTER TABLE projects ADD COLUMN tracking_enabled INTEGER NOT NULL DEFAULT 0 CHECK(tracking_enabled IN (0,1));
ALTER TABLE projects ADD COLUMN archive_enabled INTEGER NOT NULL DEFAULT 0 CHECK(archive_enabled IN (0,1));
ALTER TABLE projects ADD COLUMN tracking_initialized INTEGER NOT NULL DEFAULT 0;
ALTER TABLE projects ADD COLUMN watch_status TEXT NOT NULL DEFAULT 'off';
ALTER TABLE projects ADD COLUMN tracking_error TEXT;
ALTER TABLE projects ADD COLUMN removed_at TEXT;

ALTER TABLE documents RENAME TO documents_v1;
DROP INDEX documents_project;
CREATE TABLE documents (
 id TEXT PRIMARY KEY,
 project_id TEXT NOT NULL REFERENCES projects(id),
 family_name TEXT NOT NULL,
 kind TEXT NOT NULL,
 tracking_state TEXT NOT NULL DEFAULT 'discovered',
 active_source_id TEXT,
 current_version_id TEXT,
 merged_into TEXT REFERENCES documents(id)
);
CREATE INDEX documents_project ON documents(project_id);
CREATE TABLE source_files (
 id TEXT PRIMARY KEY,
 project_id TEXT NOT NULL REFERENCES projects(id),
 document_id TEXT NOT NULL REFERENCES documents(id),
 name TEXT NOT NULL,
 relative_path TEXT NOT NULL,
 path_key TEXT NOT NULL,
 kind TEXT NOT NULL,
 size INTEGER NOT NULL,
 modified_at TEXT,
 last_observed TEXT NOT NULL,
 status TEXT NOT NULL,
 disposition TEXT NOT NULL DEFAULT 'discovered',
 last_hash TEXT,
 hashed_stamp TEXT,
 hashed_size INTEGER,
 UNIQUE(project_id, path_key)
);
CREATE INDEX sources_document ON source_files(document_id);
INSERT INTO documents(id,project_id,family_name,kind,active_source_id)
 SELECT id,project_id,name,kind,id FROM documents_v1;
INSERT INTO source_files(id,project_id,document_id,name,relative_path,path_key,kind,size,modified_at,last_observed,status)
 SELECT id,project_id,id,name,relative_path,path_key,kind,size,modified_at,last_observed,status FROM documents_v1;
DROP TABLE documents_v1;
CREATE TABLE document_versions (
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
 UNIQUE(document_id, number),
 UNIQUE(document_id, sha256)
);
CREATE TABLE document_aliases (
 project_id TEXT NOT NULL REFERENCES projects(id),
 parent_key TEXT NOT NULL,
 stem TEXT NOT NULL,
 kind TEXT NOT NULL,
 document_id TEXT NOT NULL REFERENCES documents(id),
 PRIMARY KEY(project_id,parent_key,stem,kind,document_id)
);
CREATE TABLE pending_matches (
 source_id TEXT PRIMARY KEY REFERENCES source_files(id),
 explanation TEXT NOT NULL,
 candidate_ids TEXT NOT NULL,
 observed_hash TEXT NOT NULL,
 created_at TEXT NOT NULL
);
CREATE TABLE operations (
 id TEXT PRIMARY KEY,
 project_id TEXT NOT NULL REFERENCES projects(id),
 kind TEXT NOT NULL,
 state TEXT NOT NULL DEFAULT 'pending',
 payload TEXT NOT NULL,
 error TEXT,
 created_at TEXT NOT NULL,
 updated_at TEXT NOT NULL
);
CREATE INDEX operations_pending ON operations(project_id,state);
CREATE TABLE tracking_activity (
 id TEXT PRIMARY KEY,
 project_id TEXT NOT NULL REFERENCES projects(id),
 kind TEXT NOT NULL,
 message TEXT NOT NULL,
 created_at TEXT NOT NULL
);
CREATE INDEX activity_project ON tracking_activity(project_id,created_at);
