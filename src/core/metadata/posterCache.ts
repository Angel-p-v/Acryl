import { mkdirSync, createWriteStream, existsSync } from 'node:fs';
import { access, readdir, stat, rm, readFile, rename } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join } from 'node:path';
import { pipeline } from 'node:stream/promises';

import { formatError } from '../../shared/errors.js';

import type { Logger } from '../../shared/logger.js';

/** Directorio de almacenamiento permanente de pósters (sin expiración de caché). */
export const DEFAULT_CACHE_DIR = join(homedir(), '.local', 'share', 'media-manager', 'posters');

/** Asegura que el directorio de almacenamiento exista al cargar el módulo. */
mkdirSync(DEFAULT_CACHE_DIR, { recursive: true });

/** Extensiones de imagen permitidas. */
const ALLOWED_EXTENSIONS = ['.jpg', '.jpeg', '.png', '.webp', '.gif'];

/** Tamaño mínimo en bytes para considerar que un fichero es una imagen válida. */
const MIN_IMAGE_BYTES = 100;

/** Cabeceras Content-Type aceptadas para una descarga de póster. */
const ALLOWED_CONTENT_TYPES = ['image/jpeg', 'image/jpg', 'image/png', 'image/webp', 'image/gif'];

/** Magic bytes de formatos de imagen soportados (inicio del fichero). */
const IMAGE_MAGIC_BYTES: Record<string, Buffer> = {
  '.jpg': Buffer.from([0xff, 0xd8, 0xff]),
  '.jpeg': Buffer.from([0xff, 0xd8, 0xff]),
  '.png': Buffer.from([0x89, 0x50, 0x4e, 0x47]),
  '.webp': Buffer.from([0x52, 0x49, 0x46, 0x46]), // RIFF, se confirma 'WEBP' en el offset 8
  '.gif': Buffer.from([0x47, 0x49, 0x46, 0x38]),
};

/** Comprueba si una cadena de Content-Type es un tipo de imagen aceptado. */
export function isAllowedContentType(contentType: string | null): boolean {
  if (!contentType) return false;
  const type = (contentType.split(';')[0] ?? '').trim().toLowerCase();
  return ALLOWED_CONTENT_TYPES.includes(type);
}

/** Comprueba que los primeros bytes del fichero coincidan con magic bytes de imagen. */
export function hasValidMagicBytes(buffer: Buffer, extension: string): boolean {
  const magic = IMAGE_MAGIC_BYTES[extension];
  if (!magic) return false;
  if (buffer.length < magic.length) return false;
  for (let i = 0; i < magic.length; i += 1) {
    if (buffer[i] !== magic[i]) return false;
  }

  // WEBP requiere además la marca 'WEBP' en el offset 8 tras la cabecera RIFF.
  if (extension === '.webp') {
    const webpMarker = buffer.slice(8, 12).toString('ascii');
    return webpMarker === 'WEBP';
  }

  return true;
}

/**
 * Valida que un fichero descargado sea una imagen real y no un cuerpo de error
 * HTML o datos basura. Comprueba Content-Type, magic bytes y tamaño mínimo.
 *
 * @param filePath - Ruta al fichero descargado.
 * @param extension - Extensión esperada (para elegir los magic bytes).
 * @returns `true` si el fichero es una imagen válida.
 */
async function isValidImageFile(filePath: string, extension: string): Promise<boolean> {
  try {
    const statResult = await stat(filePath);
    if (statResult.size < MIN_IMAGE_BYTES) return false;

    const head = await readFile(filePath);
    return hasValidMagicBytes(head, extension);
  } catch {
    return false;
  }
}

/**
 * Sanitiza un título para usarlo como nombre de archivo de sistema.
 *
 * Elimina caracteres no válidos, espacios repetidos y truncación excesiva,
 * dejando un nombre descriptivo y seguro (ej. "black-torch-season-01").
 *
 * @param title - Título de la serie o película (puede contener [Judas], S01E01, etc.)
 * @param contentType - Tipo de contenido: 'series', 'anime' o 'movie'
 * @returns Nombre de archivo sanitizado sin extensión (ej. "black-torch")
 */
