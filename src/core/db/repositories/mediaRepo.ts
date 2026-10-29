import { DbError } from '../../../shared/errors.js';

import type { MediaType } from '../../config/schema.js';
import type { Db } from '../connection.js';
import type { MediaInput, MediaRow } from '../types.js';

type MediaDbRow = {
  id: number;
  type: MediaType;
  title: string;
  sort_title: string;
  year: number | null;
  folder_path: string;
  created_at: string;
  updated_at: string;
};

export type ListMediaOptions = {
  type?: MediaType;
  limit?: number;
  offset?: number;
  orderBy?: 'title' | 'added' | 'recent';
  /** Filtrar solo favoritos. */
  favorites?: boolean;
  /** Filtrar títulos en curso (con progreso guardado y no completados). */
  inProgress?: boolean;
  /** Filtrar títulos no vistos (sin progreso o completado). */
  unwatched?: boolean;
  /** Filtrar títulos completados. */
  watched?: boolean;
  /** Filtrar títulos que tengan al menos uno de los tags indicados (IDs). */
  tags?: number[];
};

export type SearchMediaOptions = {
  type?: MediaType;
  limit?: number;
};

export type MediaRepository = {
  upsert(input: MediaInput): MediaRow;
  findById(id: number): MediaRow | undefined;
  findByFolderPath(folderPath: string): MediaRow | undefined;
  list(options?: ListMediaOptions): MediaRow[];
  search(query: string, options?: SearchMediaOptions): MediaRow[];
  remove(id: number): boolean;
  removeEmpty(): number;
  count(type?: MediaType): number;
  listFolders(): string[];
  /** Actualiza la ruta de una carpeta (rename/move). Devuelve true si se actualizó. */
  updateFolderPath(oldPath: string, newPath: string): boolean;
};

export type MediaRepositoryDeps = {
  /** Repositorio de preferencias (para filtro de favoritos). */
  preferenceRepo?: {
    listFavorites(limit?: number): { mediaId: number }[];
    countFavorites(): number;
  };
  /** Repositorio de reproducción (para filtros inProgress, unwatched, watched). */
  playbackRepo?: {
    listInProgress(limit?: number): { media: { id: number } }[];
    listRecent(limit?: number): { media: { id: number }; state: { completedAt: string | null } }[];
    count(): number;
  };
  /** Repositorio de etiquetas (para filtro por tag). */
  tagRepo?: {
    listMediaByTag(tagId: number): number[];
  };
};

