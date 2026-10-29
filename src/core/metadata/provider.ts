/**
 * Tipos comunes y contrato (interface) para proveedores de metadatos.
 *
 * Patrón Strategy / Provider: cualquier proveedor (TMDB, TVDB, OMDB, AniList
 * o un proveedor interno por defecto) implementa esta interfaz, de modo que el
 * orquestador no conoce detalles de implementación.
 */

import type { MetadataProviderError } from './errors.js';

export type MediaKind = 'movie' | 'tv';

/** Resultado normalizado de búsqueda de metadatos. */
export type MetadataSearchResult = {
  /** Identificador externo en el proveedor. */
  readonly externalId: number;
  /** Título del título. */
  readonly title: string;
  /** Sinopsis (puede ser null). */
  readonly overview: string | null;
  /** Ruta del póster en el proveedor (puede ser null). */
  readonly posterPath: string | null;
  /** Ruta del backdrop en el proveedor (puede ser null). */
  readonly backdropPath: string | null;
  /** Puntuación media (puede ser null). */
  readonly voteAverage: number | null;
  /** Fecha de estreno / primera emisión (puede ser null). */
  readonly releaseDate: string | null;
  /** Idioma original (puede ser null). */
  readonly originalLanguage: string | null;
  /** Ruta local al póster descargado en caché (puede ser null). */
  readonly localPosterPath: string | null;
};

/** Resultado de enriquecimiento de un título existente. */
export type MetadataEnrichment = {
  readonly externalId: number;
  readonly provider: string;
  readonly title: string;
  readonly overview: string | null;
  readonly posterPath: string | null;
  readonly backdropPath: string | null;
  readonly voteAverage: number | null;
  readonly releaseDate: string | null;
  readonly originalLanguage: string | null;
  readonly localPosterPath: string | null;
};

/** Tipo discriminado para el resultado de una operación de metadatos. */
export type MetadataResult<TData> =
  | { readonly ok: true; readonly data: TData }
  | { readonly ok: false; readonly error: MetadataProviderError };

/**
 * Contrato común que debe cumplir todo proveedor de metadatos.
 *
 * Implementa el patrón Strategy para que MetadataManager pueda intercambiar
 * proveedores sin cambiar su lógica.
 */
export type MetadataProvider = {
  /** Nombre legible del proveedor (ej. 'tmdb', 'tvdb', 'default'). */
  readonly name: string;
  /** Si el proveedor está listo para usarse (tiene claves/configuración). */
  readonly isConfigured: boolean;
  /** Tipo de medio que soporta principalmente. */
  readonly kind: MediaKind;

  /**
   * Busca títulos por consulta de texto.
   * Nunca lanza: devuelve MetadataResult con error si algo falla.
   */
  search(query: string): Promise<MetadataResult<readonly MetadataSearchResult[]>>;

  /**
   * Obtiene detalles de un título por su ID externo.
   * Nunca lanza: devuelve MetadataResult con error si algo falla.
   */
  getDetails(
    externalId: number,
    mediaType: MediaKind,
  ): Promise<MetadataResult<MetadataSearchResult>>;
};
