import type { MediaType } from '../config/schema.js';

/** Fila de la tabla `media`. */
export type MediaRow = {
  id: number;
  type: MediaType;
  title: string;
  sortTitle: string;
  year: number | null;
  folderPath: string;
  createdAt: string;
  updatedAt: string;
};

/** Fila de la tabla `media_files`. */
export type MediaFileRow = {
  id: number;
  mediaId: number;
  season: number | null;
  episode: number | null;
  title: string | null;
  filePath: string;
  sizeBytes: number | null;
  mtimeMs: number | null;
  indexedAt: string;
};

/** Datos necesarios para crear o actualizar un título. */
export type MediaInput = {
  type: MediaType;
  title: string;
  year?: number | null;
  folderPath: string;
};

/** Datos necesarios para crear o actualizar un fichero. */
export type MediaFileInput = {
  mediaId: number;
  season?: number | null;
  episode?: number | null;
  title?: string | null;
  filePath: string;
  sizeBytes?: number | null;
  mtimeMs?: number | null;
};

/** Resumen de una ejecución de indexación. */
export type IndexRunRow = {
  id: number;
  startedAt: string;
  finishedAt: string | null;
  filesSeen: number;
  filesAdded: number;
  filesRemoved: number;
  status: 'running' | 'completed' | 'failed';
};

/** Fila de la tabla `playback_state`. */
export type PlaybackStateRow = {
  mediaFileId: number;
  positionSeconds: number;
  durationSeconds: number | null;
  completedAt: string | null;
  lastPlayedAt: string;
  playCount: number;
  updatedAt: string;
};

/**
 * Datos de un estado de reproducción. El repositorio no calcula reglas de negocio:
 * recibe la posición, la finalización y las fechas ya resueltas por el dominio.
 */
export type PlaybackInput = {
  mediaFileId: number;
  positionSeconds: number;
  durationSeconds?: number | null;
  completedAt?: string | null;
  lastPlayedAt: string;
  playCount?: number;
  updatedAt: string;
};

/** Estado de reproducción con el título y el fichero a los que pertenece. */
export type PlaybackEntryRow = {
  state: PlaybackStateRow;
  media: MediaRow;
  file: MediaFileRow;
};

/** Fila de la tabla `media_preferences`. */
export type MediaPreferenceRow = {
  mediaId: number;
  favorite: boolean;
  createdAt: string;
  updatedAt: string;
};
