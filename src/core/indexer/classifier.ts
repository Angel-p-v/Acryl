import path from 'node:path';

import { parseEpisode } from './parsers/episode.js';
import { parseMovie } from './parsers/movie.js';
import { parseTitleAndYear, stripExtension } from './parsers/text.js';

import type { LibraryRoot, MediaType } from '../config/schema.js';

/** Un fichero de vídeo con los datos que necesita la base de datos. */
export type ClassifiedFile = {
  type: MediaType;
  title: string;
  year: number | null;
  /** Carpeta del título. Para películas es la ruta del propio fichero. */
  folderPath: string;
  filePath: string;
  sizeBytes: number;
  mtimeMs: number;
  season: number | null;
  episode: number | null;
};

export type ClassifyInput = {
  root: LibraryRoot;
  filePath: string;
  sizeBytes: number;
  mtimeMs: number;
};

/**
 * Limpia patrones de episodio (S01E02, 1x02, etc.) de un nombre de fichero
 * para obtener el título del título/serie.
 */
function extractCleanTitle(fileName: string): string {
  return (
    fileName
      .replace(/\bS\d{1,2}[ ._-]*(?:E|EP)?[ ._-]*\d{1,3}(?=\D|$)/i, ' ')
      .replace(/\bT\d{1,2}[ ._-]*(?:E|EP)?[ ._-]*\d{1,3}(?=\D|$)/i, ' ')
      .replace(/\b\d{1,2}x\d{1,3}(?=\D|$)/i, ' ')
      .replace(/\bSeason[ ._-]*\d{1,2}[ ._-]*(?:Episode|Ep|E)[ ._-]*\d{1,3}(?=\D|$)/i, ' ')
      .replace(/\bSeason\s*\d{1,2}\b/gi, ' ')
      .replace(/\bTemporada\s*\d{1,2}\b/gi, ' ')
      .replace(/\b(?:Episode|Ep|E)[ ._-]*\d{1,3}(?=\D|$)/i, ' ')
      .replace(/(?:^|[\s._])-\s*\d{1,3}(?:\s*v\d+)?(?=\s*(?:-|\(|\[|$))/i, ' ')
      // Quita sufijos tipo v2, v3 que quedan tras limpiar el episodio
      .replace(/\s*v\d+$/i, ' ')
      .replace(/[._]+/g, ' ')
      .replace(/\s+/g, ' ')
      .replace(/^[\s\-–—._]+|[\s\-–—._]+$/g, '')
      .trim()
  );
}

/**
 * Heurística para inferir el tipo de contenido a partir del nombre de una
 * carpeta contenedora. Se usa como fallback cuando el nombre del fichero no
 * tiene un patrón reconocible (ni de episodio ni de película).
 *
 * Devuelve 'series' cuando el nombre parece una serie (tiene año entre
 * paréntesis, o palabras como 'temporada', 'season'), 'movie' cuando parece
 * una película, y 'unknown' cuando no se puede determinar.
 */
function inferTypeFromFolder(folderName: string): MediaType {
  const lower = folderName.toLowerCase();

  // Palabras clave que indican claramente una serie
  if (/\b(temporada|season|serie|series|tv|show)\b/i.test(lower)) {
    return 'series';
  }

  // Año entre paréntesis al final: típico de series ("Show (2022)") y películas
  if (/\s\(\d{4}\)\s*$/.test(folderName)) {
    return 'series';
  }

  // Año suelto o con separadores: más típico de películas
  if (/\b(19\d{2}|20\d{2})\b/.test(folderName)) {
    return 'movie';
  }

  return 'unknown';
}

/**
 * Asigna un fichero escaneado a un título de la biblioteca.
 *
 * El tipo de contenido se determina inspeccionando el nombre del fichero y
 * la carpeta contenedora, no únicamente el `type` de la `LibraryRoot`. Así
 * una película en una raíz de series (o viceversa) se clasifica correctamente.
 *
 * Devuelve `null` solo para casos realmente ambiguos (ruta fuera de la raíz);
 * en cualquier otro caso devuelve un objeto con tipo 'unknown' como mínimo.
 */
export function classifyFile(input: ClassifyInput): ClassifiedFile | null {
  const relative = path.relative(input.root.path, input.filePath);
  if (relative.length === 0 || relative.startsWith('..') || path.isAbsolute(relative)) {
    return null;
  }

  const basename = stripExtension(path.basename(input.filePath));
  const [folderName, ...rest] = relative.split(path.sep);

  // 1. Intentar detectar patrón de episodio (tiene prioridad sobre todo)
  const episode = parseEpisode(input.filePath);
  if (episode) {
    const isStandaloneFile = rest.length === 0;
    let cleanTitle: string;
    let year: number | null = null;
    if (isStandaloneFile) {
      cleanTitle = extractCleanTitle(basename);
    } else if (folderName) {
      const parsed = parseTitleAndYear(folderName);
      cleanTitle = parsed?.title ?? folderName;
      // Limpiar residuos de temporada que puedan quedar en el nombre de la
      // carpeta raíz (ej. "Solo Leveling Season 1" → "Solo Leveling")
      cleanTitle = cleanTitle
        .replace(/\bSeason\s*\d{1,2}\b/gi, ' ')
        .replace(/\bTemporada\s*\d{1,2}\b/gi, ' ')
        .replace(/\bS\d{1,2}\b/gi, ' ')
        .replace(/\bT\d{1,2}\b/gi, ' ')
        .replace(/\s+/g, ' ')
        .trim();
      year = parsed?.year ?? null;
    } else {
      cleanTitle = basename;
    }
    return {
      type: input.root.type === 'movie' ? 'series' : input.root.type,
      title: cleanTitle,
      year,
      folderPath:
        !isStandaloneFile && folderName ? path.join(input.root.path, folderName) : input.filePath,
      filePath: input.filePath,
      sizeBytes: input.sizeBytes,
      mtimeMs: input.mtimeMs,
      season: episode.season,
      episode: episode.episode,
    };
  }

  // 2. Intentar detectar patrón de película.
  //   - Si el nombre tiene año detectable, es claramente una película.
  //   - Si no tiene año pero la raíz está configurada como 'movie', confiamos
  //     en la configuración del usuario.
  //   - En cualquier otro caso (raíz de series/anime sin año), es ambiguo:
  //     se deja para la heurística de carpeta padre o 'unknown'.
  const movie = parseMovie(input.filePath);
  if (movie && (movie.year !== null || input.root.type === 'movie')) {
    return {
      type: 'movie',
      title: movie.title,
      year: movie.year,
      folderPath: input.filePath,
      filePath: input.filePath,
      sizeBytes: input.sizeBytes,
      mtimeMs: input.mtimeMs,
      season: null,
      episode: null,
    };
  }

  // 3. Heurística de carpeta padre
  if (folderName && rest.length > 0) {
    const inferred = inferTypeFromFolder(folderName);
    const title = parseTitleAndYear(folderName);
    return {
      type: inferred,
      title: title?.title ?? folderName,
      year: title?.year ?? null,
      folderPath: path.join(input.root.path, folderName),
      filePath: input.filePath,
      sizeBytes: input.sizeBytes,
      mtimeMs: input.mtimeMs,
      season: null,
      episode: null,
    };
  }

  // 4. Fichero suelto en la raíz sin patrón reconocible
  return {
    type: 'unknown',
    title: basename,
    year: null,
    folderPath: input.root.path,
    filePath: input.filePath,
    sizeBytes: input.sizeBytes,
    mtimeMs: input.mtimeMs,
    season: null,
    episode: null,
  };
}
