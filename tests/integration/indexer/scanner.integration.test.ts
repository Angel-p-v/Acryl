import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import { scanLibrary } from '../../../src/core/indexer/scanner.js';
import { IndexError } from '../../../src/shared/errors.js';

const FIXTURES = fileURLToPath(new URL('../../fixtures/library', import.meta.url));
const SERIES_ROOT = path.join(FIXTURES, 'series');

const options = {
  maxDepth: 4,
  followSymlinks: false,
  minSizeBytes: 0,
  exclude: ['**/*.sample.*', '**/sample/**', '**/.hidden/**', '**/@eaDir/**'],
};

describe('scanLibrary', () => {
  it('encuentra los vídeos y descarta muestras, ficheros ocultos y subtítulos', async () => {
    const files = await scanLibrary(SERIES_ROOT, options);

    expect(files).toHaveLength(4);
    expect(files.map((file) => path.basename(file.filePath))).toEqual([
      'Dark.2x01.mkv',
      'Dark.2x02.mkv',
      'Severance.S01E01.1080p.mkv',
      'Severance.S01E02.1080p.mkv',
    ]);
    for (const file of files) {
      expect(file.sizeBytes).toBeGreaterThan(0);
      expect(file.mtimeMs).toBeGreaterThan(0);
      expect(path.isAbsolute(file.filePath)).toBe(true);
    }
  });

  it('respeta maxDepth', async () => {
    const files = await scanLibrary(SERIES_ROOT, { ...options, maxDepth: 1 });
    expect(files).toHaveLength(0);
  });

  it('descarta ficheros por debajo de minSizeBytes', async () => {
    const files = await scanLibrary(SERIES_ROOT, { ...options, minSizeBytes: 1_000_000 });
    expect(files).toHaveLength(0);
  });

  it('falla con un mensaje útil si la raíz no existe', async () => {
    await expect(scanLibrary(path.join(FIXTURES, 'no-existe'), options)).rejects.toThrow(
      IndexError,
    );
  });

  it('falla si la raíz es un fichero', async () => {
    await expect(scanLibrary(path.join(FIXTURES, 'movies', 'notes.txt'), options)).rejects.toThrow(
      /no es un directorio/,
    );
  });
});
