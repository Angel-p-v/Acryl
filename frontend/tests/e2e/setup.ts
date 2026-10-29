import { afterAll } from 'vitest';
import { execSync } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

/**
 * Configura el entorno de pruebas E2E.
 *
 * Crea una base de datos SQLite temporal con medios de prueba para que
 * las pruebas E2E puedan ejecutarse de forma aislada y determinística.
 */

export const E2E_DB_PATH = join(mkdtempSync(join(tmpdir(), 'acryl-e2e-')), 'library.db');

/**
 * Inserta medios de prueba en la base de datos SQLite temporal.
 */
function seedTestDatabase(dbPath: string): void {
  const schema = [
    'CREATE TABLE IF NOT EXISTS media (',
    '  id INTEGER PRIMARY KEY,',
    "  type TEXT NOT NULL CHECK (type IN ('series', 'anime', 'movie')),",
    '  title TEXT NOT NULL,',
    '  sort_title TEXT NOT NULL,',
    '  year INTEGER,',
    '  folder_path TEXT NOT NULL UNIQUE,',
    "  created_at TEXT NOT NULL DEFAULT (datetime('now')),",
    "  updated_at TEXT NOT NULL DEFAULT (datetime('now')),",
    '  is_favorite INTEGER NOT NULL DEFAULT 0 CHECK (is_favorite IN (0, 1)),',
    '  is_watched INTEGER NOT NULL DEFAULT 0 CHECK (is_watched IN (0, 1)),',
    '  poster_path TEXT,',
    '  synopsis TEXT,',
    '  metadata_fetched_at TEXT',
    ');',
    'CREATE TABLE IF NOT EXISTS media_files (',
    '  id INTEGER PRIMARY KEY,',
    '  media_id INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE,',
    '  season INTEGER,',
    '  episode INTEGER,',
    '  title TEXT,',
    '  file_path TEXT NOT NULL UNIQUE,',
    '  size_bytes INTEGER,',
    '  mtime_ms INTEGER,',
    "  indexed_at TEXT NOT NULL DEFAULT (datetime('now'))",
    ');',
    'CREATE VIRTUAL TABLE IF NOT EXISTS media_fts USING fts5(',
    '  title,',
    "  content='media',",
    "  content_rowid='id'",
    ');',
    'CREATE TRIGGER IF NOT EXISTS media_fts_insert AFTER INSERT ON media BEGIN',
    '  INSERT INTO media_fts(rowid, title) VALUES (new.id, new.title);',
    'END;',
    'CREATE TRIGGER IF NOT EXISTS media_fts_delete AFTER DELETE ON media BEGIN',
    "  INSERT INTO media_fts(media_fts, rowid, title) VALUES ('delete', old.id, old.title);",
    'END;',
    'CREATE TRIGGER IF NOT EXISTS media_fts_update AFTER UPDATE ON media BEGIN',
    "  INSERT INTO media_fts(media_fts, rowid, title) VALUES ('delete', old.id, old.title);",
    '  INSERT INTO media_fts(rowid, title) VALUES (new.id, new.title);',
    'END;',
  ].join('\n');

  execSync(`sqlite3 "${dbPath}" "${schema}"`, { stdio: 'pipe' });

  const testMedia = [
    {
      type: 'movie',
      title: 'Arrival',
      sort_title: 'arrival',
      year: 2016,
      folder_path: '/media/movies/Arrival',
      file_path: '/media/movies/Arrival/Arrival.2016.1080p.mkv',
    },
    {
      type: 'series',
      title: 'Severance',
      sort_title: 'severance',
      year: 2022,
      folder_path: '/media/series/Severance',
      file_path: '/media/series/Severance/Season 01/Severance.S01E01.1080p.mkv',
    },
    {
      type: 'anime',
      title: 'Frieren',
      sort_title: 'frieren',
      year: 2023,
      folder_path: '/media/anime/Frieren',
      file_path: '/media/anime/Frieren/Frieren - 01 (1080p).mkv',
    },
  ];

  for (const m of testMedia) {
    const sql = [
      `INSERT INTO media (type, title, sort_title, year, folder_path, is_favorite, is_watched) `,
      `VALUES ('${m.type}', '${m.title}', '${m.sort_title}', ${m.year}, '${m.folder_path}', 0, 0);`,
      `INSERT INTO media_files (media_id, season, episode, title, file_path, size_bytes) `,
      `VALUES (last_insert_rowid(), ${m.type === 'movie' ? 'NULL' : 1}, ${m.type === 'movie' ? 'NULL' : 1}, `,
      `'${m.title} Episode 1', '${m.file_path}', 1000000);`,
      `INSERT INTO media_fts(rowid, title) VALUES (last_insert_rowid(), '${m.title}');`,
    ].join(' ');

    execSync(`sqlite3 "${dbPath}" "${sql}"`, { stdio: 'pipe' });
  }
}

try {
  seedTestDatabase(E2E_DB_PATH);
} catch (error) {
  console.error('No se pudo configurar la base de datos de pruebas E2E:', error);
  throw error;
}

afterAll(() => {
  try {
    rmSync(E2E_DB_PATH, { force: true });
    const tmpDir = E2E_DB_PATH.split('/').slice(0, -1).join('/');
    rmSync(tmpDir, { recursive: true, force: true });
  } catch {
    // Ignorar errores de limpieza
  }
});