/** Crea el repositorio de títulos sobre una conexión SQLite dada. */
export function createMediaRepository(db: Db, deps: MediaRepositoryDeps = {}): MediaRepository {
  const { preferenceRepo, playbackRepo, tagRepo } = deps;
  const upsertStatement = db.prepare(`
    INSERT INTO media (type, title, sort_title, year, folder_path)
    VALUES (@type, @title, @sort_title, @year, @folder_path)
    ON CONFLICT(folder_path) DO UPDATE SET
      type = excluded.type,
      title = excluded.title,
      sort_title = excluded.sort_title,
      year = excluded.year,
      updated_at = datetime('now')
    RETURNING id
  `);

  const findByIdStatement = db.prepare('SELECT * FROM media WHERE id = ?');
  const findByFolderStatement = db.prepare('SELECT * FROM media WHERE folder_path = ?');
  const removeStatement = db.prepare('DELETE FROM media WHERE id = ?');
  const removeEmptyStatement = db.prepare(`
    DELETE FROM media
    WHERE NOT EXISTS (SELECT 1 FROM media_files WHERE media_files.media_id = media.id)
  `);
  const countStatement = db.prepare<[MediaType], { total: number }>(
    'SELECT COUNT(*) AS total FROM media WHERE type = ?',
  );
  const countAllStatement = db.prepare<[], { total: number }>(
    'SELECT COUNT(*) AS total FROM media',
  );
  const updateFolderPathStatement = db.prepare(
    "UPDATE media SET folder_path = @newPath, updated_at = datetime('now') WHERE folder_path = @oldPath",
  );
  const foldersStatement = db.prepare<[], { folder_path: string }>(
    'SELECT folder_path FROM media ORDER BY sort_title',
  );

  const listStatements = {
    title: db.prepare<{ type: MediaType | null; limit: number; offset: number }, MediaDbRow>(`
      SELECT * FROM media
      WHERE (@type IS NULL OR type = @type)
      ORDER BY sort_title ASC
      LIMIT @limit OFFSET @offset
    `),
    added: db.prepare<{ type: MediaType | null; limit: number; offset: number }, MediaDbRow>(`
      SELECT * FROM media
      WHERE (@type IS NULL OR type = @type)
      ORDER BY created_at DESC
      LIMIT @limit OFFSET @offset
    `),
    recent: db.prepare<{ type: MediaType | null; limit: number; offset: number }, MediaDbRow>(`
      SELECT * FROM media
      WHERE (@type IS NULL OR type = @type)
      ORDER BY updated_at DESC
      LIMIT @limit OFFSET @offset
    `),
  };

  const searchStatement = db.prepare<
    { query: string; limit: number; type: MediaType | null },
    MediaDbRow
  >(`
    SELECT m.* FROM media m
    JOIN media_fts f ON f.rowid = m.id
    WHERE media_fts MATCH @query
      AND (@type IS NULL OR m.type = @type)
    ORDER BY bm25(media_fts), m.sort_title
    LIMIT @limit
  `);

  return {
    upsert(input: MediaInput): MediaRow {
      const sortTitle = input.title.toLocaleLowerCase();
      const result = upsertStatement.get({
        type: input.type,
        title: input.title,
        sort_title: sortTitle,
        year: input.year ?? null,
        folder_path: input.folderPath,
      }) as { id: number } | undefined;

      if (!result) throw new DbError('No se pudo guardar el título');
      const row = findByIdStatement.get(result.id) as MediaDbRow | undefined;
      if (!row) throw new DbError('No se pudo leer el título guardado');
      return toMediaRow(row);
    },

    findById(id: number): MediaRow | undefined {
      const row = findByIdStatement.get(id) as MediaDbRow | undefined;
      return row ? toMediaRow(row) : undefined;
    },

    findByFolderPath(folderPath: string): MediaRow | undefined {
      const row = findByFolderStatement.get(folderPath) as MediaDbRow | undefined;
      return row ? toMediaRow(row) : undefined;
    },

    /**
     * Comportamiento de los filtros watched/unwatched:
     *
     * - watched: título tiene AL MENOS UN archivo con completedAt !== null
     * - unwatched: título NO TIENE archivos con completedAt !== null
     *   (incluye títulos nunca reproducidos y parcialmente vistos sin completar)
     *
     * Ambos filtros usan una subconsulta SQL nativa para evitar el límite hardcoded
     * de 10000 entradas y aprovechar los índices de la BD.
     */
    list(options: ListMediaOptions = {}): MediaRow[] {
      const { favorites, inProgress, unwatched, watched, tags, ...baseOptions } = options;

      // Si no hay filtros especiales, usar la consulta simple
      if (!favorites && !inProgress && !unwatched && !watched && (!tags || tags.length === 0)) {
        const statement = listStatements[baseOptions.orderBy ?? 'title'];
        const rows = statement.all({
          type: baseOptions.type ?? null,
          limit: baseOptions.limit ?? 500,
          offset: baseOptions.offset ?? 0,
        });
        return rows.map(toMediaRow);
      }

      // Para filtros que requieren joins, construimos la consulta dinámicamente
      const conditions: string[] = [];
      const params: Record<string, unknown> = {
        limit: baseOptions.limit ?? 500,
        offset: baseOptions.offset ?? 0,
      };

      if (baseOptions.type) {
        conditions.push('m.type = @type');
        params.type = baseOptions.type;
      }

      const orderByClause = (() => {
        switch (baseOptions.orderBy) {
          case 'added':
            return 'm.created_at DESC';
          case 'recent':
            return 'm.updated_at DESC';
          case 'title':
          default:
            return 'm.sort_title ASC';
        }
      })();

      // Helper: chunk array to respect SQLite parameter limit (999)
      const chunkArray = <T>(arr: T[], size: number): T[][] => {
        const chunks: T[][] = [];
        for (let i = 0; i < arr.length; i += size) {
          chunks.push(arr.slice(i, i + size));
        }
        return chunks;
      };

      // Helper: add IN clause with parameterization and chunking
      // Uses a unique prefix for each call to avoid parameter name collisions
      let inClauseCounter = 0;
      const addInClause = (field: string, ids: Set<number>, isNotIn = false): void => {
        const idArray = Array.from(ids);
        if (idArray.length === 0) return;

        const prefix = `${field}_${inClauseCounter++}`;
        const chunks = chunkArray(idArray, 500); // well under 999 limit
        if (chunks.length === 1) {
          const placeholders = idArray.map((_, i) => `@${prefix}_${i}`).join(',');
          conditions.push(`m.${field} ${isNotIn ? 'NOT ' : ''}IN (${placeholders})`);
          idArray.forEach((id, i) => {
            params[`${prefix}_${i}`] = id;
          });
        } else {
          // Multiple chunks: combine with OR
          const chunkConditions = chunks.map((chunk, chunkIndex) => {
            const placeholders = chunk.map((_, i) => `@${prefix}_${chunkIndex}_${i}`).join(',');
            chunk.forEach((id, i) => {
              params[`${prefix}_${chunkIndex}_${i}`] = id;
            });
            return `m.${field} ${isNotIn ? 'NOT ' : ''}IN (${placeholders})`;
          });
          conditions.push(`(${chunkConditions.join(' OR ')})`);
        }
      };

      // Favoritos: join con media_preferences
      if (favorites) {
        if (!preferenceRepo) return [];
        const favIds = new Set(preferenceRepo.listFavorites().map((p) => p.mediaId));
        if (favIds.size === 0) return [];
        addInClause('id', favIds);
      }

      // Etiquetas: join con media_tags (AND si varios tags)
      if (tags && tags.length > 0) {
        if (!tagRepo) return [];
        const tagRepoRef = tagRepo;
        // Para cada tag, obtener los media_ids y hacer intersección (AND)
        const tagMediaSets: Set<number>[] = tags.map((tagId) => {
          const ids: number[] = tagRepoRef.listMediaByTag(tagId);
          return new Set(ids);
        });
        // Intersección de todos los sets (AND entre tags)
        const commonIds = tagMediaSets.reduce<Set<number>>((acc, set) => {
          const result = new Set<number>();
          for (const id of set) {
            if (acc.has(id)) result.add(id);
          }
          return result;
        }, new Set(tagMediaSets[0]));
        if (commonIds.size === 0) return [];
        addInClause('id', commonIds);
      }

      // En curso: join con playback_state (no completados, posición > 0)
      if (inProgress) {
        if (!playbackRepo) return [];
        const inProgressIds = new Set(playbackRepo.listInProgress().map((e) => e.media.id));
        if (inProgressIds.size === 0) return [];
        addInClause('id', inProgressIds);
      }

      // Vistos / No vistos: usar subconsulta en lugar de hardcoded 10000
      if (watched || unwatched) {
        if (!playbackRepo) return [];
        // Subconsulta para obtener media_ids con al menos un completedAt
        const completedSubquery = `
          SELECT DISTINCT m2.id
          FROM media m2
          JOIN media_files f2 ON f2.media_id = m2.id
          JOIN playback_state p2 ON p2.media_file_id = f2.id
          WHERE p2.completed_at IS NOT NULL
        `;

        if (watched) {
          // Títulos con al menos un fichero completado
          conditions.push(`m.id IN (${completedSubquery})`);
        } else if (unwatched) {
          // Títulos que NO tienen ningún fichero completado
          conditions.push(`m.id NOT IN (${completedSubquery})`);
        }
      }

      const whereClause = conditions.length > 0 ? `WHERE ${conditions.join(' AND ')}` : '';
      const sql = `
        SELECT * FROM media m
        ${whereClause}
        ORDER BY ${orderByClause}
        LIMIT @limit OFFSET @offset
      `;

      const statement = db.prepare(sql);
      const rows = statement.all(params) as MediaDbRow[];
      return rows.map(toMediaRow);
    },

    search(query: string, options: SearchMediaOptions = {}): MediaRow[] {
      const match = toFtsQuery(query);
      let rows: MediaDbRow[] = [];
      if (match) {
        rows = searchStatement.all({
          query: match,
          limit: options.limit ?? 20,
          type: options.type ?? null,
        });
      }
      // Fallback: si FTS5 no devuelve nada (errores de escritura, términos
      // parciales, etc.), prueba con LIKE como búsqueda difusa sobre el título.
      if (rows.length === 0 && query.trim().length > 0) {
        const likePattern = `%${escapeLike(query)}%`;
        const fallback = db.prepare<
          { like: string; type: MediaType | null; limit: number },
          MediaDbRow
        >(`
          SELECT * FROM media
          WHERE sort_title LIKE @like
            AND (@type IS NULL OR type = @type)
          ORDER BY sort_title ASC
          LIMIT @limit
        `);
        rows = fallback.all({
          like: likePattern,
          type: options.type ?? null,
          limit: options.limit ?? 20,
        });
      }
      return rows.map(toMediaRow);
    },

    remove(id: number): boolean {
      return removeStatement.run(id).changes > 0;
    },

    /** Elimina los títulos que se han quedado sin ficheros. */
    removeEmpty(): number {
      return removeEmptyStatement.run().changes;
    },

    count(type?: MediaType): number {
      if (type) return countStatement.get(type)?.total ?? 0;
      return countAllStatement.get()?.total ?? 0;
    },

    listFolders(): string[] {
      return foldersStatement.all().map((row) => row.folder_path);
    },

    /** Actualiza la ruta de una carpeta (rename/move). Devuelve true si se actualizó. */
    updateFolderPath(oldPath: string, newPath: string): boolean {
      return updateFolderPathStatement.run({ oldPath, newPath }).changes > 0;
    },
  };
}

/** Convierte una fila de SQLite al tipo del dominio. */
function toMediaRow(row: MediaDbRow): MediaRow {
  return {
    id: row.id,
    type: row.type,
    title: row.title,
    sortTitle: row.sort_title,
    year: row.year,
    folderPath: row.folder_path,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  };
}

/**
 * Convierte una consulta del usuario en una expresión FTS5 segura.
 * Se escapan las comillas y se añade el comodín de prefijo a cada término.
 * Los términos se unen con OR para que basten con que coincida cualquiera de
 * ellos (búsqueda más tolerante a errores de escritura o palabras sueltas).
 */
export function toFtsQuery(query: string): string {
  const terms = query
    .replace(/["*]/g, ' ')
    .split(/\s+/)
    .filter((term) => term.length > 0);

  if (terms.length === 0) return '';
  return terms.map((term) => `"${term}"*`).join(' OR ');
}

/**
 * Escapa los caracteres especiales de LIKE (%, _) y la barra invertida para
 * usarlos de forma segura en un patrón LIKE.
 */
function escapeLike(value: string): string {
  return value.replace(/[\\%_]/g, (char) => `\\${char}`);
}
