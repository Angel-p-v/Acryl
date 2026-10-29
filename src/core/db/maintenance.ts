import { DbError, formatError } from '../../shared/errors.js';

import type { Db } from './connection.js';

/** Operaciones de mantenimiento que la CLI puede ejecutar sobre la base de datos. */
export type DatabaseMaintenance = {
  optimize: () => void;
  vacuum: () => void;
};

/**
 * `PRAGMA optimize` deja que SQLite actualice sus estadísticas internas; es barato
 * y se ejecuta también al cerrar. `VACUUM` reconstruye el fichero y solo se lanza
 * de forma explícita porque puede tardar y necesita espacio temporal.
 */
export function createDatabaseMaintenance(db: Db): DatabaseMaintenance {
  return {
    optimize: () => {
      optimizeDatabase(db);
    },
    vacuum: () => {
      vacuumDatabase(db);
    },
  };
}

export function optimizeDatabase(db: Db): void {
  try {
    db.pragma('optimize');
  } catch (error) {
    throw new DbError(`No se pudo optimizar la base de datos: ${formatError(error)}`, {
      cause: error,
    });
  }
}

export function vacuumDatabase(db: Db): void {
  try {
    // En WAL, VACUUM necesita consolidar antes el log para que el tamaño final
    // refleje el espacio realmente liberado.
    db.pragma('wal_checkpoint(TRUNCATE)');
    db.exec('VACUUM');
  } catch (error) {
    throw new DbError(`No se pudo compactar la base de datos: ${formatError(error)}`, {
      cause: error,
    });
  }
}
