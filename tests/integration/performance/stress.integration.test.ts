import os from 'node:os';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import { closeDatabase, openDatabase, type Db } from '../../../src/core/db/connection.js';
import {
  createMediaRepository,
  type MediaRepository,
} from '../../../src/core/db/repositories/mediaRepo.js';

const STRESS_ENTRIES = 100_000;
// ARM64 es inherentemente más lento para escrituras SQLite que x64.
// 60 s da margen en runners ARM64 de bajo rendimiento.
const SEED_BUDGET_MS = os.arch() === 'arm64' ? 60_000 : 15_000;
const QUERY_BUDGET_MS = 500;

let db: Db;
let mediaRepo: MediaRepository;
let seedDurationMs = 0;

beforeAll(() => {
  db = openDatabase(':memory:');
  const insert = db.prepare(`
    INSERT INTO media (type, title, sort_title, year, folder_path)
    VALUES (@type, @title, @sort_title, @year, @folder_path)
  `);

  const startedAt = performance.now();
  db.transaction(() => {
    for (let index = 0; index < STRESS_ENTRIES; index += 1) {
      const title = `Stress Title ${index}`;
      insert.run({
        type: 'series',
        title,
        sort_title: title.toLocaleLowerCase(),
        year: 2020,
        folder_path: `/library/stress-${index}`,
      });
    }
  })();
  seedDurationMs = performance.now() - startedAt;

  db.pragma('optimize');
  mediaRepo = createMediaRepository(db);
}, 120_000);

afterAll(() => {
  closeDatabase(db);
});

describe('estrés con 100 000 entradas (integración)', () => {
  it('siembra el índice dentro del presupuesto', () => {
    expect(mediaRepo.count()).toBe(STRESS_ENTRIES);
    expect(seedDurationMs).toBeLessThan(SEED_BUDGET_MS);
  });

  it('resuelve una búsqueda FTS dentro del presupuesto', () => {
    const startedAt = performance.now();
    const results = mediaRepo.search('99999');
    const durationMs = performance.now() - startedAt;

    expect(results).toHaveLength(1);
    expect(results[0]?.title).toBe('Stress Title 99999');
    expect(durationMs).toBeLessThan(QUERY_BUDGET_MS);
  });

  it('lista la primera página dentro del presupuesto', () => {
    const startedAt = performance.now();
    const results = mediaRepo.list({ type: 'series', limit: 50, orderBy: 'title' });
    const durationMs = performance.now() - startedAt;

    expect(results).toHaveLength(50);
    expect(durationMs).toBeLessThan(QUERY_BUDGET_MS);
  });
});
