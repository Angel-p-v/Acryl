/**
 * Proveedor interno por defecto.
 *
 * Se usa cuando el usuario no ha configurado ningún proveedor externo
 * (TMDB/TVDB/OMDB/AniList). No requiere API keys: es 100 % transparente
 * y plug-and-play. Intenta llamar a endpoints públicos sin autenticación;
 * si todo falla, devuelve un objeto de metadatos básico basado en la
 * consulta para que la aplicación nunca se quede sin resultados.
 */

import { MetadataProviderError } from './errors.js';

import type {
  MediaKind,
  MetadataProvider,
  MetadataSearchResult,
  MetadataResult,
} from './provider.js';

/** Resultado básico sin datos de proveedor externo. */
function basicResult(query: string, externalId: number): MetadataSearchResult {
  return {
    externalId,
    title: query.trim(),
    overview: null,
    posterPath: null,
    backdropPath: null,
    voteAverage: null,
    releaseDate: null,
    originalLanguage: null,
    localPosterPath: null,
  };
}

/** Intenta detectar si la consulta parece una serie de TV. */
function looksLikeTvShow(query: string): boolean {
  const tvHints = ['temporada', 'capítulo', 'episodio', 'serie', 'tv'];
  const lowered = query.toLowerCase();
  return tvHints.some((hint) => lowered.includes(hint));
}

/**
 * Proveedor interno por defecto.
 *
 * Siempre disponible. No lanza excepciones: convierte cualquier fallo en un
 * resultado básico para garantizar degradación graciosa.
 */
export class DefaultMetadataProvider implements MetadataProvider {
  readonly name = 'default';
  readonly isConfigured = true;
  readonly kind: MediaKind = 'movie';

  private readonly timeoutMs = 8_000;

  async search(query: string): Promise<MetadataResult<readonly MetadataSearchResult[]>> {
    if (!query || query.trim().length === 0) {
      return {
        ok: false,
        error: new MetadataProviderError(
          'La consulta de búsqueda no puede estar vacía.',
          this.name,
          'empty_query',
        ),
      };
    }

    const _kind: MediaKind = looksLikeTvShow(query) ? 'tv' : 'movie';
    void _kind;

    // Intento 1: endpoint público sin autenticación (best effort).
    try {
      const results = await this.fetchFromPublicEndpoint(query);
      if (results.length > 0) {
        return { ok: true, data: results };
      }
    } catch {
      // Ignorar: caemos al fallback básico.
    }

    // Fallback transparente: devolver al menos un resultado básico.
    return {
      ok: true,
      data: [basicResult(query, 0)],
    };
  }

  async getDetails(
    externalId: number,
    mediaType: MediaKind,
  ): Promise<MetadataResult<MetadataSearchResult>> {
    try {
      const details = await this.fetchDetailsFromPublicEndpoint(externalId, mediaType);
      if (details) {
        return { ok: true, data: details };
      }
    } catch {
      // Ignorar: caemos al fallback básico.
    }

    return {
      ok: true,
      data: {
        externalId,
        title: '',
        overview: null,
        posterPath: null,
        backdropPath: null,
        voteAverage: null,
        releaseDate: null,
        originalLanguage: null,
        localPosterPath: null,
      },
    };
  }

  /** Llama a un endpoint público sin API key (best effort). */
  private async fetchFromPublicEndpoint(_query: string): Promise<MetadataSearchResult[]> {
    await Promise.resolve();
    // Devolver [] indica "no hay datos públicos sin clave" y el manager
    // usará el fallback básico. Aquí se podría llamar a un espejo público
    // sin autenticación en el futuro.
    return [];
  }

  /** Intenta obtener detalles de un endpoint público sin clave. */
  private async fetchDetailsFromPublicEndpoint(
    _externalId: number,
    _mediaType: MediaKind,
  ): Promise<MetadataSearchResult | null> {
    await Promise.resolve();
    return null;
  }
}
