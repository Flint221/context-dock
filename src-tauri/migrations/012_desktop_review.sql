CREATE TABLE IF NOT EXISTS review_decisions (
 project_id TEXT NOT NULL REFERENCES projects(id), path TEXT NOT NULL,
 decision TEXT NOT NULL CHECK(decision IN ('approve','ignore')), decided_at TEXT NOT NULL,
 PRIMARY KEY(project_id,path)
);
CREATE TABLE IF NOT EXISTS desktop_notices (
 key TEXT PRIMARY KEY, signature TEXT NOT NULL, changed_at INTEGER NOT NULL,
 last_sent INTEGER NOT NULL DEFAULT 0, last_signature TEXT NOT NULL DEFAULT ''
);