export function sanitizeFileName(title: string, contentType?: string): string {
  let cleaned = title;

  // Eliminar etiquetas entre corchetes: [Judas], [Dual Audio], etc.
  cleaned = cleaned.replace(/\[[^\]]*\]/g, ' ');

  // Eliminar especificadores de temporada/episodio: S01E01, 1x01, T1E2, E01, etc.
  cleaned = cleaned.replace(/\bS\d{1,2}\s*(?:E\s*)?\d{1,3}\b/gi, ' ');
  cleaned = cleaned.replace(/\b\d{1,2}x\d{1,3}\b/gi, ' ');
  cleaned = cleaned.replace(/\bT\d{1,2}E\d{1,3}\b/gi, ' ');
  cleaned = cleaned.replace(/\bE\d{1,3}\b/gi, ' ');

  // Eliminar calidad y formato: 1080p, 720p, BluRay, WEBDL, x264, HEVC, etc.
  cleaned = cleaned.replace(/\b(1080p|720p|480p|2160p|4k)\b/gi, ' ');
  cleaned = cleaned.replace(/\b(BluRay|Blu-Ray|BluRay)\b/gi, ' ');
  cleaned = cleaned.replace(/\b(WEBDL|WEB-DL|WebDL)\b/gi, ' ');
  cleaned = cleaned.replace(/\b(WEBRIP|WEB-RIP|WebRip)\b/gi, ' ');
  cleaned = cleaned.replace(/\b(x264|x265|HEVC|H\.264|H\.265)\b/gi, ' ');
  cleaned = cleaned.replace(/\b(HDTV|PDTV|DVDRip|DVD-Rip|BRRip|BDRip)\b/gi, ' ');

  // Eliminar extensiones de archivo
  cleaned = cleaned.replace(/\.(mkv|mp4|avi|mov|wmv|flv|webm|m4v|ts|m2ts)$/gi, ' ');

  // Eliminar año entre paréntesis al final: (2023), (2023-2024)
  cleaned = cleaned.replace(/\s*\(\d{4}(?:-\d{4})?\)\s*$/, ' ');

  // Convertir a minúsculas, reemplazar espacios y caracteres no alfanuméricos por guiones
  cleaned = cleaned
    .toLocaleLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');

  // Truncar a 80 caracteres para evitar nombres excesivamente largos
  cleaned = cleaned.slice(0, 80).replace(/-+$/, '');

  // Añadir sufijo de tipo de contenido si se proporciona y hay nombre base
  if (contentType && cleaned) {
    cleaned = `${cleaned}-${contentType}`;
  } else if (contentType && !cleaned) {
    cleaned = contentType;
  }

  return cleaned || 'poster';
}

/**
 * Obtiene la extensión de archivo a partir de una URL de imagen.
 *
 * Devuelve '.jpg' por defecto si no se puede determinar la extensión.
 */
function getImageExtension(posterUrl: string): string {
  try {
    const url = new URL(posterUrl);
    const pathname = url.pathname.toLowerCase();
    for (const ext of ALLOWED_EXTENSIONS) {
      if (pathname.endsWith(ext)) return ext;
    }
  } catch {
    // URL no válida, usar extensión por defecto
  }
  return '.jpg';
}

/**
 * Construye el nombre de archivo descriptivo para un póster.
 *
 * @param title - Título de la serie o película
 * @param contentType - Tipo de contenido ('series', 'anime', 'movie')
 * @param posterUrl - URL de la imagen original (para obtener la extensión)
 * @returns Nombre de archivo con extensión (ej. "black-torch-series.jpg")
 */
function buildFileName(title: string, contentType: string | undefined, posterUrl: string): string {
  const base = sanitizeFileName(title, contentType);
  const ext = getImageExtension(posterUrl);
  return `${base}${ext}`;
}

/**
 * Descarga un póster desde una URL y lo guarda en el almacenamiento permanente.
 *
 * El archivo se guarda con un nombre descriptivo basado en el título y tipo
 * de contenido (ej. black-torch-series.jpg). Si el archivo ya existe en disco,
 * se reutiliza sin volver a descargar.
 *
 * La descarga se realiza mediante `fetch` + `fs.createWriteStream` y se espera
 * al evento `finish` para confirmar que el búfer se escribió y cerró correctamente.
 *
 * `cacheDir` permite inyectar un directorio alternativo (útil en tests).
 */
