import { mkdir, mkdtemp, rm, unlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { watchLibrary, type LibraryWatcher } from '../../../src/core/indexer/watcher.js';
import { openLibrary, type OpenedLibrary } from '../../../src/core/library/open.js';

import type { IndexStats } from '../../../src/core/indexer/service.js';

describe('watchLibrary (integración)', () => {
  let workDir: string;
  let root: string;
  let opened: OpenedLibrary;
  let watcher: LibraryWatcher | undefined;

  beforeEach(async () => {
    workDir = await mkdtemp(path.join(tmpdir(), 'mm-watch-'));
    root = path.join(workDir, 'series');
    await mkdir(root, { recursive: true });
    opened = openLibrary(path.join(workDir, 'library.db'));
  });

  afterEach(async () => {
    await watcher?.close();
    opened.close();
    await rm(workDir, { recursive: true, force: true });
  });

  function startWatcher(): { nextIndex: () => Promise<IndexStats> } {
    const events: IndexStats[] = [];
    const waiters: ((stats: IndexStats) => void)[] = [];

    const errors: unknown[] = [];
    watcher = watchLibrary({
      repositories: opened.repositories,
      library: {
        roots: [{ path: root, type: 'series' }],
        exclude: [],
        scan: { followSymlinks: false, maxDepth: 4, minSizeBytes: 0 },
      },
      debounceMs: 50,
      onIndexed: (stats) => {
        const waiter = waiters.shift();
        if (waiter === undefined) events.push(stats);
        else waiter(stats);
      },
      onError: (error) => {
        errors.push(error);
      },
    });

    return {
      nextIndex: () =>
        new Promise<IndexStats>((resolve, reject) => {
          const existing = events.shift();
          if (existing !== undefined) {
            resolve(existing);
            return;
          }

          const timer = setTimeout(() => {
            reject(
              new Error(
                `No llegó ninguna indexación en 10 s. Errores: ${
                  errors.map(String).join('; ') || 'ninguno'
                }`,
              ),
            );
          }, 10_000);

          waiters.push((stats) => {
            clearTimeout(timer);
            resolve(stats);
          });
        }),
    };
  }

  it('reindexa al añadir un vídeo y poda al eliminarlo', async () => {
    const folder = path.join(root, 'Show (2022)');
    await mkdir(folder, { recursive: true });

    const { nextIndex } = startWatcher();
    await watcher?.ready;

    const filePath = path.join(folder, 'Show.S01E01.mkv');
    await writeFile(filePath, 'video', 'utf8');

    await nextIndex();
    expect(opened.library.count()).toBe(1);
    expect(opened.library.find('Show').files).toHaveLength(1);

    await unlink(filePath);
    await nextIndex();

    expect(opened.library.count()).toBe(0);
  }, 20_000);
});
