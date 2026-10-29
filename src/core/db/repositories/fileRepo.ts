import type { Db } from '../connection.js';
import type { MediaFileInput, MediaFileRow } from '../types.js';

type FileDbRow = {
  id: number;
  media_id: number;
  season: number | null;
  episode: number | null;
  title: string | null;
  file_path: string;
  size_bytes: number | null;
  mtime_ms: number | null;
  indexed_at: string;
};

export type FileRepository = {
  upsertMany(files: MediaFileInput[]): number;
  findByPath(filePath: string): MediaFileRow | undefined;
  listByMedia(mediaId: number): MediaFileRow[];
  listAllPaths(): string[];
  removeByMedia(mediaId: number): number;
  removeMissing(knownPaths: ReadonlySet<string>): number;
  /** Actualiza la ruta de un fichero (rename/move). */
  updateFilePath(oldPath: string, newPath: string): boolean;
  count(mediaId?: number): number;
};

/** Crea el repositorio de ficheros sobre una conexión SQLite dada. */
export function createFileRepository(db: Db): FileRepository {
  const upsertStatement = db.prepare(`
    INSERT INTO media_files (media_id, season, episode, title, file_path, size_bytes, mtime_ms)
    VALUES (@media_id, @season, @episode, @title, @file_path, @size_bytes, @mtime_ms)
    ON CONFLICT(file_path) DO UPDATE SET
      media_id = excluded.media_id,
      season = excluded.season,
      episode = excluded.episode,
      title = excluded.title,
      size_bytes = excluded.size_bytes,
      mtime_ms = excluded.mtime_ms,
      indexed_at = datetime('now')
  `);

  const findByPathStatement = db.prepare('SELECT * FROM media_files WHERE file_path = ?');
  const listByMediaStatement = db.prepare<[number], FileDbRow>(
    'SELECT * FROM media_files WHERE media_id = ? ORDER BY season, episode, file_path',
  );
  const listAllPathsStatement = db.prepare<[], { file_path: string }>(
    'SELECT file_path FROM media_files',
  );
  const removeByMediaStatement = db.prepare('DELETE FROM media_files WHERE media_id = ?');
  const countAllStatement = db.prepare<[], { total: number }>(
    'SELECT COUNT(*) AS total FROM media_files',
  );
  const countByMediaStatement = db.prepare<[number], { total: number }>(
    'SELECT COUNT(*) AS total FROM media_files WHERE media_id = ?',
  );

  const updateFilePathStatement = db.prepare(
    "UPDATE media_files SET file_path = @newPath, indexed_at = datetime('now') WHERE file_path = @oldPath",
  );

  const upsertManyTx = db.transaction((files: MediaFileInput[]): number => {
    for (const file of files) {
      upsertStatement.run({
        media_id: file.mediaId,
        season: file.season ?? null,
        episode: file.episode ?? null,
        title: file.title ?? null,
        file_path: file.filePath,
        size_bytes: file.sizeBytes ?? null,
        mtime_ms: file.mtimeMs ?? null,
      });
    }
    return files.length;
  });

  return {
    upsertMany(files: MediaFileInput[]): number {
      if (files.length === 0) return 0;
      return upsertManyTx(files);
    },

    findByPath(filePath: string): MediaFileRow | undefined {
      const row = findByPathStatement.get(filePath) as FileDbRow | undefined;
      return row ? toFileRow(row) : undefined;
    },

    listByMedia(mediaId: number): MediaFileRow[] {
      return listByMediaStatement.all(mediaId).map(toFileRow);
    },

    listAllPaths(): string[] {
      return listAllPathsStatement.all().map((row) => row.file_path);
    },

    removeByMedia(mediaId: number): number {
      return removeByMediaStatement.run(mediaId).changes;
    },

    /**
     * Elimina los ficheros que ya no están en disco y devuelve cuántos se han borrado.
     * Se usa tras una indexación completa para limpiar la biblioteca.
     */
    removeMissing(knownPaths: ReadonlySet<string>): number {
      const orphans = listAllPathsStatement.all().filter((row) => !knownPaths.has(row.file_path));
      if (orphans.length === 0) return 0;

      const removeStatement = db.prepare('DELETE FROM media_files WHERE file_path = ?');
      const removeTx = db.transaction((paths: string[]): number => {
        for (const filePath of paths) removeStatement.run(filePath);
        return paths.length;
      });
      return removeTx(orphans.map((row) => row.file_path));
    },

    /** Actualiza la ruta de un fichero (rename/move). Devuelve true si se actualizó. */
    updateFilePath(oldPath: string, newPath: string): boolean {
      return updateFilePathStatement.run({ oldPath, newPath }).changes > 0;
    },

    count(mediaId?: number): number {
      if (mediaId === undefined) return countAllStatement.get()?.total ?? 0;
      return countByMediaStatement.get(mediaId)?.total ?? 0;
    },
  };
}

/** Convierte una fila de SQLite al tipo del dominio. */
function toFileRow(row: FileDbRow): MediaFileRow {
  return {
    id: row.id,
    mediaId: row.media_id,
    season: row.season,
    episode: row.episode,
    title: row.title,
    filePath: row.file_path,
    sizeBytes: row.size_bytes,
    mtimeMs: row.mtime_ms,
    indexedAt: row.indexed_at,
  };
}
