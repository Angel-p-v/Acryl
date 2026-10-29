import { AmbiguousError, NotFoundError, formatError } from '../../shared/errors.js';

import type { Logger } from '../../shared/logger.js';
import type { MediaType, MetadataConfig } from '../config/schema.js';
import type { FileRepository } from '../db/repositories/fileRepo.js';
import type { IndexRunRepository } from '../db/repositories/indexRunRepo.js';
import type { ListMediaOptions, MediaRepository } from '../db/repositories/mediaRepo.js';
import type { MetadataRepository, MediaMetadataRow } from '../db/repositories/metadataRepo.js';
import type { PlaybackRepository } from '../db/repositories/playbackRepo.js';
import type { PreferenceRepository } from '../db/repositories/preferencesRepo.js';
import type { TagRepository, TagRow } from '../db/repositories/tagRepo.js';
import type { IndexRunRow, MediaFileRow, MediaRow } from '../db/types.js';
import type { MetadataService } from '../metadata/service.js';

/** Repositorios que necesita la capa de consulta, inyectados por el llamante. */
export type LibraryRepositories = {
  mediaRepo: MediaRepository;
  fileRepo: FileRepository;
  indexRunRepo: IndexRunRepository;
  playbackRepo: PlaybackRepository;
  preferenceRepo: PreferenceRepository;
  tagRepo: TagRepository;
  metadataRepo: MetadataRepository;
  metadataService: MetadataService;
  metadataConfig: MetadataConfig;
  /** Logger para logs de depuración (se activa con --verbose). Opcional. */
  logger?: Logger;
};

export type SearchMediaOptions = {
  type?: MediaType;
  limit?: number;
};

export type MediaDetails = {
  media: MediaRow;
  files: MediaFileRow[];
};

export type LibraryStats = {
  media: Record<'total' | MediaType, number>;
  files: number;
  lastRun: IndexRunRow | null;
  inProgress: number;
};

/**
 * Casos de uso de consulta de la biblioteca.
 *
 * La capa de presentación (CLI o TUI) solo habla con esta interfaz: no conoce
 * SQL ni la forma de las tablas.
 */
export type LibraryService = {
  list(options?: ListMediaOptions): MediaRow[];
  count(type?: MediaType): number;
  search(query: string, options?: SearchMediaOptions): MediaRow[];
  /** Resuelve un id o un título y devuelve el título con sus ficheros. */
  find(query: string): MediaDetails;
  /** Títulos en curso (con progreso guardado y no completados). */
  listInProgress(limit?: number): MediaRow[];
  stats(): LibraryStats;
  /** Número de títulos marcados como favoritos. */
  countFavorites(): number;
  /** Lista todos los tags disponibles. */
  listTags(): TagRow[];
  /** Obtiene un tag por nombre. */
  getTagByName(name: string): TagRow | undefined;
  /** Asigna un tag a un título. */
  assignTag(mediaId: number, tagName: string): void;
  /** Quita un tag de un título. */
  unassignTag(mediaId: number, tagName: string): void;
  /** Lista los tags de un título. */
  listTagsForMedia(mediaId: number): TagRow[];
  /** Obtiene metadatos externos de un título. */
  getMetadata(mediaId: number): MediaMetadataRow | undefined;
  /** Establece o actualiza metadatos externos de un título. */
  setMetadata(input: {
    mediaId: number;
    provider: 'tvmaze' | 'jikan' | 'default';
    externalId: number;
    title?: string | null;
    overview?: string | null;
    posterPath?: string | null;
    backdropPath?: string | null;
    voteAverage?: number | null;
    firstAirDate?: string | null;
    releaseDate?: string | null;
    originalLanguage?: string | null;
  }): MediaMetadataRow;
  /** Elimina metadatos externos de un título. */
  removeMetadata(mediaId: number): boolean;
  /** Elimina todos los metadatos externos (forza refresco al configurar nuevas API keys). */
  clearAllMetadata(): number;
  /**
   * Obtiene metadatos de un título, refrescándolos automáticamente si están
   * expirados según el TTL configurado. Si `forceRefresh` es true, se ignora
   * el TTL y siempre se intenta obtener metadatos de la API externa (útil
   * cuando el usuario pulsa manualmente la tecla "r" en la TUI).
   * Si no hay API key configurada, devuelve los metadatos almacenados
   * (o undefined si no existen).
   */
  getOrFetchMetadata(
    mediaId: number,
    options?: { forceRefresh?: boolean },
  ): Promise<MediaMetadataRow | undefined>;
  /**
   * Refresca todos los metadatos cuyo caché ha expirado según el TTL configurado.
   * Devuelve cuántos se refrescaron y cuántos fallaron.
   */
  refreshExpiredMetadata(): Promise<{ refreshed: number; failed: number }>;
  /**
   * Obtiene metadatos para todos los títulos que no los tienen aún.
   * Respeta el rate limiting de cada proveedor.
   */
  fetchMissingMetadata(): Promise<{ fetched: number; failed: number }>;
  /** Devuelve la cantidad de metadatos expirados según el TTL configurado. */
  getExpiredMetadataCount(): number;
  /** Devuelve true si hay algún proveedor de metadatos configurado. */
  hasMetadataProviderConfigured(): boolean;
  /** Servicio de metadatos externos (permite actualizar claves API en caliente). */
  readonly metadataService: MetadataService;
};

