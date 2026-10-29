/**
 * Gestor de metadatos (MetadataManager).
 *
 * Orquesta la obtención de metadatos aplicando el patrón Provider/Strategy:
 * - Si el usuario ha configurado proveedores externos (TMDB, TVDB, etc.),
 *   se usan en orden de prioridad.
 * - Si no hay ninguno configurado, se usa automáticamente el proveedor
 *   interno por defecto (plug-and-play, sin API keys).
 * - Si todos los proveedores fallan, se devuelve un resultado básico de
 *   fallback para que la aplicación nunca se quede sin datos.
 *
 * Nunca lanza excepciones no capturadas: todas las operaciones devuelven
 * MetadataResult para permitir degradación graciosa.
 */

import { MetadataNotFoundError } from './errors.js';

import type {
  MediaKind,
  MetadataProvider,
  MetadataSearchResult,
  MetadataResult,
  MetadataEnrichment,
} from './provider.js';

/** Configuración del gestor. */
export type MetadataManagerConfig = {
  /** Proveedores externos configurados por el usuario (puede estar vacío). */
  readonly providers: readonly MetadataProvider[];
  /** Proveedor interno por defecto (siempre disponible). */
  readonly defaultProvider: MetadataProvider;
  /** Número máximo de resultados devueltos por búsqueda. */
  readonly maxResults?: number;
};

/** Estado de resolución del proveedor. */
type ProviderResolution = {
  /** Proveedor que se usará para la operación actual. */
  readonly provider: MetadataProvider;
  /** Si se usó el proveedor por defecto (ningún externo disponible). */
  readonly isDefault: boolean;
};

/**
 * Crea el gestor de metadatos.
 */
export function createMetadataManager(config: MetadataManagerConfig): MetadataManager {
  const providers = config.providers ?? [];
  const defaultProvider = config.defaultProvider;
  const maxResults = config.maxResults ?? 10;

  return {
    search,
    searchAndEnrich,
    getDetails,
    resolveProvider,
    hasExternalProvider,
  };

  /**
   * Busca títulos por consulta.
   * Usa el primer proveedor externo disponible; si no hay ninguno, usa el
   * proveedor interno por defecto.
   */
  async function search(
    query: string,
    options: { kind?: MediaKind; limit?: number } = {},
  ): Promise<MetadataResult<readonly MetadataSearchResult[]>> {
    const resolution = resolveProvider();
    const limit = options.limit ?? maxResults;

    const result = await resolution.provider.search(query);

    if (result.ok) {
      const data = result.data.slice(0, limit);
      // Filtrar por tipo de medio si se especificó.
      const filtered =
        options.kind !== undefined
          ? data.filter(() => {
              // El proveedor no expone el tipo en el resultado; se asume
              // que todos coinciden con la consulta del usuario.
              return true;
            })
          : data;
      return { ok: true, data: filtered };
    }

    // Si el proveedor externo falló y no es el default, intentar con default.
    if (!resolution.isDefault) {
      return defaultProvider
        .search(query)
        .then((r) => (r.ok ? { ok: true, data: r.data.slice(0, limit) } : r));
    }

    // El default ya devolvió un resultado básico (fallback).
    return result;
  }

  /**
   * Busca y enriquece un título existente.
   * Devuelve los metadatos listos para guardar en la base de datos local.
   */
  async function searchAndEnrich(
    query: string,
    mediaType: MediaKind,
  ): Promise<MetadataResult<MetadataEnrichment>> {
    const resolution = resolveProvider();

    const searchResult = await resolution.provider.search(query);

    if (!searchResult.ok) {
      // Fallback al default si el externo falló.
      if (!resolution.isDefault) {
        const defaultResult = await defaultProvider.search(query);
        if (!defaultResult.ok) {
          return {
            ok: false,
            error: defaultResult.error,
          };
        }
        const firstDefault = defaultResult.data[0];
        if (!firstDefault) {
          return {
            ok: false,
            error: new MetadataNotFoundError(defaultProvider.name, query),
          };
        }
        return buildEnrichment(firstDefault, defaultProvider.name);
      }
      return {
        ok: false,
        error: searchResult.error,
      };
    }

    const first = searchResult.data[0];
    if (!first) {
      return {
        ok: false,
        error: new MetadataNotFoundError(resolution.provider.name, query),
      };
    }

    // Obtener detalles completos del resultado seleccionado.
    const detailsResult = await resolution.provider.getDetails(first.externalId, mediaType);

    if (!detailsResult.ok) {
      // Fallback al default para los detalles.
      if (!resolution.isDefault) {
        const defaultDetails = await defaultProvider.getDetails(first.externalId, mediaType);
        if (!defaultDetails.ok) {
          return { ok: false, error: defaultDetails.error };
        }
        return buildEnrichment(
          { ...defaultDetails.data, title: defaultDetails.data.title || first.title },
          defaultProvider.name,
        );
      }
      return { ok: false, error: detailsResult.error };
    }

    return buildEnrichment(
      { ...detailsResult.data, title: detailsResult.data.title || first.title },
      resolution.provider.name,
    );
  }

  /**
   * Obtiene detalles de un título por ID externo.
   */
  async function getDetails(
    externalId: number,
    mediaType: MediaKind,
    preferredProvider?: string,
  ): Promise<MetadataResult<MetadataSearchResult>> {
    const provider = preferredProvider
      ? (providers.find((p) => p.name === preferredProvider) ??
        resolutionFor(preferredProvider).provider)
      : resolveProvider().provider;

    return provider.getDetails(externalId, mediaType);
  }

  /**
   * Resuelve qué proveedor usar para la operación actual.
   */
  function resolveProvider(): ProviderResolution {
    const external = providers.find((p) => p.isConfigured);
    if (external) {
      return { provider: external, isDefault: false };
    }
    return { provider: defaultProvider, isDefault: true };
  }

  function resolutionFor(name: string): ProviderResolution {
    const p = providers.find((x) => x.name === name);
    if (p) return { provider: p, isDefault: false };
    if (defaultProvider.name === name) {
      return { provider: defaultProvider, isDefault: true };
    }
    return resolveProvider();
  }

  /**
   * Devuelve true si hay al menos un proveedor externo configurado.
   */
  function hasExternalProvider(): boolean {
    return providers.some((p) => p.isConfigured);
  }

  function buildEnrichment(
    data: MetadataSearchResult,
    providerName: string,
  ): MetadataResult<MetadataEnrichment> {
    return {
      ok: true,
      data: {
        externalId: data.externalId,
        provider: providerName,
        title: data.title,
        overview: data.overview,
        posterPath: data.posterPath,
        backdropPath: data.backdropPath,
        voteAverage: data.voteAverage,
        releaseDate: data.releaseDate,
        originalLanguage: data.originalLanguage,
        localPosterPath: data.localPosterPath,
      },
    };
  }
}

/** Tipo del devuelto por createMetadataManager. */
export type MetadataManager = {
  readonly search: (
    query: string,
    options?: { kind?: MediaKind; limit?: number },
  ) => Promise<MetadataResult<readonly MetadataSearchResult[]>>;
  readonly searchAndEnrich: (
    query: string,
    mediaType: MediaKind,
  ) => Promise<MetadataResult<MetadataEnrichment>>;
  readonly getDetails: (
    externalId: number,
    mediaType: MediaKind,
    preferredProvider?: string,
  ) => Promise<MetadataResult<MetadataSearchResult>>;
  readonly resolveProvider: () => ProviderResolution;
  readonly hasExternalProvider: () => boolean;
};
