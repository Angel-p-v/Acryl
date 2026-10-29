import { cachePoster, getCachedPoster } from './posterCache.js';
import { rateLimiters } from './rateLimiter.js';
import { MetadataError, formatError } from '../../shared/errors.js';

import type { Logger } from '../../shared/logger.js';

/** Serie de TVMaze. */
type TvMazeShow = {
  id: number;
  name: string;
  type: string;
  language: string;
  genres: string[];
  status: string;
  runtime: number | null;
  premiered: string | null;
  officialSite: string | null;
  schedule: { time: string | null; days: string[] };
  rating: { average: number | null };
  weight: number;
  network: { name: string | null; country: { name: string | null } | null } | null;
  webChannel: { name: string | null; country: { name: string | null } | null } | null;
  externals: { tvrage: number | null; thetvdb: number | null; imdb: string | null };
  image: { medium: string | null; original: string | null } | null;
  summary: string | null;
  updated: number;
  _links: {
    self: { href: string };
    previousepisode?: { href: string };
    nextepisode?: { href: string };
    lastepisode?: { href: string };
  };
};

/** Respuesta de búsqueda de Jikan (anime). */
type JikanSearchResponse = {
  data: JikanSearchItem[];
  pagination: { last_visible_page: number };
};

/** Resultado de búsqueda de Jikan. */
type JikanSearchItem = {
  mal_id: number;
  title: string;
  title_english: string | null;
  title_japanese: string | null;
  title_synonyms: string[];
  type: string;
  source: string | null;
  episodes: number | null;
  status: string;
  airing: boolean;
  aired: { from: string | null; to: string | null; string: string };
  duration: string | null;
  rating: string | null;
  score: number | null;
  ranked: number | null;
  popularity: number | null;
  members: number | null;
  favorites: number | null;
  synopsis: string | null;
  background: string | null;
  season: string | null;
  year: number | null;
  broadcast: { day: string | null; time: string | null; timezone: string | null } | null;
  producers: { mal_id: number; name: string }[];
  licensors: { mal_id: number; name: string }[];
  studios: { mal_id: number; name: string }[];
  genres: { mal_id: number; name: string }[];
  explicit_genres: { mal_id: number; name: string }[];
  themes: { mal_id: number; name: string }[];
  demographic: { mal_id: number; name: string } | null;
  images: {
    jpg: {
      image_url: string | null;
      small_image_url: string | null;
      large_image_url: string | null;
    };
    webp: {
      image_url: string | null;
      small_image_url: string | null;
      large_image_url: string | null;
    };
  };
  trailer: { youtube_id: string | null; url: string | null; embed_url: string | null } | null;
};

/** Detalles de un anime de Jikan. */
type JikanAnimeDetails = {
  mal_id: number;
  title: string;
  title_english: string | null;
  title_japanese: string | null;
  title_synonyms: string[];
  type: string;
  source: string | null;
  episodes: number | null;
  status: string;
  airing: boolean;
  aired: { from: string | null; to: string | null; string: string };
  duration: string | null;
  rating: string | null;
  score: number | null;
  ranked: number | null;
  popularity: number | null;
  members: number | null;
  favorites: number | null;
  synopsis: string | null;
  background: string | null;
  season: string | null;
  year: number | null;
  broadcast: { day: string | null; time: string | null; timezone: string | null } | null;
  producers: { mal_id: number; name: string }[];
  licensors: { mal_id: number; name: string }[];
  studios: { mal_id: number; name: string }[];
  genres: { mal_id: number; name: string }[];
  explicit_genres: { mal_id: number; name: string }[];
  themes: { mal_id: number; name: string }[];
  demographic: { mal_id: number; name: string } | null;
  images: {
    jpg: {
      image_url: string | null;
      small_image_url: string | null;
      large_image_url: string | null;
    };
    webp: {
      image_url: string | null;
      small_image_url: string | null;
      large_image_url: string | null;
    };
  };
  trailer: { youtube_id: string | null; url: string | null; embed_url: string | null } | null;
};

/** Resultado de búsqueda de metadatos. */
export type MetadataSearchResult = {
  provider: 'tvmaze' | 'jikan' | 'default';
  externalId: number;
  title: string;
  overview: string | null;
  posterPath: string | null;
  backdropPath: string | null;
  voteAverage: number | null;
  releaseDate: string | null;
  originalLanguage: string | null;
  /** Ruta local al póster descargado en caché, si existe. */
  localPosterPath: string | null;
};