/**
 * Comportamiento de los filtros watched/unwatched en mediaRepo.list():
 *
 * - watched: título tiene AL MENOS UN archivo con completedAt !== null
 * - unwatched: título NO TIENE archivos con completedAt !== null
 *   (incluye títulos nunca reproducidos y parcialmente vistos sin completar)
 */

/** Crea el servicio de biblioteca sobre un conjunto de repositorios. */
export function createLibraryService(repositories: LibraryRepositories): LibraryService {
  const {
    mediaRepo,
    fileRepo,
    indexRunRepo,
    playbackRepo,
    preferenceRepo,
    tagRepo,
    metadataRepo,
    metadataService,
    metadataConfig,
    logger,
  } = repositories;

  const detailsFor = (media: MediaRow): MediaDetails => ({
    media,
    files: fileRepo.listByMedia(media.id),
  });

  return {
    list(options: ListMediaOptions = {}): MediaRow[] {
      return mediaRepo.list(options);
    },

    count(type?: MediaType): number {
      return mediaRepo.count(type);
    },

    search(query: string, options: SearchMediaOptions = {}): MediaRow[] {
      return mediaRepo.search(query, options);
    },

    find(query: string): MediaDetails {
      const trimmed = query.trim();
      if (trimmed.length === 0) {
        throw new NotFoundError('Indica un id o un título para consultar la biblioteca.');
      }

      // Un número puede ser un id o el título de una película (`1917`). Se intenta
      // primero como id y, si no existe, se busca por título.
      if (/^\d+$/.test(trimmed)) {
        const byId = mediaRepo.findById(Number(trimmed));
        if (byId) return detailsFor(byId);
      }

      const matches = mediaRepo.search(trimmed, { limit: 20 });
      const normalized = normalizeForMatch(trimmed);
      const exact = matches.filter((media) => normalizeForMatch(media.title) === normalized);
      const candidates = exact.length > 0 ? exact : matches;

      if (candidates.length === 0) {
        throw new NotFoundError(`No hay ningún título que coincida con «${trimmed}».`);
      }

      if (candidates.length > 1) {
        const options = candidates
          .slice(0, 10)
          .map((media) => `  · #${media.id} ${formatTitle(media)}`)
          .join('\n');
        throw new AmbiguousError(
          `Hay varios títulos que coinciden con «${trimmed}»:\n${options}\nUsa el id para elegir uno.`,
        );
      }

      const candidate = candidates[0];
      if (!candidate) throw new NotFoundError('No se encontró el título');
      return detailsFor(candidate);
    },

    listInProgress(limit = 200): MediaRow[] {
      const entries = playbackRepo.listInProgress(limit);
      // Devolvemos solo los MediaRow únicos (un título puede tener varios ficheros en progreso)
      const seen = new Set<number>();
      return entries
        .map((entry) => entry.media)
        .filter((media) => {
          if (seen.has(media.id)) return false;
          seen.add(media.id);
          return true;
        });
    },

    stats(): LibraryStats {
      return {
        media: {
          total: mediaRepo.count(),
          series: mediaRepo.count('series'),
          anime: mediaRepo.count('anime'),
          movie: mediaRepo.count('movie'),
          unknown: mediaRepo.count('unknown'),
        },
        files: fileRepo.count(),
        lastRun: indexRunRepo.latest() ?? null,
        inProgress: playbackRepo.count(),
      };
    },

    countFavorites(): number {
      return preferenceRepo.countFavorites();
    },

    listTags(): TagRow[] {
      return tagRepo.list();
    },

    getTagByName(name: string): TagRow | undefined {
      return tagRepo.getByName(name);
    },

    assignTag(mediaId: number, tagName: string): void {
      const tag = tagRepo.getByName(tagName);
      if (!tag) {
        // Crear el tag si no existe
        const created = tagRepo.create({ name: tagName });
        tagRepo.assign(mediaId, created.id);
      } else {
        tagRepo.assign(mediaId, tag.id);
      }
    },

    unassignTag(mediaId: number, tagName: string): void {
      const tag = tagRepo.getByName(tagName);
      if (tag) {
        tagRepo.unassign(mediaId, tag.id);
      }
    },

    listTagsForMedia(mediaId: number): TagRow[] {
      return tagRepo.listByMedia(mediaId);
    },

    getMetadata(mediaId: number): MediaMetadataRow | undefined {
      return metadataRepo.get(mediaId);
    },

    setMetadata(input: {
      mediaId: number;
      provider: 'tvmaze' | 'jikan' | 'default';
      externalId: number;
      title?: string | null;
      overview?: string | null;
      posterPath?: string | null;
      backdropPath?: string | null;
      voteAverage?: number | null;
      firstAirDate?: string | null;
      releaseDate?: string | null;
      originalLanguage?: string | null;
      localPosterPath?: string | null;
    }): MediaMetadataRow {
      return metadataRepo.set({
        mediaId: input.mediaId,
        provider: input.provider,
        externalId: input.externalId,
        title: input.title ?? null,
        overview: input.overview ?? null,
        posterPath: input.posterPath ?? null,
        backdropPath: input.backdropPath ?? null,
        voteAverage: input.voteAverage ?? null,
        firstAirDate: input.firstAirDate ?? null,
        releaseDate: input.releaseDate ?? null,
        originalLanguage: input.originalLanguage ?? null,
        localPosterPath: input.localPosterPath ?? null,
        updatedAt: new Date().toISOString(),
      });
    },

    removeMetadata(mediaId: number): boolean {
      return metadataRepo.remove(mediaId);
    },

    clearAllMetadata(): number {
      return metadataRepo.removeAll();
    },

    /**
     * Obtiene metadatos de un título, refrescándolos automáticamente si están
     * expirados según el TTL configurado. Si `forceRefresh` es true, se ignora
     * el TTL y siempre se intenta obtener metadatos de la API externa (útil
     * cuando el usuario pulsa manualmente la tecla "r" en la TUI).
     * Si no hay API key configurada, devuelve los metadatos almacenados
     * (o undefined si no existen).
     */
    async getOrFetchMetadata(
      mediaId: number,
      options: { forceRefresh?: boolean } = {},
    ): Promise<MediaMetadataRow | undefined> {
      const { forceRefresh = false } = options;
      await Promise.resolve();
      const existing = metadataRepo.get(mediaId);
      const ttlHours = metadataConfig.cacheTtlHours ?? 24;
      const now = new Date();

      // Si existen metadatos y no han expirado, devolverlos directamente,
      // a menos que se force el refresco o que no tengan póster y haya un
      // proveedor externo disponible.
      if (existing && !forceRefresh) {
        const fetchedAt = new Date(existing.fetchedAt);
        const ageHours = (now.getTime() - fetchedAt.getTime()) / (1000 * 60 * 60);
        const hasExternalProvider = metadataService.hasProviderConfigured();
        const needsPoster = existing.localPosterPath === null;

        if (ageHours < ttlHours && !(needsPoster && hasExternalProvider)) {
          return existing;
        }
      }

      // Si hay algún proveedor disponible, intentar obtener metadatos
      if (metadataService.hasProviderConfigured()) {
        // Si ya tenemos metadatos previos (expirados o sin póster) con externalId, refrescar por ID
        if (existing?.externalId) {
          for (const provider of ['tvmaze', 'jikan'] as const) {
            if (!metadataService.isProviderConfigured(provider)) continue;

            try {
              const mediaType = 'tv';
              const details = await metadataService.getDetails(
                existing.externalId,
                provider,
                mediaType,
              );
              logger?.debug('metadata.fetch.by-id.ok', {
                mediaId,
                provider,
                externalId: details.externalId,
                title: details.title,
              });
              return metadataRepo.set({
                mediaId,
                provider,
                externalId: details.externalId,
                title: details.title,
                overview: details.overview,
                posterPath: details.posterPath,
                backdropPath: details.backdropPath,
                voteAverage: details.voteAverage,
                releaseDate: details.releaseDate,
                originalLanguage: details.originalLanguage,
                localPosterPath: details.localPosterPath ?? null,
                updatedAt: now.toISOString(),
              });
            } catch (error) {
              logger?.debug('metadata.fetch.by-id.fail', {
                mediaId,
                provider,
                error: formatError(error),
              });
              // Si falla un proveedor, probar el siguiente
              continue;
            }
          }
        }

        // Si no hay metadatos previos (o no tienen externalId), buscar por título
        const media = mediaRepo.findById(mediaId);
        if (media) {
          const mediaType = media.type === 'movie' ? 'movie' : 'tv';

          // Limpiar el título antes de buscar en la API externa para evitar
          // que etiquetas como [Judas], S01E01, 1080p, etc. impidan encontrar el título.
          const searchQuery = cleanSearchTitle(media.title) || media.title;

          logger?.debug('metadata.search.title', {
            mediaId,
            originalTitle: media.title,
            cleanedTitle: searchQuery,
            mediaType,
          });

          // Lista de consultas a probar: primero la limpia, luego la original como fallback
          const queriesToTry = [searchQuery];
          if (searchQuery !== media.title) {
            queriesToTry.push(media.title);
          }

          for (const query of queriesToTry) {
            logger?.debug('metadata.search.attempt', {
              mediaId,
              query,
              mediaType,
            });

            try {
              const searchResults = await metadataService.search(query);
              logger?.debug('metadata.search.results', {
                mediaId,
                query,
                totalResults: searchResults.length,
                providers: searchResults.map((r) => r.provider),
              });

              const result = searchResults.find(
                (
                  r,
                ): r is {
                  provider: 'tvmaze' | 'jikan' | 'default';
                  externalId: number;
                  title: string;
                  overview: string | null;
                  posterPath: string | null;
                  backdropPath: string | null;
                  voteAverage: number | null;
                  releaseDate: string | null;
                  originalLanguage: string | null;
                  localPosterPath: string | null;
                } => r.provider === 'tvmaze' || r.provider === 'jikan',
              );

              if (result) {
                logger?.debug('metadata.search.selected-result', {
                  mediaId,
                  query,
                  provider: result.provider,
                  externalId: result.externalId,
                  title: result.title,
                });

                const details = await metadataService.getDetails(
                  result.externalId,
                  result.provider,
                  mediaType,
                );
                // Si el proveedor no devolvió título (proveedor local sin API key),
                // usamos el título del resultado de búsqueda (título del usuario).
                const title = details.title || result.title;
                logger?.debug('metadata.fetch.by-title.ok', {
                  mediaId,
                  provider: result.provider,
                  externalId: details.externalId,
                  title,
                  hasPoster: !!details.localPosterPath,
                });
                return metadataRepo.set({
                  mediaId,
                  provider: result.provider,
                  externalId: details.externalId,
                  title,
                  overview: details.overview,
                  posterPath: details.posterPath,
                  backdropPath: details.backdropPath,
                  voteAverage: details.voteAverage,
                  releaseDate: details.releaseDate,
                  originalLanguage: details.originalLanguage,
                  localPosterPath: details.localPosterPath ?? null,
                  updatedAt: now.toISOString(),
                });
              } else {
                logger?.debug('metadata.search.no-real-results', {
                  mediaId,
                  query,
                  hasDefaultResult: searchResults.some((r) => r.provider === 'default'),
                });
              }
            } catch (error) {
              logger?.debug('metadata.search.attempt.fail', {
                mediaId,
                query,
                error: formatError(error),
              });
            }
          }
        }
      }

      logger?.debug('metadata.search.no-metadata-found', {
        mediaId,
      });

      return existing;
    },

    /**
     * Refresca todos los metadatos cuyo caché ha expirado según el TTL configurado.
     * Devuelve cuántos se refrescaron y cuántos fallaron.
     */
    async refreshExpiredMetadata(): Promise<{ refreshed: number; failed: number }> {
      const ttlHours = metadataConfig.cacheTtlHours ?? 24;
      const expired = metadataRepo.listExpired(ttlHours);

      let refreshed = 0;
      let failed = 0;

      for (const existing of expired) {
        if (!metadataService.hasProviderConfigured()) {
          failed++;
          continue;
        }

        let updated = false;
        for (const provider of ['tvmaze', 'jikan'] as const) {
          if (!metadataService.isProviderConfigured(provider)) continue;

          try {
            const mediaType = 'tv';
            const details = await metadataService.getDetails(
              existing.externalId,
              provider,
              mediaType,
            );
            metadataRepo.set({
              mediaId: existing.mediaId,
              provider,
              externalId: details.externalId,
              title: details.title,
              overview: details.overview,
              posterPath: details.posterPath,
              backdropPath: details.backdropPath,
              voteAverage: details.voteAverage,
              releaseDate: details.releaseDate,
              originalLanguage: details.originalLanguage,
              localPosterPath: details.localPosterPath ?? null,
              updatedAt: new Date().toISOString(),
            });
            refreshed++;
            updated = true;
            break;
          } catch {
            // Si falla un proveedor, probar el siguiente
            continue;
          }
        }

        if (!updated) {
          failed++;
        }
      }

      return { refreshed, failed };
    },

    /**
     * Obtiene metadatos para todos los títulos que no los tienen aún.
     * Útil para comportamiento tipo Sonarr/Radarr: al indexar, se buscan
     * automáticamente los metadatos externos de los títulos nuevos.
     * Respeta el rate limiting de cada proveedor.
     */
    async fetchMissingMetadata(): Promise<{ fetched: number; failed: number }> {
      if (!metadataService.hasProviderConfigured()) {
        return { fetched: 0, failed: 0 };
      }

      let fetched = 0;
      let failed = 0;
      let offset = 0;
      const limit = 50;

      // Procesar en lotes para no cargar todos los títulos en memoria de una vez
      while (true) {
        const media = mediaRepo.list({ limit, offset });
        if (media.length === 0) break;

        for (const row of media) {
          const alreadyHas = metadataRepo.get(row.id);
          if (alreadyHas) continue;

          try {
            const result = await this.getOrFetchMetadata(row.id);
            if (result) fetched++;
          } catch {
            failed++;
          }
        }

        offset += limit;
        if (media.length < limit) break;
      }

      return { fetched, failed };
    },

    /** Devuelve la cantidad de metadatos expirados según el TTL configurado. */
    getExpiredMetadataCount(): number {
      const ttlHours = metadataConfig.cacheTtlHours ?? 24;
      return metadataRepo.listExpired(ttlHours).length;
    },

    /** Devuelve true si hay algún proveedor de metadatos configurado. */
    hasMetadataProviderConfigured(): boolean {
      return metadataService.hasProviderConfigured();
    },

    /** Servicio de metadatos externos (permite actualizar claves API en caliente). */
    metadataService,
  };
}

