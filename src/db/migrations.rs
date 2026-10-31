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

const MEDIA_PARENT_ID_SQL: &str = r#"
ALTER TABLE media ADD COLUMN parent_id INTEGER REFERENCES media(id);

CREATE INDEX IF NOT EXISTS idx_media_parent ON media(parent_id);
"#;

const MEDIA_STREAMS_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS media_streams (
  id          INTEGER PRIMARY KEY,
  media_id    INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE,
  file_path   TEXT    NOT NULL,
  stream_type TEXT    NOT NULL CHECK (stream_type IN ('audio', 'subtitle')),
  stream_index INTEGER NOT NULL,
  language    TEXT,
  codec       TEXT,
  title       TEXT,
  indexed_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_media_streams_media ON media_streams(media_id);
CREATE INDEX IF NOT EXISTS idx_media_streams_file ON media_streams(file_path);
CREATE INDEX IF NOT EXISTS idx_media_streams_type ON media_streams(stream_type);
"#;

/// Repara los triggers de FTS5 que nunca llegaron a crearse en bases de datos
/// antiguas (el runner anterior dividía el SQL por `;` y rompía los triggers)
/// y reconstruye el índice para que las búsquedas existentes funcionen.
const FTS_TRIGGERS_REPAIR_SQL: &str = r#"
DROP TRIGGER IF EXISTS media_fts_insert;
DROP TRIGGER IF EXISTS media_fts_delete;
DROP TRIGGER IF EXISTS media_fts_update;

CREATE TRIGGER media_fts_insert AFTER INSERT ON media BEGIN
  INSERT INTO media_fts(rowid, title) VALUES (new.id, new.title);
END;

CREATE TRIGGER media_fts_delete AFTER DELETE ON media BEGIN
  INSERT INTO media_fts(media_fts, rowid, title) VALUES ('delete', old.id, old.title);
END;

CREATE TRIGGER media_fts_update AFTER UPDATE ON media BEGIN
  INSERT INTO media_fts(media_fts, rowid, title) VALUES ('delete', old.id, old.title);
  INSERT INTO media_fts(rowid, title) VALUES (new.id, new.title);
END;

INSERT INTO media_fts(media_fts) VALUES ('rebuild');
"#;

/// Añade la URL del banner ancho (background/banner de TVmaze o AniList),
/// independiente del póster vertical.
const MEDIA_BANNER_PATH_SQL: &str = r#"
ALTER TABLE media ADD COLUMN banner_path TEXT;
"#;

/// Preferencias de pista por archivo y marca de extracción con `ffprobe`.
///
/// `audio_stream_index` y `subtitle_stream_index` guardan índices relativos al
/// tipo, empezando en 0 (el orden en que los devuelve `ffprobe`). `NULL`
/// significa «automático»: mpv elige la pista por defecto. Para desactivar los
/// subtítulos se usa `subtitles_disabled = 1`, porque `NULL` ya representa el
/// comportamiento automático y necesitamos distinguir ambos casos.
const MEDIA_FILE_TRACKS_SQL: &str = r#"
ALTER TABLE media_files ADD COLUMN audio_stream_index INTEGER;
ALTER TABLE media_files ADD COLUMN subtitle_stream_index INTEGER;
ALTER TABLE media_files ADD COLUMN subtitles_disabled INTEGER NOT NULL DEFAULT 0;
ALTER TABLE media_files ADD COLUMN streams_indexed_at TEXT;
"#;

/// Añade la columna de calificación (0-5) a la tabla media.
const MEDIA_RATING_SQL: &str = r#"
ALTER TABLE media ADD COLUMN rating INTEGER NOT NULL DEFAULT 0 CHECK (rating >= 0 AND rating <= 5);
"#;

/// Tablas de categorías para filtrado de medios, puntuación obtenida de
/// metadatos (vote_average de TVmaze) e imagen específica de episodio.
const MEDIA_CATEGORIES_AND_VOTE_SQL: &str = r#"
-- Catálogo de categorías (géneros/etiquetas) para filtrar la biblioteca.
CREATE TABLE IF NOT EXISTS categories (
  id          INTEGER PRIMARY KEY,
  name        TEXT    NOT NULL,
  slug        TEXT    NOT NULL UNIQUE,
  created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_categories_slug ON categories(slug);

-- Relación muchos-a-muchos entre medios y categorías.
CREATE TABLE IF NOT EXISTS media_categories (
  media_id    INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE,
  category_id INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
  PRIMARY KEY (media_id, category_id)
);

CREATE INDEX IF NOT EXISTS idx_media_categories_media ON media_categories(media_id);
CREATE INDEX IF NOT EXISTS idx_media_categories_category ON media_categories(category_id);

-- Puntuación obtenida de los metadatos (vote_average de TVmaze), separada de
-- la calificación de usuario (rating, 0-5 estrellas).
ALTER TABLE media ADD COLUMN vote_average REAL;

-- Imagen específica de episodio (thumbnail), independiente del póster de la
-- serie. Se rellena cuando la fuente de metadatos proporciona una imagen por
-- episodio; de lo contrario se usa un fallback visual.
ALTER TABLE media_files ADD COLUMN episode_image_path TEXT;
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
    Migration {
        version: 3,
        name: "media-parent-id",
        sql: MEDIA_PARENT_ID_SQL,
    },
    Migration {
        version: 4,
        name: "media-streams",
        sql: MEDIA_STREAMS_SQL,
    },
    Migration {
        version: 5,
        name: "fts-triggers-repair",
        sql: FTS_TRIGGERS_REPAIR_SQL,
    },
    Migration {
        version: 6,
        name: "media-banner-path",
        sql: MEDIA_BANNER_PATH_SQL,
    },
    Migration {
        version: 7,
        name: "media-file-tracks",
        sql: MEDIA_FILE_TRACKS_SQL,
    },
    Migration {
        version: 8,
        name: "media-rating",
        sql: MEDIA_RATING_SQL,
    },
    Migration {
        version: 9,
        name: "media-categories-and-vote",
        sql: MEDIA_CATEGORIES_AND_VOTE_SQL,
    },
];
