import { statSync } from 'node:fs';

/**
 * Tamaño ocupado por la base de datos, incluyendo WAL y SHM.
 *
 * `:memory:` no tiene fichero; devolver 0 evita que los comandos de estadísticas
 * intenten leer una ruta inexistente.
 */
export function databaseSizeBytes(databasePath: string): number {
  if (databasePath === ':memory:') return 0;

  return [`${databasePath}`, `${databasePath}-wal`, `${databasePath}-shm`].reduce((total, file) => {
    try {
      return total + statSync(file).size;
    } catch {
      // Un fichero auxiliar puede no existir todavía; su tamaño es cero.
      return total;
    }
  }, 0);
}