export async function cachePoster(
  posterUrl: string | null,
  title: string,
  contentType?: string,
  logger?: Logger,
  cacheDir: string = DEFAULT_CACHE_DIR,
): Promise<string | null> {
  if (!posterUrl) {
    logger?.info('poster.cache.skip', { reason: 'no poster URL', title });
    return null;
  }

  // Asegurar que el directorio de almacenamiento existe antes de escribir
  mkdirSync(cacheDir, { recursive: true });

  const fileName = buildFileName(title, contentType, posterUrl);
  const localPath = join(cacheDir, fileName);

  // Si ya existe en disco, reutilizar sin volver a descargar
  try {
    await access(localPath);
    logger?.info('poster.cache.hit', { title, localPath });
    return localPath;
  } catch {
    // No existe, continuar con la descarga
  }

  try {
    const response = await fetch(posterUrl);
    if (!response.ok) {
      logger?.warn('poster.download.failed', {
        title,
        posterUrl,
        localPath,
        status: response.status,
        statusText: response.statusText,
      });
      return null;
    }

    // Validar Content-Type antes de descargar el cuerpo.
    const contentType = response.headers.get('content-type');
    if (!isAllowedContentType(contentType)) {
      logger?.warn('poster.download.invalid-content-type', {
        title,
        contentType,
        posterUrl,
        localPath,
      });
      return null;
    }

    if (!response.body) {
      logger?.warn('poster.download.no-body', {
        title,
        contentType,
        posterUrl,
        localPath,
      });
      return null;
    }

    // Crear el directorio de nuevo justo antes de abrir el stream de escritura,
    // para evitar fallos silenciosos si la ruta no existe.
    mkdirSync(cacheDir, { recursive: true });

    // Escribir primero en un fichero temporal .part; solo se renombra al
    // destino final si supera la validación de imagen. Esto evita dejar
    // ficheros corruptos o incompletos con el nombre definitivo.
    const partPath = `${localPath}.part`;
    const fileStream = createWriteStream(partPath);

    // pipeline espera al evento 'finish' del stream, confirmando escritura completa
    await pipeline(response.body, fileStream);

    // Validar que el fichero descargado sea una imagen real.
    const extension = getImageExtension(posterUrl);
    const valid = await isValidImageFile(partPath, extension);
    if (!valid) {
      logger?.warn('poster.download.invalid-image', {
        title,
        contentType,
        posterUrl,
        localPath,
        reason: 'magic-bytes o tamaño mínimo no válidos',
      });
      try {
        await rm(partPath);
      } catch {
        // El fichero ya puede no existir; ignorar.
      }
      return null;
    }

    // Renombrar atómicamente de .part al nombre definitivo.
    await rename(partPath, localPath);

    // Verificar que el archivo existe en disco tras la renombrada.
    if (!existsSync(localPath)) {
      logger?.warn('poster.download.missing-file', {
        title,
        contentType,
        posterUrl,
        localPath,
      });
      return null;
    }

    const statResult = await stat(localPath);
    logger?.info('poster.download.finished', {
      title,
      contentType,
      posterUrl,
      localPath,
      bytes: statResult.size,
    });

    return localPath;
  } catch (error) {
    logger?.warn('poster.cache.error', {
      title,
      contentType,
      posterUrl,
      localPath,
      error: formatError(error),
    });
    return null;
  }
}

/**
 * Devuelve la ruta de un póster previamente almacenado para un título, si existe.
 *
 * Se usa como fallback cuando la descarga desde la API externa falla.
 *
 * `cacheDir` permite inyectar un directorio alternativo (útil en tests).
 */
export async function getCachedPoster(
  title: string,
  contentType?: string,
  cacheDir: string = DEFAULT_CACHE_DIR,
): Promise<string | null> {
  try {
    const entries = await readdir(cacheDir);
    const base = sanitizeFileName(title, contentType);

    // Buscar cualquier archivo que comience con el nombre base (cualquier extensión)
    for (const entry of entries) {
      if (entry.startsWith(`${base}.`)) {
        const localPath = join(cacheDir, entry);
        try {
          await access(localPath);
          return localPath;
        } catch {
          // El fichero desapareció entre el listado y el access; seguir buscando
        }
      }
    }
  } catch {
    // El directorio de almacenamiento no existe aún; no hay nada que devolver
  }
  return null;
}

/**
 * Elimina un póster específico del almacenamiento permanente por título y tipo.
 *
 * Se usa cuando se quiere eliminar los metadatos de un título y su póster asociado.
 *
 * `cacheDir` permite inyectar un directorio alternativo (útil en tests).
 */
export async function removeCachedPoster(
  title: string,
  contentType?: string,
  cacheDir: string = DEFAULT_CACHE_DIR,
): Promise<boolean> {
  try {
    const entries = await readdir(cacheDir);
    const base = sanitizeFileName(title, contentType);
    let removed = false;

    for (const entry of entries) {
      if (entry.startsWith(`${base}.`)) {
        const localPath = join(cacheDir, entry);
        try {
          await rm(localPath);
          removed = true;
        } catch {
          // Ignorar errores de eliminación individual
        }
      }
    }

    return removed;
  } catch {
    return false;
  }
}

/**
 * Elimina pósters del almacenamiento permanente.
 *
 * Cuando `maxAgeDays <= 0` borra todos los archivos del directorio.
 * Cuando `maxAgeDays > 0` borra archivos más antiguos que ese umbral de edad.
 *
 * Se mantiene como operación de mantenimiento opcional; el almacenamiento
 * en sí es permanente (no hay expiración automática).
 *
 * `cacheDir` permite inyectar un directorio alternativo (útil en tests).
 */
export async function clearPosterCache(
  cacheDir: string = DEFAULT_CACHE_DIR,
  maxAgeDays = 30,
): Promise<{ deleted: number; errors: string[] }> {
  let deleted = 0;
  const errors: string[] = [];

  const deleteAll = maxAgeDays <= 0;
  const cutoff = deleteAll ? 0 : Date.now() - maxAgeDays * 24 * 60 * 60 * 1000;

  try {
    const entries = await readdir(cacheDir);
    for (const entry of entries) {
      const localPath = join(cacheDir, entry);
      try {
        const fileStat = await stat(localPath);
        if (deleteAll || fileStat.mtimeMs < cutoff) {
          await rm(localPath);
          deleted++;
        }
      } catch (error) {
        errors.push(formatError(error));
      }
    }
  } catch {
    // El directorio no existe aún; no hay nada que borrar
  }

  return { deleted, errors };
}
