//! Definiciones de migraciones de esquema para SQLite.
//!
//! Las migraciones se declaran en Rust (SQL embebido) para que viajen
//! compiladas dentro del binario y no dependan de ficheros `.sql` en disco,
//! igual que el proyecto original las declara en TypeScript para que viajen
//! en `dist/`.

/// Una migración de esquema: número de versión, nombre y SQL aplicable.
#[derive(Debug, Clone)]
pub struct Migration {
    pub version: i32,
    pub name: &'static str,
    pub sql: &'static str,
}

/// v1: esquema inicial. Tablas media, media_files, index_runs, FTS5 y triggers.
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
CREATE INDEX IF NOT EXISTS idx_files_season_episode ON media_files(media_id, season, episode);

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

// v2: índices de apoyo para listados ordenados y mantenimiento. Aditiva,
// solo de rendimiento, no cambia datos ni el contrato de las tablas.
const PERFORMANCE_INDEXES_SQL: &str = r#"
CREATE INDEX IF NOT EXISTS idx_media_type_sort_title ON media(type, sort_title);
CREATE INDEX IF NOT EXISTS idx_media_created_at ON media(created_at);
CREATE INDEX IF NOT EXISTS idx_media_updated_at ON media(updated_at);
CREATE INDEX IF NOT EXISTS idx_index_runs_started_at ON index_runs(started_at);
"#;

// v3: estado de reproducción y metadatos. Agrega columnas a `media` (aditivas,
// con valores por defecto seguros) y la tabla `playback_history` para el
// historial de reproducción.
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
CREATE INDEX IF NOT EXISTS idx_playback_history_played_at ON playback_history(played_at);
"#;

/// Lista de migraciones en orden de versión.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "init",
        sql: INIT_SQL,
    },
    Migration {
        version: 2,
        name: "performance-indexes",
        sql: PERFORMANCE_INDEXES_SQL,
    },
    Migration {
        version: 3,
        name: "media-state-and-history",
        sql: MEDIA_STATE_AND_HISTORY_SQL,
    },
];
