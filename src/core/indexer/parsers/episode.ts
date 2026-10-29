import path from 'node:path';

import { stripExtension } from './text.js';

/** Temporada y episodio detectados en el nombre de un fichero. */
export type EpisodeInfo = {
  /** `null` cuando el nombre usa numeración absoluta (`Show - 03`). */
  season: number | null;
  episode: number;
};

type EpisodePattern = {
  readonly pattern: RegExp;
  readonly toInfo: (match: RegExpMatchArray) => EpisodeInfo;
};

/**
 * Patrones ordenados de más específico a más genérico. El primero que coincide
 * gana: así `Show.S01E02.1x03.mkv` se interpreta como S01E02 de forma estable.
 * Los cuantificadores de los grupos están acotados, de modo que los valores
 * capturados siempre son números válidos.
 */
const PATTERNS: readonly EpisodePattern[] = [
  {
    // S01E02, S01.E02, S01 E02, S01EP02, S01E01v2 (con sufijos tipo v2)
    pattern: /\bS(\d{1,2})[ ._-]*E(?:P)?[ ._-]*(\d{1,3})(?=\D|$)/i,
    toInfo: (match) => ({ season: Number(match[1]), episode: Number(match[2]) }),
  },
  {
    // T1E2 (temporada/episodio en español)
    pattern: /\bT(\d{1,2})[ ._-]*E(?:P)?[ ._-]*(\d{1,3})(?=\D|$)/i,
    toInfo: (match) => ({ season: Number(match[1]), episode: Number(match[2]) }),
  },
  {
    // 1x02, 01x02 (pero no 1920x1080: el \b impide empezar en mitad de un número)
    pattern: /\b(\d{1,2})x(\d{1,3})(?=\D|$)/i,
    toInfo: (match) => ({ season: Number(match[1]), episode: Number(match[2]) }),
  },
  {
    // Season 1 Episode 2, Season.1.Ep.2
    pattern: /\bSeason[ ._-]*(\d{1,2})[ ._-]*(?:Episode|Ep|E)[ ._-]*(\d{1,3})(?=\D|$)/i,
    toInfo: (match) => ({ season: Number(match[1]), episode: Number(match[2]) }),
  },
  {
    // E03, Ep. 3, Episode 12 (sin temporada)
    pattern: /\b(?:Episode|Ep|E)[ ._-]*(\d{1,3})(?=\D|$)/i,
    toInfo: (match) => ({ season: null, episode: Number(match[1]) }),
  },
  {
    // Numeración absoluta: "Show - 03", "Serie - 12 - Título", "Show - 03v2 (1080p)"
    pattern: /(?:^|[\s._])-\s*(\d{1,3})(?:\s*v\d+)?(?=\s*(?:-|\(|\[|$))/i,
    toInfo: (match) => ({ season: null, episode: Number(match[1]) }),
  },
];

/**
 * Extrae temporada y episodio del nombre de un fichero.
 * Devuelve `null` de forma explícita cuando el nombre es ambiguo o no contiene
 * un patrón reconocible; nunca lanza una excepción.
 */
export function parseEpisode(filePath: string): EpisodeInfo | null {
  const name = stripExtension(path.basename(filePath)).replaceAll('_', ' ');

  for (const { pattern, toInfo } of PATTERNS) {
    const match = pattern.exec(name);
    if (match) return toInfo(match);
  }

  return null;
}
