import { mkdtemp, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { describe, expect, it } from 'vitest';

import { loadConfig } from '../../../src/core/config/loader.js';
import { ConfigError } from '../../../src/shared/errors.js';

describe('loadConfig (integración)', () => {
  it('carga un archivo real desde el disco y normaliza las rutas', async () => {
    const dir = await mkdtemp(path.join(tmpdir(), 'mm-int-'));
    const configFile = path.join(dir, 'config.yaml');
    await writeFile(
      configFile,
      [
        'version: 1',
        'library:',
        '  roots:',
        '    - path: ./anime',
        '      type: anime',
        'database:',
        '  path: ./data/library.db',
        'player:',
        '  default: mpv',
        '',
      ].join('\n'),
      'utf8',
    );

    const config = await loadConfig({ configPath: configFile, env: {} });

    expect(config.library.roots).toHaveLength(1);
    expect(config.library.roots[0]?.path).toBe(path.join(dir, 'anime'));
    expect(config.database.path).toBe(path.join(dir, 'data', 'library.db'));
  });

  it('falla con un mensaje que indica el archivo y el campo', async () => {
    const dir = await mkdtemp(path.join(tmpdir(), 'mm-int-'));
    const configFile = path.join(dir, 'config.yaml');
    await writeFile(
      configFile,
      'player:\n  default: mpv\nlibrary:\n  scan:\n    maxDepth: 99\n',
      'utf8',
    );

    const error = await loadConfig({ configPath: configFile, env: {} }).catch(
      (cause: unknown) => cause,
    );

    expect(error).toBeInstanceOf(ConfigError);
    expect((error as ConfigError).message).toContain('library.scan.maxDepth');
    expect((error as ConfigError).filePath).toBe(configFile);
  });

  it('funciona sin archivo de configuración', async () => {
    const dir = await mkdtemp(path.join(tmpdir(), 'mm-int-'));

    const config = await loadConfig({ configPath: path.join(dir, 'nada.yaml'), env: {} });

    expect(config.version).toBe(1);
    expect(config.library.roots).toEqual([]);
  });
});
