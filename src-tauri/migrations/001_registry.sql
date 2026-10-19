CREATE TABLE projects (
 id TEXT PRIMARY KEY, name TEXT NOT NULL, path TEXT NOT NULL,
 path_key TEXT NOT NULL UNIQUE, created_at TEXT NOT NULL,
 status TEXT NOT NULL DEFAULT 'available', last_scan TEXT, scan_error TEXT
);
CREATE TABLE documents (
 id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
 name TEXT NOT NULL, relative_path TEXT NOT NULL, path_key TEXT NOT NULL,
 kind TEXT NOT NULL, size INTEGER NOT NULL, modified_at TEXT,
 last_observed TEXT NOT NULL, status TEXT NOT NULL,
 UNIQUE(project_id, path_key)
);
CREATE INDEX documents_project ON documents(project_id);
CREATE TABLE preferences (key TEXT PRIMARY KEY, value TEXT NOT NULL);
