import path from 'node:path';

/** Extensiones de vídeo que el indexador considera parte de la biblioteca. */
export const VIDEO_EXTENSIONS: readonly string[] = [
  'mkv',
  'mp4',
  'm4v',
  'avi',
  'mov',
  'webm',
  'wmv',
  'flv',
  'mpg',
  'mpeg',
  'm2ts',
  'ts',
  'ogv',
];

/** Comprueba si una ruta tiene extensión de vídeo (sin distinguir mayúsculas). */
export function isVideoFile(filePath: string): boolean {
  const extension = path.extname(filePath).slice(1).toLowerCase();
  return VIDEO_EXTENSIONS.includes(extension);
}