/** Configuración del servicio de metadatos (sin claves API - usa proveedores públicos). */
export type MetadataServiceConfig = {
  logger: {
    debug(msg: string, bindings?: Record<string, unknown>): void;
    warn(msg: string, bindings?: Record<string, unknown>): void;
    error(msg: string, bindings?: Record<string, unknown>): void;
  };
};

/**
 * Adapta el logger de la configuración (que tiene un tipo limitado) al tipo
 * Logger completo que requiere cachePoster.
 */
type ConfigLogger = {
  debug(msg: string, bindings?: Record<string, unknown>): void;
  warn(msg: string, bindings?: Record<string, unknown>): void;
  error(msg: string, bindings?: Record<string, unknown>): void;
};

function adaptLogger(configLogger: ConfigLogger): Logger {
  return {
    level: 'debug',
    format: 'text',
    error: (event: string, fields?: Record<string, unknown>) => configLogger.error(event, fields),
    warn: (event: string, fields?: Record<string, unknown>) => configLogger.warn(event, fields),
    info: (event: string, fields?: Record<string, unknown>) => configLogger.debug(event, fields),
    debug: (event: string, fields?: Record<string, unknown>) => configLogger.debug(event, fields),
    child: () => adaptLogger(configLogger),
  };
}

/**
 * Servicio para obtener metadatos externos usando proveedores públicos
 * gratuitos sin necesidad de claves API.
 *
 * - TVMaze (https://www.tvmaze.com/api): Series y películas, sin API key.
 * - Jikan (https://jikan.me/): Anime, sin API key.
 *
 * Los proveedores se intentan en orden: TVMaze primero, luego Jikan como
 * fallback para anime. Si ninguno tiene resultados, se devuelve un resultado
 * local basado en la consulta.
 */
export class MetadataService {
  private readonly logger: Logger;

  constructor(config: MetadataServiceConfig) {
    this.logger = adaptLogger(config.logger);
  }

  /**
   * Actualiza la configuración en tiempo de ejecución (mantenido por compatibilidad).
   * Los proveedores públicos no requieren configuración, así que esto es un no-op.
   */
  updateConfig(_newConfig: Partial<MetadataServiceConfig>): void {
    // No hay claves API que actualizar: los proveedores son públicos y gratuitos.
  }

  /**
   * Busca títulos en TVMaze por consulta de texto.
   * Devuelve resultados mapeados a un formato común.
   * TVMaze es gratuito y no requiere API key.
   */
  async searchTvMaze(query: string): Promise<MetadataSearchResult[]> {
    await rateLimiters.tvmaze.waitForSlot();

    const url = new URL('https://api.tvmaze.com/singlesearch/shows');
    url.searchParams.set('q', query);
    url.searchParams.set('embed', 'episodes');

    this.logger.debug('[METADATA API] TVMaze búsqueda', {
      provider: 'tvmaze',
      query,
      url: url.toString(),
    });

    const response = await fetch(url.toString(), {
      headers: { Accept: 'application/json' },
    });

    // TVMaze devuelve 404 cuando no encuentra nada: devolver array vacío
    if (response.status === 404) {
      this.logger.debug('[METADATA API] TVMaze sin resultados', {
        provider: 'tvmaze',
        query,
        status: response.status,
      });
      return [];
    }

    if (!response.ok) {
      const body = await response.text().catch(() => '');
      this.logger.debug('[METADATA API] TVMaze error HTTP', {
        provider: 'tvmaze',
        query,
        status: response.status,
        statusText: response.statusText,
        body: body.slice(0, 500),
      });
      throw new MetadataError(`TVMaze API respondió ${response.status}`);
    }

    const data = (await response.json()) as TvMazeShow;

    this.logger.debug('[METADATA API] TVMaze resultado', {
      provider: 'tvmaze',
      query,
      title: data.name,
      id: data.id,
      hasPoster: !!data.image,
      hasSummary: !!data.summary,
    });

    const posterPath = data.image?.original ?? data.image?.medium ?? null;
    let localPosterPath: string | null = null;
    if (posterPath) {
      localPosterPath = await cachePoster(posterPath, data.name, 'tv', this.logger);
      localPosterPath ??= await getCachedPoster(data.name, 'tv');
    }

    return [
      {
        provider: 'tvmaze' as const,
        externalId: data.id,
        title: data.name,
        overview: data.summary ? stripHtml(data.summary) : null,
        posterPath,
        backdropPath: null,
        voteAverage: data.rating.average ?? null,
        releaseDate: data.premiered,
        originalLanguage: data.language,
        localPosterPath,
      },
    ];
  }

