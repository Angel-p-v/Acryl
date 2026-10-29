import { mkdtemp, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { describe, expect, it } from 'vitest';

import { runConfigCommand } from '../../../src/commands/config.js';
import { loadConfig } from '../../../src/core/config/loader.js';
import { ValidationError } from '../../../src/shared/errors.js';

async function tempConfig(content: string): Promise<string> {
  const dir = await mkdtemp(path.join(tmpdir(), 'mm-config-set-'));
  const file = path.join(dir, 'config.yaml');
  await writeFile(file, content, 'utf8');
  return file;
}

describe('comando mm config --set', () => {
  it('establece una clave anidada y la persiste en disco', async () => {
    const file = await tempConfig('player:\n  default: mpv\n');

    await runConfigCommand({ config: file, set: 'player.default=vlc' });

    const reloaded = await loadConfig({ configPath: file });
    expect(reloaded.player.default).toBe('vlc');
  });

  it('establece un valor booleano', async () => {
    const file = await tempConfig('ui:\n  compact: false\n');

    await runConfigCommand({ config: file, set: 'ui.compact=true' });

    const reloaded = await loadConfig({ configPath: file });
    expect(reloaded.ui.compact).toBe(true);
  });

  it('establece un valor numérico', async () => {
    const file = await tempConfig('library:\n  scan:\n    maxDepth: 4\n');

    await runConfigCommand({ config: file, set: 'library.scan.maxDepth=6' });

    const reloaded = await loadConfig({ configPath: file });
    expect(reloaded.library.scan.maxDepth).toBe(6);
  });

  it('establece una cadena en una clave nueva de nivel raíz', async () => {
    const file = await tempConfig('player:\n  default: mpv\n');

    await runConfigCommand({ config: file, set: 'logging.level=debug' });

    const reloaded = await loadConfig({ configPath: file });
    expect(reloaded.logging.level).toBe('debug');
  });

  it('lanza ValidationError si --set no tiene el formato clave=valor', async () => {
    const file = await tempConfig('player:\n  default: mpv\n');

    await expect(runConfigCommand({ config: file, set: 'novalor' })).rejects.toBeInstanceOf(
      ValidationError,
    );
    await expect(runConfigCommand({ config: file, set: '=valor' })).rejects.toBeInstanceOf(
      ValidationError,
    );
  });

  it('lanza ValidationError si la ruta no es un objeto', async () => {
    const file = await tempConfig('player:\n  default: mpv\n');

    await expect(
      runConfigCommand({ config: file, set: 'player.default.otra=x' }),
    ).rejects.toBeInstanceOf(ValidationError);
  });
});
