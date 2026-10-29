import { DbError } from '../../../shared/errors.js';

import type { MediaType } from '../../config/schema.js';
import type { Db } from '../connection.js';
import type { PlaybackEntryRow, PlaybackInput, PlaybackStateRow } from '../types.js';

export type { PlaybackEntryRow, PlaybackStateRow, PlaybackInput } from '../types.js';
export type { PlaybackConfig } from '../../config/schema.js';

type PlaybackDbRow = {
  media_file_id: number;
  position_seconds: number;
  duration_seconds: number | null;
  completed_at: string | null;
  last_played_at: string;
  play_count: number;
  updated_at: string;
};

type PlaybackJoinDbRow = PlaybackDbRow & {
  media_id: number;
  media_type: MediaType;
  media_title: string;
  media_sort_title: string;
  media_year: number | null;
  media_folder_path: string;
  media_created_at: string;
  media_updated_at: string;
  file_id: number;
  file_media_id: number;
  file_path: string;
  file_season: number | null;
  file_episode: number | null;
  file_title: string | null;
  file_size_bytes: number | null;
  file_mtime_ms: number | null;
  file_indexed_at: string;
};

export type PlaybackRepository = {
  /** Guarda o actualiza el estado de un fichero y devuelve la fila resultante. */
  save(input: PlaybackInput): PlaybackStateRow;
  get(mediaFileId: number): PlaybackStateRow | undefined;
  remove(mediaFileId: number): boolean;
  /** Títulos empezados y no completados, del más reciente al más antiguo. */
  listInProgress(limit?: number): PlaybackEntryRow[];
  /** Historial reciente, completados incluidos. */
  listRecent(limit?: number): PlaybackEntryRow[];
  count(): number;
};

const entryColumns = `
  s.media_file_id, s.position_seconds, s.duration_seconds, s.completed_at,
  s.last_played_at, s.play_count, s.updated_at,
  m.id AS media_id, m.type AS media_type, m.title AS media_title,
  m.sort_title AS media_sort_title, m.year AS media_year,
  m.folder_path AS media_folder_path,
  m.created_at AS media_created_at, m.updated_at AS media_updated_at,
  f.id AS file_id, f.media_id AS file_media_id, f.file_path AS file_path,
  f.season AS file_season, f.episode AS file_episode, f.title AS file_title,
  f.size_bytes AS file_size_bytes, f.mtime_ms AS file_mtime_ms, f.indexed_at AS file_indexed_at
`;

/** Crea el repositorio del estado de reproducción sobre una conexión SQLite dada. */
export function createPlaybackRepository(db: Db): PlaybackRepository {
  const saveStatement = db.prepare(`
    INSERT INTO playback_state (
      media_file_id, position_seconds, duration_seconds, completed_at,
      last_played_at, play_count, updated_at
    )
    VALUES (
      @media_file_id, @position_seconds, @duration_seconds, @completed_at,
      @last_played_at, @play_count, @updated_at
    )
    ON CONFLICT(media_file_id) DO UPDATE SET
      position_seconds = excluded.position_seconds,
      duration_seconds = excluded.duration_seconds,
      completed_at = excluded.completed_at,
      last_played_at = excluded.last_played_at,
      play_count = excluded.play_count,
      updated_at = excluded.updated_at
    RETURNING *
  `);

  const getStatement = db.prepare('SELECT * FROM playback_state WHERE media_file_id = ?');
  const removeStatement = db.prepare('DELETE FROM playback_state WHERE media_file_id = ?');
  const countStatement = db.prepare<[], { total: number }>(
    'SELECT COUNT(*) AS total FROM playback_state',
  );

  const entryQuery = (where: string): string => `
    SELECT ${entryColumns}
    FROM playback_state s
    JOIN media_files f ON f.id = s.media_file_id
    JOIN media m ON m.id = f.media_id
    ${where}
    ORDER BY s.last_played_at DESC, s.media_file_id DESC
    LIMIT ?
  `;

  const listInProgressStatement = db.prepare(
    entryQuery('WHERE s.completed_at IS NULL AND s.position_seconds > 0'),
  );
  const listRecentStatement = db.prepare(entryQuery(''));

  return {
    save(input: PlaybackInput): PlaybackStateRow {
      const row = saveStatement.get({
        media_file_id: input.mediaFileId,
        position_seconds: input.positionSeconds,
        duration_seconds: input.durationSeconds ?? null,
        completed_at: input.completedAt ?? null,
        last_played_at: input.lastPlayedAt,
        play_count: input.playCount ?? 0,
        updated_at: input.updatedAt,
      }) as PlaybackDbRow | undefined;

      if (!row) throw new DbError('No se pudo guardar el estado de reproducción');
      return toPlaybackState(row);
    },

    get(mediaFileId: number): PlaybackStateRow | undefined {
      const row = getStatement.get(mediaFileId) as PlaybackDbRow | undefined;
      return row ? toPlaybackState(row) : undefined;
    },

    remove(mediaFileId: number): boolean {
      return removeStatement.run(mediaFileId).changes > 0;
    },

    listInProgress(limit = 50): PlaybackEntryRow[] {
      return (listInProgressStatement.all(limit) as PlaybackJoinDbRow[]).map(toPlaybackEntry);
    },

    listRecent(limit = 50): PlaybackEntryRow[] {
      return (listRecentStatement.all(limit) as PlaybackJoinDbRow[]).map(toPlaybackEntry);
    },

    count(): number {
      return countStatement.get()?.total ?? 0;
    },
  };
}

/** Convierte una fila de SQLite al tipo del dominio. */
function toPlaybackState(row: PlaybackDbRow): PlaybackStateRow {
  return {
    mediaFileId: row.media_file_id,
    positionSeconds: row.position_seconds,
    durationSeconds: row.duration_seconds,
    completedAt: row.completed_at,
    lastPlayedAt: row.last_played_at,
    playCount: row.play_count,
    updatedAt: row.updated_at,
  };
}

function toPlaybackEntry(row: PlaybackJoinDbRow): PlaybackEntryRow {
  return {
    state: toPlaybackState(row),
    media: {
      id: row.media_id,
      type: row.media_type,
      title: row.media_title,
      sortTitle: row.media_sort_title,
      year: row.media_year,
      folderPath: row.media_folder_path,
      createdAt: row.media_created_at,
      updatedAt: row.media_updated_at,
    },
    file: {
      id: row.file_id,
      mediaId: row.file_media_id,
      filePath: row.file_path,
      season: row.file_season,
      episode: row.file_episode,
      title: row.file_title,
      sizeBytes: row.file_size_bytes,
      mtimeMs: row.file_mtime_ms,
      indexedAt: row.file_indexed_at,
    },
  };
}
