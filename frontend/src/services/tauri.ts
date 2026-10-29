import { invoke } from '@tauri-apps/api/core';

import type { Media, PlayerStatus } from '../types/media';

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
  } = {},
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
  } = {},
): Promise<readonly Media[]> {
  const result = await invoke<Media[]>('search_media_cmd', {
    db_path: dbPath,
    query,
    type: opts.type,
    limit: opts.limit,
  });
  return result;
}

/**
 * Crea el reproductor (si no existe) y carga un archivo de vídeo a través
 * del comando Tauri `player_open_cmd`.
 */
export async function playerOpen(filePath: string): Promise<void> {
  await invoke('player_open_cmd', { file_path: filePath });
}

/**
 * Reanuda o inicia la reproducción mediante `player_play_cmd`.
 */
export async function playerPlay(): Promise<void> {
  await invoke('player_play_cmd');
}

/**
 * Pausa la reproducción mediante `player_pause_cmd`.
 */
export async function playerPause(): Promise<void> {
  await invoke('player_pause_cmd');
}

/**
 * Detiene la reproducción y cierra el archivo actual mediante
 * `player_stop_cmd`.
 */
export async function playerStop(): Promise<void> {
  await invoke('player_stop_cmd');
}

/**
 * Busca a una posición (segundos) mediante `player_seek_cmd`.
 */
export async function playerSeek(seconds: number): Promise<void> {
  await invoke('player_seek_cmd', { seconds });
}

/**
 * Establece el volumen (0.0 a 100.0) mediante `player_volume_cmd`.
 */
export async function playerVolume(volume: number): Promise<void> {
  await invoke('player_volume_cmd', { volume });
}

/**
 * Obtiene el estado actual del reproductor mediante `player_status_cmd`.
 */
export async function playerStatus(): Promise<PlayerStatus> {
  return await invoke<PlayerStatus>('player_status_cmd');
}
