import { DbError } from '../../../shared/errors.js';

import type { Db } from '../connection.js';

export type MediaMetadataRow = {
  mediaId: number;
  provider: string;
  externalId: number;
  title: string | null;
  overview: string | null;
  posterPath: string | null;
  backdropPath: string | null;
  voteAverage: number | null;
  firstAirDate: string | null;
  releaseDate: string | null;
  originalLanguage: string | null;
  fetchedAt: string;
  updatedAt: string;
  /** Ruta local al póster descargado en caché, si existe. */
  localPosterPath: string | null;
};

type MetadataDbRow = {
  media_id: number;
  provider: string;
  external_id: number;
  title: string | null;
  overview: string | null;
  poster_path: string | null;
  backdrop_path: string | null;
  vote_average: number | null;
  first_air_date: string | null;
  release_date: string | null;
  original_language: string | null;
  fetched_at: string;
  updated_at: string;
  local_poster_path: string | null;
};

export type SetMetadataInput = {
  mediaId: number;
  provider: string;
  externalId: number;
  title?: string | null;
  overview?: string | null;
  posterPath?: string | null;
  backdropPath?: string | null;
  voteAverage?: number | null;
  firstAirDate?: string | null;
  releaseDate?: string | null;
  originalLanguage?: string | null;
  updatedAt: string;
  localPosterPath?: string | null;
};

/** Repositorio de metadatos externos (TMDB/TVDB). */
export type MetadataRepository = {
  /** Obtiene metadatos de un título. */
  get(mediaId: number): MediaMetadataRow | undefined;
  /** Busca metadatos por proveedor e id externo. */
  searchByExternalId(provider: string, externalId: number): MediaMetadataRow | undefined;
  /** Inserta o actualiza metadatos. */
  set(input: SetMetadataInput): MediaMetadataRow;
  /** Elimina metadatos de un título. */
  remove(mediaId: number): boolean;
  /** Elimina todos los metadatos (usado para forzar refresco al configurar API keys). */
  removeAll(): number;
  /** Obtiene metadatos cuyo fetched_at es anterior a ttlHours horas (expirados). */
  listExpired(ttlHours: number): MediaMetadataRow[];
};

/** Crea el repositorio de metadatos sobre una conexión SQLite dada. */
export function createMetadataRepository(db: Db): MetadataRepository {
  const setStatement = db.prepare(`
    INSERT INTO media_metadata (
      media_id, provider, external_id, title, overview, poster_path, backdrop_path,
      vote_average, first_air_date, release_date, original_language, fetched_at, updated_at,
      local_poster_path
    )
    VALUES (@media_id, @provider, @external_id, @title, @overview, @poster_path, @backdrop_path,
            @vote_average, @first_air_date, @release_date, @original_language, @fetched_at, @updated_at,
            @local_poster_path)
    ON CONFLICT(media_id) DO UPDATE SET
      provider = excluded.provider,
      external_id = excluded.external_id,
      title = excluded.title,
      overview = excluded.overview,
      poster_path = excluded.poster_path,
      backdrop_path = excluded.backdrop_path,
      vote_average = excluded.vote_average,
      first_air_date = excluded.first_air_date,
      release_date = excluded.release_date,
      original_language = excluded.original_language,
      fetched_at = excluded.fetched_at,
      updated_at = excluded.updated_at,
      local_poster_path = excluded.local_poster_path
    RETURNING *
  `);

  const getStatement = db.prepare('SELECT * FROM media_metadata WHERE media_id = ?');
  const searchByExternalIdStatement = db.prepare<[string, number], MetadataDbRow>(`
    SELECT * FROM media_metadata WHERE provider = ? AND external_id = ?
  `);
  const removeStatement = db.prepare('DELETE FROM media_metadata WHERE media_id = ?');

  const toMetadataRow = (row: MetadataDbRow): MediaMetadataRow => ({
    mediaId: row.media_id,
    provider: row.provider,
    externalId: row.external_id,
    title: row.title,
    overview: row.overview,
    posterPath: row.poster_path,
    backdropPath: row.backdrop_path,
    voteAverage: row.vote_average,
    firstAirDate: row.first_air_date,
    releaseDate: row.release_date,
    originalLanguage: row.original_language,
    fetchedAt: row.fetched_at,
    updatedAt: row.updated_at,
    localPosterPath: row.local_poster_path,
  });

  return {
    get(mediaId: number): MediaMetadataRow | undefined {
      const row = getStatement.get(mediaId) as MetadataDbRow | undefined;
      return row ? toMetadataRow(row) : undefined;
    },

    searchByExternalId(provider: string, externalId: number): MediaMetadataRow | undefined {
      const row = searchByExternalIdStatement.get(provider, externalId);
      return row ? toMetadataRow(row) : undefined;
    },

    set(input: SetMetadataInput): MediaMetadataRow {
      const now = new Date().toISOString();
      const row = setStatement.get({
        media_id: input.mediaId,
        provider: input.provider,
        external_id: input.externalId,
        title: input.title ?? null,
        overview: input.overview ?? null,
        poster_path: input.posterPath ?? null,
        backdrop_path: input.backdropPath ?? null,
        vote_average: input.voteAverage ?? null,
        first_air_date: input.firstAirDate ?? null,
        release_date: input.releaseDate ?? null,
        original_language: input.originalLanguage ?? null,
        fetched_at: now,
        updated_at: input.updatedAt,
        local_poster_path: input.localPosterPath ?? null,
      }) as MetadataDbRow | undefined;

      if (!row) throw new DbError('No se pudieron guardar los metadatos');
      return toMetadataRow(row);
    },

    remove(mediaId: number): boolean {
      return removeStatement.run(mediaId).changes > 0;
    },

    /** Elimina todos los metadatos de la base de datos. */
    removeAll(): number {
      return db.prepare('DELETE FROM media_metadata').run().changes;
    },

    /** Obtiene metadatos cuyo fetched_at es anterior a ttlHours horas (expirados). */
    listExpired(ttlHours: number): MediaMetadataRow[] {
      const cutoff = new Date(Date.now() - ttlHours * 60 * 60 * 1000).toISOString();
      const rows = db
        .prepare<[string], MetadataDbRow>(
          'SELECT * FROM media_metadata WHERE fetched_at < ? ORDER BY fetched_at ASC',
        )
        .all(cutoff);
      return rows.map(toMetadataRow);
    },
  };
}
