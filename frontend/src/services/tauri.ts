import { invoke } from '@tauri-apps/api/core';

import type { Media } from '../types/media';

const DEFAULT_DB_PATH = 'media-manager.db';

/**
 * Lista medios desde la base de datos SQLite a través del comando Tauri
 * `list_media_cmd`. Soporta filtros de tipo, paginación y orden.
 */
export async function listMedia(
  dbPath: string = DEFAULT_DB_PATH,
  opts: {
    readonly type?: string;
    readonly limit?: number;
    readonly offset?: number;
    readonly orderBy?: 'title' | 'added' | 'recent';
  } = {}
): Promise<{ readonly media: readonly Media[]; readonly total: number }> {
  const result = await invoke<{ media: Media[]; total: number }>('list_media_cmd', {
    db_path: dbPath,
    type: opts.type,
    limit: opts.limit,
    offset: opts.offset,
    order_by: opts.orderBy,
  });
  return result;
}

/**
 * Busca medios por texto (FTS5 con fallback LIKE) a través del comando
 * Tauri `search_media_cmd`.
 */
export async function searchMedia(
  dbPath: string = DEFAULT_DB_PATH,
  query: string,
  opts: {
    readonly type?: string;
    readonly limit?: number;
  } = {}
): Promise<readonly Media[]> {
  const result = await invoke<Media[]>('search_media_cmd', {
    db_path: dbPath,
    query,
    type: opts.type,
    limit: opts.limit,
  });
  return result;
}
