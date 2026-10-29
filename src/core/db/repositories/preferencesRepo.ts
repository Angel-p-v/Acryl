import { DbError } from '../../../shared/errors.js';

import type { Db } from '../connection.js';
import type { MediaPreferenceRow } from '../types.js';

type PreferenceDbRow = {
  media_id: number;
  favorite: number;
  created_at: string;
  updated_at: string;
};

export type SetFavoriteInput = {
  mediaId: number;
  favorite: boolean;
  updatedAt: string;
  /** Solo se usa al crear la fila; por defecto, `updatedAt`. */
  createdAt?: string;
};

export type PreferenceRepository = {
  get(mediaId: number): MediaPreferenceRow | undefined;
  isFavorite(mediaId: number): boolean;
  setFavorite(input: SetFavoriteInput): MediaPreferenceRow;
  remove(mediaId: number): boolean;
  /** Títulos marcados como favoritos, del más reciente al más antiguo. */
  listFavorites(limit?: number): MediaPreferenceRow[];
  countFavorites(): number;
};

/** Crea el repositorio de preferencias de títulos sobre una conexión SQLite dada. */
export function createPreferenceRepository(db: Db): PreferenceRepository {
  const setStatement = db.prepare(`
    INSERT INTO media_preferences (media_id, favorite, created_at, updated_at)
    VALUES (@media_id, @favorite, @created_at, @updated_at)
    ON CONFLICT(media_id) DO UPDATE SET
      favorite = excluded.favorite,
      updated_at = excluded.updated_at
    RETURNING *
  `);

  const getStatement = db.prepare('SELECT * FROM media_preferences WHERE media_id = ?');
  const removeStatement = db.prepare('DELETE FROM media_preferences WHERE media_id = ?');
  const listFavoritesStatement = db.prepare<[number], PreferenceDbRow>(
    'SELECT * FROM media_preferences WHERE favorite = 1 ORDER BY updated_at DESC, media_id ASC LIMIT ?',
  );
  const countFavoritesStatement = db.prepare<[], { total: number }>(
    'SELECT COUNT(*) AS total FROM media_preferences WHERE favorite = 1',
  );

  const getRow = (mediaId: number): MediaPreferenceRow | undefined => {
    const row = getStatement.get(mediaId) as PreferenceDbRow | undefined;
    return row ? toPreferenceRow(row) : undefined;
  };

  return {
    get: getRow,

    isFavorite(mediaId: number): boolean {
      return getRow(mediaId)?.favorite ?? false;
    },

    setFavorite(input: SetFavoriteInput): MediaPreferenceRow {
      const row = setStatement.get({
        media_id: input.mediaId,
        favorite: input.favorite ? 1 : 0,
        created_at: input.createdAt ?? input.updatedAt,
        updated_at: input.updatedAt,
      }) as PreferenceDbRow | undefined;

      if (!row) throw new DbError('No se pudo guardar la preferencia');
      return toPreferenceRow(row);
    },

    remove(mediaId: number): boolean {
      return removeStatement.run(mediaId).changes > 0;
    },

    listFavorites(limit = 50): MediaPreferenceRow[] {
      return listFavoritesStatement.all(limit).map(toPreferenceRow);
    },

    countFavorites(): number {
      return countFavoritesStatement.get()?.total ?? 0;
    },
  };
}

/** Convierte una fila de SQLite al tipo del dominio. */
function toPreferenceRow(row: PreferenceDbRow): MediaPreferenceRow {
  return {
    mediaId: row.media_id,
    favorite: row.favorite === 1,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  };
}
