import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { afterAll, beforeAll, test } from 'vitest';

import { BENCH_LIBRARY_FILES, createBenchLibrary } from './fixtures.js';
import { indexLibrary, type IndexLibraryConfig } from '../../src/core/indexer/service.js';
import { openLibrary } from '../../src/core/library/open.js';

let libraryRoot = '';
let libraryConfig: IndexLibraryConfig;

beforeAll(async () => {
  libraryRoot = await mkdtemp(path.join(tmpdir(), 'mm-bench-index-'));
  await createBenchLibrary(libraryRoot);
  libraryConfig = {
    roots: [{ path: libraryRoot, type: 'series' }],
    exclude: [],
    scan: { maxDepth: 4, followSymlinks: false, minSizeBytes: 0 },
  };
}, 120_000);

afterAll(async () => {
  if (libraryRoot !== '') {
    await rm(libraryRoot, { recursive: true, force: true });
  }
});

test('indexado completo de 10 000 ficheros', async ({ bench }) => {
  const index = bench('indexLibrary · 10 000 ficheros', async () => {
    const opened = openLibrary(':memory:');
    try {
      const stats = await indexLibrary(opened.repositories, libraryConfig);
      if (stats.filesAdded !== BENCH_LIBRARY_FILES) {
        throw new Error(
          `Se esperaban ${BENCH_LIBRARY_FILES} ficheros indexados y llegaron ${stats.filesAdded}`,
        );
      }
    } finally {
      opened.close();
    }
  });

  await index.run({ iterations: 2, time: 5_000, warmupIterations: 1, warmupTime: 1_000 });
}, 120_000);
