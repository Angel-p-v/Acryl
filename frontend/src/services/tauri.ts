import { invoke } from '@tauri-apps/api/core';

import type { Media, MediaDetails, PlayerStatus } from '../types/media';

const DEFAULT_DB_PATH = 'acryl.db';

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

/**
 * Obtiene el detalle completo de un medio (título + archivos indexados)
 * mediante `get_media_details_cmd`.
 */
export async function getMediaDetails(
  dbPath: string = DEFAULT_DB_PATH,
  mediaId: number
): Promise<MediaDetails | null> {
  return await invoke<MediaDetails | null>('get_media_details_cmd', {
    db_path: dbPath,
    media_id: mediaId,
  });
}

/**
 * Invierte el estado de favorito de un medio mediante
 * `toggle_favorite_cmd`. Devuelve el nuevo valor.
 */
export async function toggleFavorite(
  dbPath: string = DEFAULT_DB_PATH,
  mediaId: number
): Promise<boolean> {
  return await invoke<boolean>('toggle_favorite_cmd', {
    db_path: dbPath,
    media_id: mediaId,
  });
}

/**
 * Invierte el estado de visto de un medio mediante
 * `toggle_watched_cmd`. Devuelve el nuevo valor.
 */
export async function toggleWatched(
  dbPath: string = DEFAULT_DB_PATH,
  mediaId: number
): Promise<boolean> {
  return await invoke<boolean>('toggle_watched_cmd', {
    db_path: dbPath,
    media_id: mediaId,
  });
}

/**
 * Sincroniza los metadatos de un medio desde TMDB/TVDB mediante
 * `sync_metadata_cmd`. Devuelve los metadatos obtenidos.
 */
export interface SyncMetadataResult {
  readonly title?: string;
  readonly synopsis?: string;
  readonly poster_path?: string;
  readonly release_date?: string;
  readonly vote_average?: number;
}

export async function syncMetadata(
  dbPath: string = DEFAULT_DB_PATH,
  mediaId: number
): Promise<SyncMetadataResult> {
  return await invoke<SyncMetadataResult>('sync_metadata_cmd', {
    db_path: dbPath,
    media_id: mediaId,
  });
}

/**
 * Registra una reproducción en el historial mediante
 * `record_playback_cmd`.
 */
export async function recordPlayback(
  dbPath: string = DEFAULT_DB_PATH,
  mediaId: number,
  position: number,
  duration: number
): Promise<void> {
  await invoke('record_playback_cmd', {
    db_path: dbPath,
    media_id: mediaId,
    position,
    duration,
  });
}

/**
 * Obtiene el historial de reproducción mediante `get_playback_history_cmd`.
 */
export interface PlaybackRecord {
  readonly id: number;
  readonly media_id: number;
  readonly title: string;
  readonly played_at: string;
  readonly position: number;
  readonly duration: number;
}

export async function getPlaybackHistory(
  dbPath: string = DEFAULT_DB_PATH,
  limit?: number
): Promise<readonly PlaybackRecord[]> {
  return await invoke<PlaybackRecord[]>('get_playback_history_cmd', {
    db_path: dbPath,
    limit,
  });
}
