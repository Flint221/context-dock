CREATE TABLE IF NOT EXISTS archive_intents (
 project_id TEXT PRIMARY KEY REFERENCES projects(id),
 enabled INTEGER NOT NULL CHECK(enabled IN (0,1)),
 pending INTEGER NOT NULL CHECK(pending IN (0,1))
);