/** Compara títulos sin depender de mayúsculas, acentos ni espacios repetidos. */
function normalizeForMatch(value: string): string {
  return value
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .toLocaleLowerCase()
    .replace(/\s+/g, ' ')
    .trim();
}

/**
 * Limpia un título de biblioteca para usarlo como consulta de búsqueda en APIs
 * externas (TVMaze, Jikan). Elimina:
 * - Etiquetas entre corchetes: [Judas], [DualAudio], etc.
 * - Especificadores de temporada/episodio y todo lo que sigue: S01E01, S01 E01, 1x01,
 *   Season 01, T1E2, E01, etc. (corta todo desde el primer patrón en adelante)
 * - Calidad y formato: 1080p, 720p, 480p, 2160p, 4K, BluRay, WEBDL, WEB-DL, x264, x265,
 *   HEVC, H.264, H.265, 10 bits, REMUX, RIP, HDTV, PDTV, etc.
 * - Audio: Dual Audio, DD+, AAC, FLAC, DTS, etc.
 * - Extensiones de archivo: .mkv, .mp4, .avi, etc.
 * - Año entre paréntesis al final: (2023), (2023-2024)
 * - Grupos de release al final: -EMBER, -Judas, -SUBSPLEASE, etc.
 * - Espacios repetidos sobrantes.
 */
