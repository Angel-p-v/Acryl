import { existsSync, readdirSync, statSync } from 'node:fs';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

import Database from 'better-sqlite3';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { backupDatabase } from '../../../src/core/db/backup.js';
import { closeDatabase, openDatabase, runMigrations } from '../../../src/core/db/connection.js';
import { migrations } from '../../../src/core/db/migrations/index.js';
import { DbError } from '../../../src/shared/errors.js';

import type { Db } from '../../../src/core/db/connection.js';

const describeUnix = process.platform === 'win32' ? describe.skip : describe;

function migration(version: number) {
  const found = migrations.find((item) => item.version === version);
  if (!found) throw new Error(`No existe la migración v${version}`);
  return found;
}

/** Crea una base con el esquema 1.0 (v2) y un título con un fichero. */
function createLegacyDatabase(dbPath: string): void {
  const db = new Database(dbPath);
  runMigrations(db, [migration(1), migration(2)]);
  db.prepare(
    `INSERT INTO media (type, title, sort_title, year, folder_path)
     VALUES ('series', 'Legacy Show', 'legacy show', 2019, '/library/Legacy Show')`,
  ).run();
  db.prepare(
    `INSERT INTO media_files (media_id, season, episode, file_path)
     VALUES (1, 1, 1, '/library/Legacy Show/S01E01.mkv')`,
  ).run();
  db.close();
}

function appliedVersions(db: Db): number[] {
  return db
    .prepare<[], { version: number }>('SELECT version FROM schema_migrations ORDER BY version')
    .all()
    .map((row) => row.version);
}

describeUnix('backup antes de migrar (integración)', () => {
  let workDir: string;
  let dbPath: string;

  beforeEach(async () => {
    workDir = await mkdtemp(path.join(tmpdir(), 'mm-backup-'));
    dbPath = path.join(workDir, 'library.db');
    createLegacyDatabase(dbPath);
  });

  afterEach(async () => {
    await rm(workDir, { recursive: true, force: true });
  });

  function backups(): string[] {
    return readdirSync(workDir).filter((name) => name.includes('.bak-'));
  }

  it('copia la base v2 y migra a v3 sin perder datos', () => {
    const now = new Date('2026-10-23T10:15:00.000Z');
    const backupPath = `${dbPath}.bak-v2-20261023T101500000Z`;

    const db = openDatabase(dbPath, { now });
    try {
      expect(appliedVersions(db)).toEqual([1, 2, 3, 4, 5, 6, 7]);
      expect(db.prepare('SELECT COUNT(*) AS total FROM media').get()).toEqual({ total: 1 });
      expect(db.prepare('SELECT COUNT(*) AS total FROM media_files').get()).toEqual({ total: 1 });
    } finally {
      closeDatabase(db);
    }

    expect(existsSync(backupPath)).toBe(true);
    // Solo el dueño debe poder leer una copia con rutas e historial.
    expect(statSync(backupPath).mode & 0o777).toBe(0o600);

    // El backup se hizo antes de migrar: conserva el esquema v2 y los datos.
    const backup = new Database(backupPath, { readonly: true });
    try {
      expect(appliedVersions(backup)).toEqual([1, 2]);
      expect(backup.prepare('SELECT COUNT(*) AS total FROM media').get()).toEqual({ total: 1 });
      const tables = backup
        .prepare<[], { name: string }>("SELECT name FROM sqlite_master WHERE type = 'table'")
        .all()
        .map((row) => row.name);
      expect(tables).not.toContain('playback_state');
    } finally {
      backup.close();
    }
  });

  it('no vuelve a copiar si ya no hay migraciones pendientes', () => {
    closeDatabase(openDatabase(dbPath, { now: new Date('2026-10-23T10:15:00.000Z') }));
    expect(backups()).toHaveLength(1);

    closeDatabase(openDatabase(dbPath, { now: new Date('2026-10-23T11:00:00.000Z') }));

    expect(backups()).toHaveLength(1);
  });

  it('aborta la migración si el backup falla y deja el esquema como estaba', () => {
    const db = new Database(':memory:');
    try {
      runMigrations(db, [migration(1), migration(2)]);

      const missingPath = path.join(workDir, 'no-existe', 'library.db');
      expect(() => runMigrations(db, migrations, { dbPath: missingPath })).toThrow(DbError);

      expect(appliedVersions(db)).toEqual([1, 2]);
      const tables = db
        .prepare<[], { name: string }>("SELECT name FROM sqlite_master WHERE type = 'table'")
        .all()
        .map((row) => row.name);
      expect(tables).not.toContain('playback_state');
    } finally {
      db.close();
    }
  });

  it('rechaza copiar una base en memoria', () => {
    const db = new Database(':memory:');
    try {
      expect(() => backupDatabase(db, ':memory:', 'manual')).toThrow(DbError);
    } finally {
      db.close();
    }
  });
});
