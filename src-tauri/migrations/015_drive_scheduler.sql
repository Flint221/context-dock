CREATE TABLE IF NOT EXISTS drive_schedule (
 project_id TEXT PRIMARY KEY REFERENCES projects(id),
 observed_signature TEXT,
 settle_until INTEGER NOT NULL DEFAULT 0,
 blocked TEXT CHECK(blocked IS NULL OR blocked IN ('auth','attention'))
);
