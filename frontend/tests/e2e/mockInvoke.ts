import Database from 'better-sqlite3';

import { E2E_DB_PATH } from './setup';

/**
 * Mock de `invoke` de @tauri-apps/api para pruebas E2E.
 *
 * Ejecuta comandos SQL directamente contra la base de datos SQLite temporal,
 * permitiendo que las pruebas E2E funcionen sin necesidad de la aplicación
 * Tauri en ejecución.
 */

const db = new Database(E2E_DB_PATH, { readonly: false });

async function mockInvoke<T>(cmd: string, args: Record<string, unknown>): Promise<T> {
  switch (cmd) {
    case 'list_media_cmd': {
      const type = args.type as string | undefined;
      const limit = (args.limit as number) ?? 500;
      const offset = (args.offset as number) ?? 0;
      const orderBy = (args.orderBy as string) ?? 'title';

      let sql = `
        SELECT id, type, title, sort_title, year, folder_path, created_at, updated_at,
               is_favorite, is_watched, poster_path, synopsis, metadata_fetched_at
        FROM media
      `;
      const params: unknown[] = [];

      if (type) {
        sql += ' WHERE type = ?';
        params.push(type);
      }

      const orderClause =
        orderBy === 'added'
          ? 'ORDER BY created_at DESC'
          : orderBy === 'recent'
            ? 'ORDER BY updated_at DESC'
            : 'ORDER BY sort_title ASC';
      sql += ` ${orderClause} LIMIT ? OFFSET ?`;
      params.push(limit, offset);

      const mediaRows = db.prepare(sql).all(params) as {
        id: number;
        type: string;
        title: string;
        sort_title: string;
        year: number | null;
        folder_path: string;
        created_at: string;
        updated_at: string;
        is_favorite: number;
        is_watched: number;
        poster_path: string | null;
        synopsis: string | null;
        metadata_fetched_at: string | null;
      }[];

      const media = mediaRows.map((m) => ({
        ...m,
        is_favorite: Boolean(m.is_favorite),
        is_watched: Boolean(m.is_watched),
      })) as T[];

      let totalRow: { count: number };
      if (type) {
        totalRow = db
          .prepare('SELECT COUNT(*) as count FROM media WHERE type = ?')
          .get(params[0]) as { count: number };
      } else {
        totalRow = db.prepare('SELECT COUNT(*) as count FROM media').get() as { count: number };
      }

      return { media, total: totalRow.count } as unknown as T;
    }

    case 'search_media_cmd': {
      const query = (args.query as string) ?? '';
      const type = args.type as string | undefined;
      const limit = (args.limit as number) ?? 20;

      if (!query.trim()) {
        const listResult = await mockInvoke<{
          media: unknown[];
          total: number;
        }>('list_media_cmd', {
          ...args,
          limit,
        });
        return listResult.media as unknown as T;
      }

      let sql = `
        SELECT m.id, m.type, m.title, m.sort_title, m.year, m.folder_path,
               m.created_at, m.updated_at,
               m.is_favorite, m.is_watched, m.poster_path, m.synopsis,
               m.metadata_fetched_at
        FROM media m
        JOIN media_fts f ON f.rowid = m.id
        WHERE media_fts MATCH ?
      `;
      const params: unknown[] = [query];

      if (type) {
        sql += ' AND m.type = ?';
        params.push(type);
      }

      sql += ' ORDER BY bm25(media_fts), m.sort_title LIMIT ?';
      params.push(limit);

      const mediaRows = db.prepare(sql).all(params) as {
        id: number;
        type: string;
        title: string;
        sort_title: string;
        year: number | null;
        folder_path: string;
        created_at: string;
        updated_at: string;
        is_favorite: number;
        is_watched: number;
        poster_path: string | null;
        synopsis: string | null;
        metadata_fetched_at: string | null;
      }[];

      const media = mediaRows.map((m) => ({
        ...m,
        is_favorite: Boolean(m.is_favorite),
        is_watched: Boolean(m.is_watched),
      })) as T[];

      return media;
    }

    case 'count_media_cmd': {
      const type = args.type as string | undefined;
      if (type) {
        const row = db.prepare('SELECT COUNT(*) as count FROM media WHERE type = ?').get(type) as {
          count: number;
        };
        return row.count as unknown as T;
      }
      const row = db.prepare('SELECT COUNT(*) as count FROM media').get() as {
        count: number;
      };
      return row.count as unknown as T;
    }

    case 'list_folders_cmd': {
      const rows = db.prepare('SELECT folder_path FROM media ORDER BY sort_title').all() as {
        folder_path: string;
      }[];
      return rows.map((r) => r.folder_path) as unknown as T;
    }

    case 'get_media_details_cmd': {
      const mediaId = args.media_id as number;
      const row = db
        .prepare(
          `SELECT id, type, title, sort_title, year, folder_path, created_at,
                  updated_at, is_favorite, is_watched, poster_path, synopsis,
                  metadata_fetched_at
           FROM media WHERE id = ?`
        )
        .get(mediaId) as
        | {
            id: number;
            type: string;
            title: string;
            sort_title: string;
            year: number | null;
            folder_path: string;
            created_at: string;
            updated_at: string;
            is_favorite: number;
            is_watched: number;
            poster_path: string | null;
            synopsis: string | null;
            metadata_fetched_at: string | null;
          }
        | undefined;

      if (!row) {
        return null as unknown as T;
      }

      const media = {
        ...row,
        is_favorite: Boolean(row.is_favorite),
        is_watched: Boolean(row.is_watched),
      } as T;

      const files = db
        .prepare(
          `SELECT id, media_id, season, episode, title, file_path, size_bytes,
                  mtime_ms, indexed_at
           FROM media_files WHERE media_id = ? ORDER BY season, episode, file_path`
        )
        .all(mediaId) as T[];

      return { media, files } as unknown as T;
    }

    case 'toggle_favorite_cmd': {
      const mediaId = args.media_id as number;
      db.prepare(
        "UPDATE media SET is_favorite = NOT is_favorite, updated_at = datetime('now') WHERE id = ?"
      ).run(mediaId);
      const row = db.prepare('SELECT is_favorite FROM media WHERE id = ?').get(mediaId) as {
        is_favorite: number;
      };
      return Boolean(row.is_favorite) as unknown as T;
    }

    case 'toggle_watched_cmd': {
      const mediaId = args.media_id as number;
      db.prepare(
        "UPDATE media SET is_watched = NOT is_watched, updated_at = datetime('now') WHERE id = ?"
      ).run(mediaId);
      const row = db.prepare('SELECT is_watched FROM media WHERE id = ?').get(mediaId) as {
        is_watched: number;
      };
      return Boolean(row.is_watched) as unknown as T;
    }

    case 'player_open_cmd':
    case 'player_play_cmd':
    case 'player_pause_cmd':
    case 'player_stop_cmd':
    case 'player_seek_cmd':
    case 'player_volume_cmd':
      return undefined as unknown as T;

    case 'player_status_cmd':
      return {
        timePosition: 0,
        duration: 0,
        paused: true,
        volume: 70,
      } as unknown as T;

    default:
      throw new Error(`Comando Tauri desconocido en mock: ${cmd}`);
  }
}

export { mockInvoke as invoke };
export default mockInvoke;
