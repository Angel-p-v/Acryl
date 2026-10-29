import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { afterAll, beforeAll, test } from 'vitest';

import { BENCH_LIBRARY_FILES, createBenchLibrary } from './fixtures.js';
import { scanLibrary } from '../../src/core/indexer/scanner.js';

let libraryRoot = '';

beforeAll(async () => {
  libraryRoot = await mkdtemp(path.join(tmpdir(), 'mm-bench-scan-'));
  await createBenchLibrary(libraryRoot);
}, 120_000);

afterAll(async () => {
  if (libraryRoot !== '') {
    await rm(libraryRoot, { recursive: true, force: true });
  }
});

test('escaneo de 10 000 ficheros', async ({ bench }) => {
  const scan = bench('scanLibrary · 10 000 ficheros', async () => {
    const files = await scanLibrary(libraryRoot, {
      maxDepth: 4,
      followSymlinks: false,
      minSizeBytes: 0,
      exclude: [],
    });

    if (files.length !== BENCH_LIBRARY_FILES) {
      throw new Error(`Se esperaban ${BENCH_LIBRARY_FILES} ficheros y llegaron ${files.length}`);
    }
  });

  await scan.run({ iterations: 2, time: 5_000, warmupIterations: 1, warmupTime: 1_000 });
}, 120_000);
