CREATE TABLE IF NOT EXISTS drive_folder_choices (
 project_id TEXT PRIMARY KEY REFERENCES projects(id),
 may_create INTEGER NOT NULL DEFAULT 0 CHECK(may_create IN (0,1))
);
