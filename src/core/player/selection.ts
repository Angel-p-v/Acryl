import { PlayerError } from '../../shared/errors.js';

import type { MediaFileRow } from '../db/types.js';

/**
 * Elige el fichero que se va a reproducir.
 *
 * Por defecto se usa el primero que devuelve la biblioteca (ordenado por
 * temporada y episodio, así que en una serie es el primer capítulo). `fileId`
 * permite seleccionar uno concreto de los que aparecen en `mm show`.
 */
export function selectMediaFile(files: readonly MediaFileRow[], fileId?: number): MediaFileRow {
  const [first] = files;
  if (first === undefined) {
    throw new PlayerError('El título no tiene ficheros indexados que se puedan reproducir.');
  }

  if (fileId === undefined) return first;

  const selected = files.find((file) => file.id === fileId);
  if (selected === undefined) {
    const available = files.map((file) => `#${file.id}`).join(', ');
    throw new PlayerError(
      `El fichero #${fileId} no pertenece a este título. Ficheros disponibles: ${available}.`,
    );
  }

  return selected;
}
