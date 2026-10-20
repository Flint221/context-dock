CREATE TABLE ai_baselines (
 id TEXT PRIMARY KEY,
 project_id TEXT NOT NULL REFERENCES projects(id),
 title TEXT NOT NULL,
 revision INTEGER NOT NULL,
 supersedes TEXT REFERENCES ai_baselines(id),
 source_version TEXT REFERENCES document_versions(id),
 source_hash TEXT,
 content TEXT NOT NULL,
 created_at TEXT NOT NULL,
 approved_at TEXT NOT NULL,
 UNIQUE(project_id,revision)
);
CREATE TABLE ai_current_baseline (
 project_id TEXT PRIMARY KEY REFERENCES projects(id),
 baseline_id TEXT NOT NULL REFERENCES ai_baselines(id)
);
CREATE TRIGGER ai_baseline_immutable BEFORE UPDATE ON ai_baselines
BEGIN SELECT RAISE(ABORT, 'Approved baselines are immutable; create a revision.'); END;
CREATE TRIGGER ai_baseline_retained BEFORE DELETE ON ai_baselines
BEGIN SELECT RAISE(ABORT, 'Approved baseline history is retained.'); END;
CREATE TABLE ai_reports (
 id TEXT PRIMARY KEY,
 project_id TEXT NOT NULL REFERENCES projects(id),
 cache_key TEXT NOT NULL,
 kind TEXT NOT NULL,
 state TEXT NOT NULL,
 request TEXT NOT NULL,
 plan TEXT NOT NULL,
 result TEXT,
 usage TEXT,
 response_id TEXT,
 error TEXT,
 created_at TEXT NOT NULL,
 completed_at TEXT
);
CREATE INDEX ai_report_cache ON ai_reports(project_id,cache_key,state);
CREATE TABLE ai_decisions (
 report_id TEXT NOT NULL REFERENCES ai_reports(id),
 finding INTEGER NOT NULL,
 decision TEXT NOT NULL,
 updated_at TEXT NOT NULL,
 PRIMARY KEY(report_id,finding)
);
