import { invoke } from '@tauri-apps/api/core';
import type { Media } from '../types/media';

const DEFAULT_DB_PATH = 'media-manager.db';

/**
 * Lista medios desde la base de datos SQLite a traves del comando Tauri
 * `list_media_cmd`. `db_path` es la ruta al archivo .db; `query` filtra por
 * titulo (opcional, busca LIKE).
 */
export async function listMedia(
  dbPath: string = DEFAULT_DB_PATH,
  query?: string,
): Promise<Media[]> {
  return invoke<Media[]>('list_media_cmd', { db_path: dbPath, query });
}
