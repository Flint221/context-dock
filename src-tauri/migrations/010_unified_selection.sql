CREATE TABLE IF NOT EXISTS new_paths (
 project_id TEXT NOT NULL REFERENCES projects(id),
 path TEXT NOT NULL,
 PRIMARY KEY(project_id,path)
);
UPDATE cloud_projects SET auto_sync=1 WHERE enabled=1;
