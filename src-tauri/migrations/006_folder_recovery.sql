ALTER TABLE projects ADD COLUMN folder_state TEXT NOT NULL DEFAULT 'ready';
ALTER TABLE projects ADD COLUMN folder_identity TEXT;
ALTER TABLE projects ADD COLUMN folder_revision INTEGER NOT NULL DEFAULT 0;
