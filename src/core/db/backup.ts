import { chmodSync, copyFileSync, rmSync } from 'node:fs';

import { DbError, formatError } from '../../shared/errors.js';

import type { Db } from './connection.js';

/** Copia de seguridad de la base de datos, creada antes de una operación destructiva. */
export type DatabaseBackup = {
  path: string;
  createdAt: string;
};

/**
 * Marca temporal compacta en UTC (`20261023T101500123Z`), válida en cualquier
 * sistema de ficheros, ordenable alfabéticamente y con milisegundos para que dos
 * backups seguidos no colisionen.
 */
export function backupTimestamp(now: Date = new Date()): string {
  return now.toISOString().replace(/[-:.]/g, '');
}

/**
 * Copia la base de datos a `<dbPath>.bak-<etiqueta>-<timestamp>`.
 *
 * Se consolida antes el WAL para que el fichero principal contenga todo lo
 * confirmado; si la copia falla, se elimina el resto parcial y se lanza `DbError`
 * para que el llamante aborte la operación destructiva.
 */
export function backupDatabase(
  db: Db,
  dbPath: string,
  label: string,
  now: Date = new Date(),
): DatabaseBackup {
  if (dbPath === ':memory:') {
    throw new DbError('No se puede copiar una base de datos en memoria.');
  }

  const target = `${dbPath}.bak-${label}-${backupTimestamp(now)}`;
  try {
    db.pragma('wal_checkpoint(TRUNCATE)');
    copyFileSync(dbPath, target);
    // La base contiene rutas e historial de reproducción: nunca debe ser legible
    // por otros usuarios de la máquina.
    chmodSync(target, 0o600);
  } catch (error) {
    rmSync(target, { force: true });
    throw new DbError(
      `No se pudo crear el backup de la base de datos en ${target}: ${formatError(error)}`,
      { cause: error },
    );
  }

  return { path: target, createdAt: now.toISOString() };
}