  /**
   * Obtiene detalles de un título de TVMaze por ID externo.
   */
  async getTvMazeDetails(
    externalId: number,
    _mediaType: 'movie' | 'tv' = 'tv',
  ): Promise<MetadataSearchResult> {
    await rateLimiters.tvmaze.waitForSlot();

    const url = new URL(`https://api.tvmaze.com/shows/${externalId}`);

    const response = await fetch(url.toString(), {
      headers: { Accept: 'application/json' },
    });

    if (!response.ok) {
      throw new MetadataError(`TVMaze API respondió ${response.status}`);
    }

    const data = (await response.json()) as TvMazeShow;

    const posterPath = data.image?.original ?? data.image?.medium ?? null;
    if (posterPath) {
      this.logger.info('[METADATA API] Poster URL encontrada:', {
        provider: 'tvmaze',
        title: data.name,
        posterUrl: posterPath,
      });
    } else {
      this.logger.warn('[METADATA API] No se encontró URL de póster para este título:', {
        provider: 'tvmaze',
        title: data.name,
      });
    }

    let localPosterPath: string | null = null;
    if (posterPath) {
      localPosterPath = await cachePoster(posterPath, data.name, 'tv', this.logger);
      localPosterPath ??= await getCachedPoster(data.name, 'tv');
    }

    return {
      provider: 'tvmaze',
      externalId: data.id,
      title: data.name,
      overview: data.summary ? stripHtml(data.summary) : null,
      posterPath,
      backdropPath: null,
      voteAverage: data.rating.average ?? null,
      releaseDate: data.premiered,
      originalLanguage: data.language,
      localPosterPath,
    };
  }

  /**
   * Busca anime en Jikan por consulta de texto.
   * Devuelve resultados mapeados a un formato común.
   * Jikan es gratuito y no requiere API key.
   */
  async searchJikan(query: string): Promise<MetadataSearchResult[]> {
    await rateLimiters.jikan.waitForSlot();

    const url = new URL('https://api.jikan.moe/v4/anime');
    url.searchParams.set('q', query);
    url.searchParams.set('limit', '10');

    this.logger.debug('[METADATA API] Jikan búsqueda', {
      provider: 'jikan',
      query,
      url: url.toString(),
    });

    const response = await fetch(url.toString(), {
      headers: { Accept: 'application/json' },
    });

    if (!response.ok) {
      const body = await response.text().catch(() => '');
      this.logger.debug('[METADATA API] Jikan error HTTP', {
        provider: 'jikan',
        query,
        status: response.status,
        statusText: response.statusText,
        body: body.slice(0, 500),
      });
      throw new MetadataError(`Jikan API respondió ${response.status}`);
    }

    const data = (await response.json()) as JikanSearchResponse;

    this.logger.debug('[METADATA API] Jikan resultado', {
      provider: 'jikan',
      query,
      resultsCount: data.data.length,
      titles: data.data.map((item) => item.title),
    });

    const results: MetadataSearchResult[] = [];
    for (const item of data.data) {
      const posterPath = item.images?.jpg?.large_image_url ?? null;
      const itemTitle = item.title_english ?? item.title;

      if (posterPath) {
        this.logger.info('[METADATA API] Poster URL encontrada:', {
          provider: 'jikan',
          title: itemTitle,
          posterUrl: posterPath,
        });
      } else {
        this.logger.warn('[METADATA API] No se encontró URL de póster para este título:', {
          provider: 'jikan',
          title: itemTitle,
        });
      }

      let localPosterPath: string | null = null;
      if (posterPath) {
        localPosterPath = await cachePoster(posterPath, itemTitle, 'anime', this.logger);
        localPosterPath ??= await getCachedPoster(itemTitle, 'anime');
      }

      const releaseDate = item.aired?.from ?? null;

      results.push({
        provider: 'jikan' as const,
        externalId: item.mal_id,
        title: item.title_english ?? item.title,
        overview: item.synopsis ?? null,
        posterPath,
        backdropPath: null,
        voteAverage: item.score ?? null,
        releaseDate,
        originalLanguage: 'ja',
        localPosterPath,
      });
    }
    return results;
  }

