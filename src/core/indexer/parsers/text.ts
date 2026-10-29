import path from 'node:path';

/**
 * Etiquetas habituales en los nombres de releases que no forman parte del título:
 * calidad, códec, audio, idioma o edición. Se eliminan al normalizar un título.
 */
const RELEASE_TAG_PATTERN =
  /\b(?:2160p|1080p|720p|480p|4k|uhd|blu-?ray|bd-?rip|br-?rip|web[ -]?dl|web[ -]?rip|hdtv|dvd-?rip|x264|x265|h[ .]?264|h[ .]?265|hevc|avc|10bit|8bit|aac|ac3|eac3|dts(?:-hd)?|ddp?5[ .]1|truehd|atmos|remux|proper|repack|extended|unrated|french|vostfr|multi|dual|latino|castellano|subtitulado)\b/gi;

/** Un título con su año de estreno, si se ha podido determinar. */
export type TitleYear = {
  title: string;
  year: number | null;
};

/** Quita la extensión del nombre de un fichero. */
export function stripExtension(fileName: string): string {
  const extension = path.extname(fileName);
  // Sin extensión, extname devuelve ""; con nombres como "Show.S01E02" devuelve
  // ".S01E02", que no es una extensión real y no debe recortarse.
  return extension.length > 1 && extension.length <= 6
    ? fileName.slice(0, -extension.length)
    : fileName;
}

/**
 * Normaliza un nombre de release: separa por espacios, elimina grupos entre
 * corchetes y etiquetas de calidad. Es determinista y no lanza excepciones.
 */
export function normalizeTitle(raw: string): string {
  const withoutGroups = raw.replace(/\[([^\]]*)\]/g, (group, content: string) =>
    isYear(content) ? ` ${content} ` : ' ',
  );

  // Las etiquetas se eliminan antes de convertir puntos en espacios para que
  // nombres como `DDP5.1` o `H.264` sigan siendo reconocibles.
  return withoutGroups
    .replace(RELEASE_TAG_PATTERN, ' ')
    .replace(/[._]+/g, ' ')
    .replace(/\s-[A-Za-z0-9]{2,}$/, ' ')
    .replace(/\s+/g, ' ')
    .replace(/^[\s\-–—._]+|[\s\-–—._]+$/g, '')
    .trim();
}

/**
 * Extrae el título y el año de un nombre de carpeta o fichero (sin extensión).
 *
 * El año se busca como token de cuatro cifras en un rango razonable (1888 hasta
 * el año siguiente al actual). Si el nombre es solo un año (`1984`, `1917`), se
 * conserva como título: es más probable que sea el título de la película que su
 * año de estreno.
 */
export function parseTitleAndYear(
  raw: string,
  currentYear = new Date().getFullYear(),
): TitleYear | null {
  const normalized = raw
    .replace(/\[([^\]]*)\]/g, (group, content: string) => (isYear(content) ? ` ${content} ` : ' '))
    .replace(/[._]+/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();

  const candidates = [...normalized.matchAll(/\b(19\d{2}|20\d{2})\b/g)]
    .map((match) => ({
      value: Number(match[1]),
      index: match.index,
      length: match[0].length,
    }))
    .filter(
      (candidate) =>
        candidate.value >= 1888 &&
        candidate.value <= currentYear + 1 &&
        candidate.index !== undefined,
    );

  const chosen = candidates.at(-1);
  let titleSource = normalized;
  let year: number | null = null;

  if (chosen?.index !== undefined) {
    const before = normalized
      .slice(0, chosen.index)
      .replace(/[\s\-–—._([\]]+$/, '')
      .trim();
    const after = normalized
      .slice(chosen.index + chosen.length)
      .replace(/^[\s\-–—._)\]]+/, '')
      .trim();

    // Si el nombre era únicamente el año, se trata como parte del título.
    if (before.length > 0 || after.length > 0) {
      year = chosen.value;
      titleSource = `${before} ${after}`;
    }
  }

  const title = normalizeTitle(titleSource);
  return title.length > 0 ? { title, year } : null;
}

/** Comprueba si un texto entre corchetes es un año (`[2022]`). */
function isYear(value: string): boolean {
  return /^\d{4}$/.test(value);
}
