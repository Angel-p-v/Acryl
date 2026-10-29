import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { closeDatabase, openDatabase, type Db } from '../../../src/core/db/connection.js';
import { createMediaRepository } from '../../../src/core/db/repositories/mediaRepo.js';

type PlanRow = { detail: string };

describe('plan de búsqueda FTS5 (integración)', () => {
  let workDir: string;
  let db: Db;

  beforeEach(async () => {
    workDir = await mkdtemp(path.join(tmpdir(), 'mm-plan-'));
    db = openDatabase(path.join(workDir, 'library.db'));
    createMediaRepository(db).upsert({
      type: 'series',
      title: 'Severance',
      folderPath: '/media/series/Severance',
    });
  });

  afterEach(async () => {
    closeDatabase(db);
    await rm(workDir, { recursive: true, force: true });
  });

  it('la búsqueda FTS5 usa el índice virtual y no un escaneo completo', () => {
    const plan = db
      .prepare(
        `EXPLAIN QUERY PLAN
         SELECT m.* FROM media m
         JOIN media_fts f ON f.rowid = m.id
         WHERE media_fts MATCH ?`,
      )
      .all('sever') as PlanRow[];

    const details = plan.map((row) => row.detail).join('\n');
    expect(details).toContain('VIRTUAL TABLE INDEX');
  });

  it('un LIKE equivalente obliga a escanear la tabla completa', () => {
    const plan = db
      .prepare("EXPLAIN QUERY PLAN SELECT * FROM media WHERE title LIKE '%sever%'")
      .all() as PlanRow[];

    const details = plan.map((row) => row.detail).join('\n');
    expect(details).toContain('SCAN media');
    expect(details).not.toContain('VIRTUAL TABLE INDEX');
  });
});
