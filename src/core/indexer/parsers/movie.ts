import path from 'node:path';

import { parseTitleAndYear, stripExtension, type TitleYear } from './text.js';

/**
 * Extrae título y año del nombre de una película.
 * Devuelve `null` cuando, tras limpiar etiquetas de release, no queda título.
 */
export function parseMovie(filePath: string): TitleYear | null {
  const name = stripExtension(path.basename(filePath));
  return parseTitleAndYear(name);
}
