import { mkdirSync } from 'node:fs';
import path from 'node:path';

import Database from 'better-sqlite3';

import { backupDatabase } from './backup.js';
import { optimizeDatabase } from './maintenance.js';
import { migrations, type Migration } from './migrations/index.js';
import { DbError, formatError } from '../../shared/errors.js';

export type Db = Database.Database;

/** Opciones de apertura; `now` permite fijar el reloj de los backups en los tests. */
export type OpenDatabaseOptions = {
  now?: Date;
};

/** Abre (o crea) la base de datos y aplica las pragmas de la aplicación. */
export function openDatabase(dbPath: string, options: OpenDatabaseOptions = {}): Db {
  if (dbPath !== ':memory:') {
    try {
      mkdirSync(path.dirname(dbPath), { recursive: true });
    } catch (error) {
      throw new DbError(`No se pudo crear la carpeta de la base de datos: ${formatError(error)}`, {
        cause: error,
      });
    }
  }

  let db: Db;
  try {
    db = new Database(dbPath);
  } catch (error) {
    throw new DbError(`No se pudo abrir la base de datos: ${formatError(error)}`, {
      cause: error,
    });
  }

  db.pragma('journal_mode = WAL');
  db.pragma('foreign_keys = ON');
  db.pragma('busy_timeout = 5000');

  try {
    runMigrations(db, migrations, { dbPath, ...(options.now ? { now: options.now } : {}) });
  } catch (error) {
    // Una migración fallida no debe dejar el fichero bloqueado por esta conexión.
    db.close();
    throw error;
  }
  return db;
}
/** Opciones de `runMigrations`; el backup solo se crea si se conoce la ruta del fichero. */
export type RunMigrationsOptions = {
  /** Ruta del fichero de la base; sin ella no se crea backup (tests con memoria). */
  dbPath?: string;
  now?: Date;
};

/**
 * Aplica las migraciones pendientes. Es idempotente: aplicada una vez, una migración
 * con la misma versión no se vuelve a ejecutar.
 *
 * Antes de migrar una base que ya tiene esquema se guarda una copia de seguridad
 * (`<dbPath>.bak-v<versión>-<timestamp>`). Si la copia falla, la migración se aborta
 * y la base se queda como estaba.
 */
export function runMigrations(
  db: Db,
  pending: readonly Migration[] = migrations,
  options: RunMigrationsOptions = {},
): void {
  db.exec(`CREATE TABLE IF NOT EXISTS schema_migrations (
    version    INTEGER PRIMARY KEY,
    name       TEXT NOT NULL,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
  )`);

  const applied = new Set(
    db
      .prepare<[], { version: number }>('SELECT version FROM schema_migrations')
      .all()
      .map((row) => row.version),
  );

  const todo = [...pending]
    .sort((a, b) => a.version - b.version)
    .filter((migration) => !applied.has(migration.version));
  if (todo.length === 0) return;

  const fromVersion = applied.size > 0 ? Math.max(...applied) : 0;
  if (fromVersion > 0 && options.dbPath !== undefined && options.dbPath !== ':memory:') {
    backupDatabase(db, options.dbPath, `v${fromVersion}`, options.now);
  }

  for (const migration of todo) {
    try {
      db.exec('BEGIN');
      db.exec(migration.sql);
      db.prepare('INSERT INTO schema_migrations (version, name) VALUES (?, ?)').run(
        migration.version,
        migration.name,
      );
      db.exec('COMMIT');
    } catch (error) {
      db.exec('ROLLBACK');
      throw new DbError(
        `Falló la migración ${migration.version} (${migration.name}): ${formatError(error)}`,
        { cause: error },
      );
    }
  }
}

/** Cierra la base de datos de forma segura. */
export function closeDatabase(db: Db): void {
  if (!db.open) return;
  try {
    // Best effort: un fallo al optimizar no debe impedir cerrar ni ocultar el
    // error original que estuviera propagándose.
    optimizeDatabase(db);
  } catch {
    // Ignorado deliberadamente.
  }
  db.close();
}