  /**
   * Obtiene detalles de un anime de Jikan por ID externo.
   */
  async getJikanDetails(externalId: number): Promise<MetadataSearchResult> {
    await rateLimiters.jikan.waitForSlot();

    const url = new URL(`https://api.jikan.moe/v4/anime/${externalId}`);

    const response = await fetch(url.toString(), {
      headers: { Accept: 'application/json' },
    });

    if (!response.ok) {
      throw new MetadataError(`Jikan API respondió ${response.status}`);
    }

    const data = (await response.json()) as JikanAnimeDetails;

    const posterPath = data.images?.jpg?.large_image_url ?? null;
    const animeTitle = data.title_english ?? data.title;

    if (posterPath) {
      this.logger.info('[METADATA API] Poster URL encontrada:', {
        provider: 'jikan',
        title: animeTitle,
        posterUrl: posterPath,
      });
    } else {
      this.logger.warn('[METADATA API] No se encontró URL de póster para este título:', {
        provider: 'jikan',
        title: animeTitle,
      });
    }

    let localPosterPath: string | null = null;
    if (posterPath) {
      localPosterPath = await cachePoster(posterPath, animeTitle, 'anime', this.logger);
      localPosterPath ??= await getCachedPoster(animeTitle, 'anime');
    }

    const releaseDate = data.aired?.from ?? null;

    return {
      provider: 'jikan',
      externalId: data.mal_id,
      title: data.title_english ?? data.title,
      overview: data.synopsis ?? null,
      posterPath,
      backdropPath: null,
      voteAverage: data.score ?? null,
      releaseDate,
      originalLanguage: 'ja',
      localPosterPath,
    };
  }

  /**
   * Busca en todos los proveedores públicos disponibles (TVMaze primero,
   * luego Jikan como fallback para anime).
   * Siempre devuelve al menos un resultado local basado en la consulta.
   */
  async search(query: string): Promise<MetadataSearchResult[]> {
    const results: MetadataSearchResult[] = [];

    // TVMaze primero (cubre series y películas).
    try {
      const tvmazeResults = await this.searchTvMaze(query);
      results.push(...tvmazeResults);
    } catch (error) {
      this.logger.warn('TVMaze búsqueda falló', {
        error: formatError(error),
      });
    }

    // Jikan como fallback para anime.
    try {
      const jikanResults = await this.searchJikan(query);
      results.push(...jikanResults);
    } catch (error) {
      this.logger.warn('Jikan búsqueda falló', {
        error: formatError(error),
      });
    }

    // Si no hay resultados de ningún proveedor, devolver un resultado local
    // basado en la consulta para no dejar al usuario sin nada.
    if (results.length === 0) {
      results.push({
        provider: 'default',
        externalId: 0,
        title: query.trim(),
        overview: null,
        posterPath: null,
        backdropPath: null,
        voteAverage: null,
        releaseDate: null,
        originalLanguage: null,
        localPosterPath: null,
      });
    }

    return results;
  }

  /**
   * Obtiene detalles de un título en todos los proveedores públicos.
   * Devuelve el primer resultado exitoso.
   */
  async getDetails(
    externalId: number,
    provider: 'tvmaze' | 'jikan' | 'default',
    mediaType: 'movie' | 'tv' = 'tv',
  ): Promise<MetadataSearchResult> {
    // externalId === 0 indica un resultado local (sin proveedor externo).
    if (externalId === 0) {
      return {
        provider,
        externalId: 0,
        title: '',
        overview: null,
        posterPath: null,
        backdropPath: null,
        voteAverage: null,
        releaseDate: null,
        originalLanguage: null,
        localPosterPath: null,
      };
    }

    if (provider === 'tvmaze') {
      return this.getTvMazeDetails(externalId, mediaType);
    }
    if (provider === 'jikan') {
      return this.getJikanDetails(externalId);
    }
    return {
      provider: 'default',
      externalId,
      title: '',
      overview: null,
      posterPath: null,
      backdropPath: null,
      voteAverage: null,
      releaseDate: null,
      originalLanguage: null,
      localPosterPath: null,
    };
  }

  /**
   * Verifica si hay al menos un proveedor disponible.
   * Los proveedores públicos siempre están disponibles.
   */
  hasProviderConfigured(): boolean {
    return true;
  }

  /**
   * Verifica si un proveedor específico está configurado.
   * TVMaze y Jikan siempre están disponibles (no requieren API key).
   */
  isProviderConfigured(_provider: 'tvmaze' | 'jikan' | 'default'): boolean {
    return true;
  }
}

/**
 * Elimina etiquetas HTML de una cadena de texto.
 * TVMaze devuelve sinopsis con HTML (ej. <a>, <b>, <i>).
 */
function stripHtml(html: string | null): string | null {
  if (!html) return null;
  return html
    .replace(/<[^>]*>/g, '')
    .replace(/&nbsp;/g, ' ')
    .trim();
}
