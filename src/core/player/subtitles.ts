import { readdirSync } from 'node:fs';
import path from 'node:path';

/** Extensiones de archivo de subtítulo reconocidas. */
export const SUBTITLE_EXTENSIONS = ['.srt', '.sub', '.vtt', '.ass', '.ssa'] as const;

export type SubtitleExtension = (typeof SUBTITLE_EXTENSIONS)[number];

/** Comprueba si una ruta es un archivo de subtítulo por su extensión. */
export function isSubtitleFile(filePath: string): boolean {
  const ext = path.extname(filePath).toLowerCase();
  return SUBTITLE_EXTENSIONS.includes(ext as SubtitleExtension);
}

/**
 * Busca archivos de subtítulo asociados a un fichero de vídeo.
 *
 * Los subtítulos asociados están en el mismo directorio y comparten el mismo
 * nombre base que el vídeo (p. ej. `episodio.mkv` → `episodio.srt`,
 * `episodio.en.srt`, `episodio.spa.srt`).
 *
 * Devuelve las rutas ordenadas alfabéticamente para resultados estables.
 */
export function findSubtitleFiles(videoFilePath: string): string[] {
  const directory = path.dirname(videoFilePath);
  const baseName = path.basename(videoFilePath, path.extname(videoFilePath));

  let entries: string[];
  try {
    entries = readdirSync(directory);
  } catch {
    // El directorio no existe o no es legible: no hay subtítulos asociados.
    return [];
  }

  return entries
    .filter((entry) => {
      const entryBase = path.basename(entry, path.extname(entry));
      // El nombre base del subtítulo debe empezar por el nombre base del vídeo
      // (permite sufijos de idioma como .en, .spa, etc.).
      return (
        entryBase.startsWith(baseName) &&
        isSubtitleFile(entry) &&
        // Excluir el propio fichero de vídeo si casualmente coincide
        entry !== path.basename(videoFilePath)
      );
    })
    .map((entry) => path.join(directory, entry))
    .sort();
}

/** Devuelve la ruta del primer archivo de subtítulo asociado, si existe. */
export function findSubtitleFile(videoFilePath: string): string | null {
  const subtitles = findSubtitleFiles(videoFilePath);
  if (subtitles.length === 0) {
    return null;
  }
  // length > 0 garantiza que subtitles[0] está definido.
  const first = subtitles[0];
  if (first === undefined) return null;
  return first;
}
