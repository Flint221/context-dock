CREATE TABLE IF NOT EXISTS path_selection (
 project_id TEXT PRIMARY KEY REFERENCES projects(id),
 revision INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS path_exclusions (
 project_id TEXT NOT NULL REFERENCES projects(id),
 path TEXT NOT NULL,
 kind TEXT NOT NULL CHECK(kind IN ('folder','file')),
 PRIMARY KEY(project_id,path)
);
INSERT OR IGNORE INTO path_exclusions(project_id,path,kind)
 SELECT project_id,lower(replace(relative_path,char(92),'/')),'file'
 FROM source_files WHERE disposition='ignored';
