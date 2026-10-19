-- Remote resource identifiers and intents only. Credentials never enter SQLite.
CREATE TABLE cloud_installation (id TEXT PRIMARY KEY);
CREATE TABLE cloud_connection (
 singleton INTEGER PRIMARY KEY CHECK(singleton=1), scope TEXT, status TEXT NOT NULL DEFAULT 'disconnected',
 tested_at TEXT, error TEXT
);
INSERT INTO cloud_connection(singleton) VALUES(1);
CREATE TABLE cloud_projects (
 project_id TEXT PRIMARY KEY REFERENCES projects(id),
 enabled INTEGER NOT NULL DEFAULT 0, paused INTEGER NOT NULL DEFAULT 1,
 auto_sync INTEGER NOT NULL DEFAULT 0, started INTEGER NOT NULL DEFAULT 0,
 mode TEXT NOT NULL DEFAULT 'current' CHECK(mode IN ('current','history')),
 soft_limit INTEGER NOT NULL DEFAULT 52428800,
 scope TEXT, store_id TEXT UNIQUE, store_name TEXT NOT NULL,
 store_op TEXT, store_state TEXT NOT NULL DEFAULT 'none',
 status TEXT NOT NULL DEFAULT 'local_only', error TEXT,
 created_at TEXT, last_sync TEXT, usage_bytes INTEGER,
 sync_requested INTEGER NOT NULL DEFAULT 0, reconcile_requested INTEGER NOT NULL DEFAULT 0,
 delete_requested INTEGER NOT NULL DEFAULT 0,
 next_at INTEGER NOT NULL DEFAULT 0, attempts INTEGER NOT NULL DEFAULT 0,
 revision INTEGER NOT NULL DEFAULT 0
);
INSERT INTO cloud_projects(project_id,store_name) SELECT id,'ContextDock - '||name FROM projects;
CREATE TRIGGER cloud_project_added AFTER INSERT ON projects BEGIN
 INSERT INTO cloud_projects(project_id,store_name) VALUES(NEW.id,'ContextDock - '||NEW.name);
END;
CREATE TRIGGER cloud_project_removed AFTER UPDATE OF removed_at ON projects WHEN NEW.removed_at IS NOT NULL BEGIN
 UPDATE cloud_projects SET enabled=0,paused=1,auto_sync=0,started=0,status='disconnected',revision=revision+1 WHERE project_id=NEW.id;
END;
CREATE TABLE cloud_documents (
 project_id TEXT NOT NULL REFERENCES cloud_projects(project_id),
 document_id TEXT NOT NULL REFERENCES documents(id),
 PRIMARY KEY(project_id,document_id)
);
CREATE TABLE cloud_exclusions (
 project_id TEXT NOT NULL REFERENCES cloud_projects(project_id),
 version_id TEXT NOT NULL REFERENCES document_versions(id),
 PRIMARY KEY(project_id,version_id)
);
CREATE TABLE cloud_jobs (
 id TEXT PRIMARY KEY,
 project_id TEXT NOT NULL REFERENCES cloud_projects(project_id),
 document_id TEXT NOT NULL REFERENCES documents(id),
 version_id TEXT NOT NULL REFERENCES document_versions(id),
 remote_name TEXT NOT NULL UNIQUE, file_id TEXT UNIQUE,
 state TEXT NOT NULL DEFAULT 'pending', phase TEXT NOT NULL DEFAULT 'upload',
 desired INTEGER NOT NULL DEFAULT 1, remote_current INTEGER,
 association_status TEXT, usage_bytes INTEGER,
 attempts INTEGER NOT NULL DEFAULT 0, next_at INTEGER NOT NULL DEFAULT 0,
 error TEXT, updated_at TEXT NOT NULL,
 UNIQUE(project_id,version_id)
);
CREATE INDEX cloud_jobs_due ON cloud_jobs(project_id,next_at,state);
CREATE TABLE cloud_activity (
 id INTEGER PRIMARY KEY AUTOINCREMENT, project_id TEXT NOT NULL REFERENCES cloud_projects(project_id),
 kind TEXT NOT NULL, message TEXT NOT NULL, created_at TEXT NOT NULL
);
CREATE TABLE cloud_retrievals (
 id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES cloud_projects(project_id),
 created_at TEXT NOT NULL, results TEXT NOT NULL
);
