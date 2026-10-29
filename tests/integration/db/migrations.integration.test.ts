import Database from 'better-sqlite3';
import { describe, expect, it } from 'vitest';

import { runMigrations } from '../../../src/core/db/connection.js';
import { migrations } from '../../../src/core/db/migrations/index.js';

type MigrationRow = { version: number; name: string };

describe('migraciones de esquema (integración)', () => {
  it('actualiza una base v0.2 (esquema v1) a v2 sin perder datos', () => {
    const db = new Database(':memory:');
    const v1 = migrations.find((migration) => migration.version === 1);
    expect(v1).toBeDefined();

    try {
      runMigrations(db, [v1!]);

      db.prepare(
        `INSERT INTO media (type, title, sort_title, year, folder_path)
         VALUES ('series', 'Legacy Show', 'legacy show', 2019, '/library/Legacy Show')`,
      ).run();
      db.prepare(
        `INSERT INTO media_files (media_id, season, episode, file_path)
         VALUES (1, 1, 1, '/library/Legacy Show/S01E01.mkv')`,
      ).run();
      db.prepare(
        `INSERT INTO index_runs (status, files_seen, files_added)
         VALUES ('completed', 1, 1)`,
      ).run();

      runMigrations(db);

      const versions = db
        .prepare<[], MigrationRow>('SELECT version, name FROM schema_migrations ORDER BY version')
        .all();
      expect(versions).toEqual([
        { version: 1, name: 'init' },
        { version: 2, name: 'performance-indexes' },
        { version: 3, name: 'playback-and-preferences' },
        { version: 4, name: 'tags-and-collections' },
        { version: 5, name: 'external-metadata' },
        { version: 6, name: 'poster-cache' },
        { version: 7, name: 'metadata-provider-update' },
      ]);

      expect(db.prepare('SELECT COUNT(*) AS total FROM media').get()).toEqual({ total: 1 });
      expect(db.prepare('SELECT COUNT(*) AS total FROM media_files').get()).toEqual({ total: 1 });
      expect(
        db.prepare("SELECT rowid FROM media_fts WHERE media_fts MATCH 'legacy'").all(),
      ).toEqual([{ rowid: 1 }]);

      const indexes = db
        .prepare<[string], { name: string }>('SELECT name FROM pragma_index_list(?)')
        .all('media')
        .map((index) => index.name);
      expect(indexes).toContain('idx_media_type_sort_title');
      expect(indexes).toContain('idx_media_created_at');
      expect(indexes).toContain('idx_media_updated_at');

      // Aplicar de nuevo no duplica filas ni vuelve a ejecutar la migración.
      runMigrations(db);
      expect(db.prepare('SELECT COUNT(*) AS total FROM schema_migrations').get()).toEqual({
        total: 7,
      });
    } finally {
      db.close();
    }
  });

  it('actualiza una base v1.0 (esquema v1) a v3 conservando datos y cascada', () => {
    const db = new Database(':memory:');
    const v1 = migrations.find((migration) => migration.version === 1);
    expect(v1).toBeDefined();

    db.pragma('foreign_keys = ON');
    try {
      runMigrations(db, [v1!]);

      db.prepare(
        `INSERT INTO media (type, title, sort_title, year, folder_path)
         VALUES ('series', 'Legacy Show', 'legacy show', 2019, '/library/Legacy Show')`,
      ).run();
      db.prepare(
        `INSERT INTO media_files (media_id, season, episode, file_path)
         VALUES (1, 1, 1, '/library/Legacy Show/S01E01.mkv')`,
      ).run();

      runMigrations(db);

      expect(
        db
          .prepare<[], MigrationRow>('SELECT version, name FROM schema_migrations ORDER BY version')
          .all(),
      ).toEqual([
        { version: 1, name: 'init' },
        { version: 2, name: 'performance-indexes' },
        { version: 3, name: 'playback-and-preferences' },
        { version: 4, name: 'tags-and-collections' },
        { version: 5, name: 'external-metadata' },
        { version: 6, name: 'poster-cache' },
        { version: 7, name: 'metadata-provider-update' },
      ]);

      const tables = db
        .prepare<[], { name: string }>("SELECT name FROM sqlite_master WHERE type = 'table'")
        .all()
        .map((row) => row.name);
      expect(tables).toContain('playback_state');
      expect(tables).toContain('media_preferences');

      expect(db.prepare('SELECT COUNT(*) AS total FROM media').get()).toEqual({ total: 1 });
      expect(
        db.prepare("SELECT rowid FROM media_fts WHERE media_fts MATCH 'legacy'").all(),
      ).toEqual([{ rowid: 1 }]);

      // El estado se borra en cascada con su fichero.
      db.prepare(
        `INSERT INTO playback_state (media_file_id, position_seconds, last_played_at, updated_at)
         VALUES (1, 12.5, '2026-10-23T10:00:00.000Z', '2026-10-23T10:00:00.000Z')`,
      ).run();
      db.prepare('DELETE FROM media_files WHERE id = 1').run();
      expect(db.prepare('SELECT COUNT(*) AS total FROM playback_state').get()).toEqual({
        total: 0,
      });
    } finally {
      db.close();
    }
  });
});
