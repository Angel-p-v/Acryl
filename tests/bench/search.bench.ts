import { afterAll, beforeAll, test } from 'vitest';

import { closeDatabase, openDatabase, type Db } from '../../src/core/db/connection.js';
import {
  createMediaRepository,
  type MediaRepository,
} from '../../src/core/db/repositories/mediaRepo.js';

const STRESS_ENTRIES = 100_000;

let db: Db;
let mediaRepo: MediaRepository;

beforeAll(() => {
  db = openDatabase(':memory:');
  const insert = db.prepare(`
    INSERT INTO media (type, title, sort_title, year, folder_path)
    VALUES (@type, @title, @sort_title, @year, @folder_path)
  `);

  db.transaction(() => {
    for (let index = 0; index < STRESS_ENTRIES; index += 1) {
      const title = `Bench Title ${index}`;
      insert.run({
        type: 'series',
        title,
        sort_title: title.toLocaleLowerCase(),
        year: 2020,
        folder_path: `/library/bench-${index}`,
      });
    }
  })();

  db.pragma('optimize');
  mediaRepo = createMediaRepository(db);
}, 120_000);

afterAll(() => {
  closeDatabase(db);
});

test('búsqueda FTS y listado sobre 100 000 títulos', async ({ bench }) => {
  const search = bench('search · FTS sobre 100 000 títulos', () => {
    const results = mediaRepo.search('99999');
    if (results.length !== 1 || results[0]?.title !== 'Bench Title 99999') {
      throw new Error('La búsqueda FTS no devolvió el título esperado');
    }
  });
  await search.run({ iterations: 50, time: 2_000, warmupIterations: 5, warmupTime: 500 });

  const list = bench('list · primera página de 50 títulos', () => {
    const results = mediaRepo.list({ type: 'series', limit: 50, orderBy: 'title' });
    if (results.length !== 50) {
      throw new Error(`Se esperaban 50 títulos y llegaron ${results.length}`);
    }
  });
  await list.run({ iterations: 50, time: 2_000, warmupIterations: 5, warmupTime: 500 });
}, 120_000);
