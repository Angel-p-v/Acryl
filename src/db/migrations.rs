//! Migraciones de esquema SQLite.

#[derive(Debug, Clone)]
pub struct Migration {
    pub version: i32,
    pub name: &'static str,
    pub sql: &'static str,
}

const INIT_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS media (
  id          INTEGER PRIMARY KEY,
  type        TEXT    NOT NULL CHECK (type IN ('series', 'anime', 'movie')),
  title       TEXT    NOT NULL,
  sort_title  TEXT    NOT NULL,
  year        INTEGER,
  folder_path TEXT    NOT NULL UNIQUE,
  created_at  TEXT    NOT NULL DEFAULT (datetime('now')),
  updated_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS media_files (
  id          INTEGER PRIMARY KEY,
  media_id    INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE,
  season      INTEGER,
  episode     INTEGER,
  title       TEXT,
  file_path   TEXT    NOT NULL UNIQUE,
  size_bytes  INTEGER,
  mtime_ms    INTEGER,
  indexed_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS index_runs (
  id            INTEGER PRIMARY KEY,
  started_at    TEXT    NOT NULL DEFAULT (datetime('now')),
  finished_at   TEXT,
  files_seen    INTEGER NOT NULL DEFAULT 0,
  files_added   INTEGER NOT NULL DEFAULT 0,
  files_removed INTEGER NOT NULL DEFAULT 0,
  status        TEXT    NOT NULL DEFAULT 'running' CHECK (status IN ('running', 'completed', 'failed'))
);

CREATE INDEX IF NOT EXISTS idx_media_sort_title ON media(sort_title);
CREATE INDEX IF NOT EXISTS idx_media_type ON media(type);
CREATE INDEX IF NOT EXISTS idx_files_media ON media_files(media_id);

CREATE VIRTUAL TABLE IF NOT EXISTS media_fts USING fts5(
  title,
  content='media',
  content_rowid='id'
);

CREATE TRIGGER IF NOT EXISTS media_fts_insert AFTER INSERT ON media BEGIN
  INSERT INTO media_fts(rowid, title) VALUES (new.id, new.title);
END;

CREATE TRIGGER IF NOT EXISTS media_fts_delete AFTER DELETE ON media BEGIN
  INSERT INTO media_fts(media_fts, rowid, title) VALUES ('delete', old.id, old.title);
END;

CREATE TRIGGER IF NOT EXISTS media_fts_update AFTER UPDATE ON media BEGIN
  INSERT INTO media_fts(media_fts, rowid, title) VALUES ('delete', old.id, old.title);
  INSERT INTO media_fts(rowid, title) VALUES (new.id, new.title);
END;
"#;

const MEDIA_STATE_AND_HISTORY_SQL: &str = r#"
ALTER TABLE media ADD COLUMN is_favorite INTEGER NOT NULL DEFAULT 0 CHECK (is_favorite IN (0, 1));
ALTER TABLE media ADD COLUMN is_watched INTEGER NOT NULL DEFAULT 0 CHECK (is_watched IN (0, 1));
ALTER TABLE media ADD COLUMN poster_path TEXT;
ALTER TABLE media ADD COLUMN synopsis TEXT;
ALTER TABLE media ADD COLUMN metadata_fetched_at TEXT;

CREATE TABLE IF NOT EXISTS playback_history (
  id          INTEGER PRIMARY KEY,
  media_id    INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE,
  played_at   TEXT NOT NULL DEFAULT (datetime('now')),
  position    REAL NOT NULL DEFAULT 0,
  duration    REAL NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_playback_history_media ON playback_history(media_id);
"#;

pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "init",
        sql: INIT_SQL,
    },
    Migration {
        version: 2,
        name: "media-state-and-history",
        sql: MEDIA_STATE_AND_HISTORY_SQL,
    },
];
