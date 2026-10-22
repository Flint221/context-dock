-- Ordered import journal entries require a version-aware reader on downgrade.
CREATE INDEX IF NOT EXISTS operations_document_state ON operations(document_id,kind,state);
CREATE TABLE IF NOT EXISTS initial_import_notices (
 project_id TEXT PRIMARY KEY REFERENCES projects(id),
 message TEXT NOT NULL
);
