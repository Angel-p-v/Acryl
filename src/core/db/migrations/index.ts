/** Una migración de esquema: número de versión, nombre y SQL aplicable. */
export type Migration = {
  readonly version: number;
  readonly name: string;
  readonly sql: string;
};

const initSql = `
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
`;

/**
 * Migraciones del esquema, en orden. Se declaran en TypeScript (y no como ficheros
 * `.sql`) para que viajen compiladas en `dist/` y no dependan de rutas en disco
 * al ejecutar el paquete instalado.
 */

// v2: índices de apoyo para los listados ordenados y para el mantenimiento. La
// migración es solo de rendimiento; no cambia datos ni el contrato de las tablas.
const performanceIndexesSql = `
CREATE INDEX IF NOT EXISTS idx_media_type_sort_title ON media(type, sort_title);
CREATE INDEX IF NOT EXISTS idx_media_created_at ON media(created_at);
CREATE INDEX IF NOT EXISTS idx_media_updated_at ON media(updated_at);
CREATE INDEX IF NOT EXISTS idx_index_runs_started_at ON index_runs(started_at);
`;

// v3: estado de reproducción y preferencias. Aditiva: no toca las tablas
// existentes ni FTS5, y el estado se borra en cascada con su fichero o título.
const playbackAndPreferencesSql = `
CREATE TABLE IF NOT EXISTS playback_state (
  media_file_id    INTEGER PRIMARY KEY REFERENCES media_files(id) ON DELETE CASCADE,
  position_seconds REAL    NOT NULL DEFAULT 0 CHECK (position_seconds >= 0),
  duration_seconds REAL    CHECK (duration_seconds IS NULL OR duration_seconds > 0),
  completed_at     TEXT,
  last_played_at   TEXT    NOT NULL,
  play_count       INTEGER NOT NULL DEFAULT 0 CHECK (play_count >= 0),
  updated_at       TEXT    NOT NULL
);

CREATE TABLE IF NOT EXISTS media_preferences (
  media_id   INTEGER PRIMARY KEY REFERENCES media(id) ON DELETE CASCADE,
  favorite   INTEGER NOT NULL DEFAULT 0 CHECK (favorite IN (0, 1)),
  created_at TEXT    NOT NULL,
  updated_at TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_playback_last_played ON playback_state(last_played_at);
CREATE INDEX IF NOT EXISTS idx_playback_completed ON playback_state(completed_at);
CREATE INDEX IF NOT EXISTS idx_preferences_favorite ON media_preferences(favorite);
`;

// v4: etiquetas/colecciones. Aditiva: tabla tags + media_tags (muchos-a-muchos).
const tagsSql = `
CREATE TABLE IF NOT EXISTS tags (
  id         INTEGER PRIMARY KEY,
  name       TEXT    NOT NULL UNIQUE,
  color      TEXT,
  created_at TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS media_tags (
  media_id INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE,
  tag_id   INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  PRIMARY KEY (media_id, tag_id)
);

CREATE INDEX IF NOT EXISTS idx_media_tags_tag ON media_tags(tag_id);
CREATE INDEX IF NOT EXISTS idx_media_tags_media ON media_tags(media_id);
`;

// v5: metadatos externos (TMDB/TVDB/TVMaze/Jikan). Aditiva: tabla media_metadata.
const metadataSql = `
CREATE TABLE IF NOT EXISTS media_metadata (
  media_id         INTEGER PRIMARY KEY REFERENCES media(id) ON DELETE CASCADE,
  provider         TEXT    NOT NULL CHECK (provider IN ('tmdb', 'tvdb', 'tvmaze', 'jikan')),
  external_id      INTEGER NOT NULL,
  title            TEXT,
  overview         TEXT,
  poster_path      TEXT,
  backdrop_path    TEXT,
  vote_average     REAL,
  first_air_date   TEXT,
  release_date     TEXT,
  original_language TEXT,
  fetched_at       TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at       TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_metadata_provider ON media_metadata(provider, external_id);
`;

// v6: caché local de imágenes. Aditiva: columna local_poster_path en media_metadata.
const posterCacheSql = `
ALTER TABLE media_metadata ADD COLUMN local_poster_path TEXT;
`;

// v7: actualizar CHECK constraint de provider para incluir tvmaze y jikan.
// SQLite no permite ALTER TABLE para modificar CHECK constraints, así que
// recreamos la tabla copiando los datos existentes.
const metadataProviderUpdateSql = `
CREATE TABLE IF NOT EXISTS media_metadata_new (
  media_id         INTEGER PRIMARY KEY REFERENCES media(id) ON DELETE CASCADE,
  provider         TEXT    NOT NULL CHECK (provider IN ('tmdb', 'tvdb', 'tvmaze', 'jikan')),
  external_id      INTEGER NOT NULL,
  title            TEXT,
  overview         TEXT,
  poster_path      TEXT,
  backdrop_path    TEXT,
  vote_average     REAL,
  first_air_date   TEXT,
  release_date     TEXT,
  original_language TEXT,
  fetched_at       TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at       TEXT NOT NULL,
  local_poster_path TEXT
);

INSERT OR REPLACE INTO media_metadata_new
SELECT media_id, provider, external_id, title, overview, poster_path, backdrop_path,
       vote_average, first_air_date, release_date, original_language, fetched_at, updated_at, local_poster_path
FROM media_metadata;

DROP TABLE media_metadata;

ALTER TABLE media_metadata_new RENAME TO media_metadata;

CREATE INDEX IF NOT EXISTS idx_metadata_provider ON media_metadata(provider, external_id);
`;

export const migrations: readonly Migration[] = [
  { version: 1, name: 'init', sql: initSql },
  { version: 2, name: 'performance-indexes', sql: performanceIndexesSql },
  { version: 3, name: 'playback-and-preferences', sql: playbackAndPreferencesSql },
  { version: 4, name: 'tags-and-collections', sql: tagsSql },
  { version: 5, name: 'external-metadata', sql: metadataSql },
  { version: 6, name: 'poster-cache', sql: posterCacheSql },
  { version: 7, name: 'metadata-provider-update', sql: metadataProviderUpdateSql },
];