function cleanSearchTitle(title: string): string {
  let cleaned = title;

  // 1. Cortar la cadena desde el primer patrón de temporada/episodio en adelante.
  //    Esto descarta todo lo que venga después de "S01", "Season 1", "T01",
  //    "S01E02", "1x02", "E02", etc.
  cleaned = cleaned.replace(
    /\b(?:S\d{1,2}|Season\s*\d{1,2}|T\d{1,2}|E\d{1,3}|S\d{1,2}E\d{1,3}|\d{1,2}x\d{1,3}).*/gi,
    ' ',
  );

  // 2. Eliminar etiquetas entre corchetes y su contenido: [Judas], [Dual Audio], etc.
  cleaned = cleaned.replace(/\[[^\]]*\]/g, ' ');

  // 3. Eliminar grupos de release al final: -EMBER, -Judas, -SUBSPLEASE, etc.
  cleaned = cleaned.replace(/\s*-\s*[A-Za-z0-9]+(?:\s*$|\s)/g, ' ');

  // 4. Eliminar resoluciones y calidad
  cleaned = cleaned.replace(/\b(1080p|720p|480p|2160p|4k|4K|HD|FHD|UHD)\b/gi, ' ');

  // 5. Eliminar fuentes y formatos
  cleaned = cleaned.replace(
    /\b(BluRay|Blu-Ray|BRRIP|BDRip|WEBDL|WEB-DL|WebDL|WEBRIP|WEB-RIP|WebRip|REMUX|RIP|HDTV|PDTV|DVDRip|DVD-Rip)\b/gi,
    ' ',
  );

  // 6. Eliminar codecs
  cleaned = cleaned.replace(/\b(x264|x265|HEVC|H\.264|H\.265)\b/gi, ' ');

  // 7. Eliminar profundidad de bits y audio
  cleaned = cleaned.replace(
    /\b(10\s*bits|10bit|DD\+|DDP|DTS|FLAC|AAC|Dual\s*Audio|LATINO|CASTELLANO|SUBESPAÑOL|SUB)\b/gi,
    ' ',
  );

  // 8. Eliminar extensiones de archivo
  cleaned = cleaned.replace(/\.(mkv|mp4|avi|mov|wmv|flv|webm|m4v|ts|m2ts)$/gi, ' ');

  // 9. Eliminar año entre paréntesis al final del título: (2023), (2023-2024)
  cleaned = cleaned.replace(/\s*\(\d{4}(?:-\d{4})?\)\s*$/, ' ');

  // Colapsar espacios múltiples y recortar
  return cleaned.replace(/\s+/g, ' ').trim();
}

/** Formatea un título con su año para listados de desambiguación. */
function formatTitle(media: MediaRow): string {
  return media.year === null ? media.title : `${media.title} (${media.year})`;
}
