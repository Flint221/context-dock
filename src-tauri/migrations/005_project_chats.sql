CREATE TABLE IF NOT EXISTS project_chats (
 id TEXT PRIMARY KEY,
 project_id TEXT NOT NULL REFERENCES projects(id),
 title TEXT NOT NULL,
 created_at TEXT NOT NULL,
 updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS project_chat_list ON project_chats(project_id,updated_at);
CREATE TABLE IF NOT EXISTS project_chat_turns (
 id TEXT PRIMARY KEY,
 chat_id TEXT NOT NULL REFERENCES project_chats(id) ON DELETE CASCADE,
 question TEXT NOT NULL,
 answer TEXT,
 state TEXT NOT NULL CHECK(state IN ('searching','running','completed','failed','uncertain')),
 sources TEXT NOT NULL DEFAULT '[]',
 usage TEXT,
 error TEXT,
 created_at TEXT NOT NULL,
 completed_at TEXT
);
CREATE INDEX IF NOT EXISTS project_chat_turn_order ON project_chat_turns(chat_id,created_at);
