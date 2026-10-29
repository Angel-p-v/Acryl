import type { Db } from '../connection.js';
import type { IndexRunRow } from '../types.js';

type RunDbRow = {
  id: number;
  started_at: string;
  finished_at: string | null;
  files_seen: number;
  files_added: number;
  files_removed: number;
  status: 'running' | 'completed' | 'failed';
};

export type IndexRunStats = {
  filesSeen: number;
  filesAdded: number;
  filesRemoved: number;
};

export type IndexRunRepository = {
  start(): number;
  complete(id: number, stats: IndexRunStats): void;
  fail(id: number, stats?: Partial<IndexRunStats>): void;
  latest(): IndexRunRow | undefined;
  list(limit?: number): IndexRunRow[];
};

/** Crea el repositorio de ejecuciones de indexación. */
export function createIndexRunRepository(db: Db): IndexRunRepository {
  const insertStatement = db.prepare('INSERT INTO index_runs DEFAULT VALUES');
  const completeStatement = db.prepare(`
    UPDATE index_runs
    SET finished_at = datetime('now'), files_seen = ?, files_added = ?, files_removed = ?, status = 'completed'
    WHERE id = ?
  `);
  const failStatement = db.prepare(`
    UPDATE index_runs
    SET finished_at = datetime('now'), files_seen = ?, files_added = ?, files_removed = ?, status = 'failed'
    WHERE id = ?
  `);
  const latestStatement = db.prepare('SELECT * FROM index_runs ORDER BY id DESC LIMIT 1');
  const listStatement = db.prepare<[number], RunDbRow>(
    'SELECT * FROM index_runs ORDER BY id DESC LIMIT ?',
  );

  return {
    start(): number {
      const result = insertStatement.run();
      return Number(result.lastInsertRowid);
    },

    complete(id: number, stats: IndexRunStats): void {
      completeStatement.run(stats.filesSeen, stats.filesAdded, stats.filesRemoved, id);
    },

    fail(id: number, stats: Partial<IndexRunStats> = {}): void {
      failStatement.run(stats.filesSeen ?? 0, stats.filesAdded ?? 0, stats.filesRemoved ?? 0, id);
    },

    latest(): IndexRunRow | undefined {
      const row = latestStatement.get() as RunDbRow | undefined;
      return row ? toRunRow(row) : undefined;
    },

    list(limit = 10): IndexRunRow[] {
      return listStatement.all(limit).map(toRunRow);
    },
  };
}

/** Convierte una fila de SQLite al tipo del dominio. */
function toRunRow(row: RunDbRow): IndexRunRow {
  return {
    id: row.id,
    startedAt: row.started_at,
    finishedAt: row.finished_at,
    filesSeen: row.files_seen,
    filesAdded: row.files_added,
    filesRemoved: row.files_removed,
    status: row.status,
  };
}
