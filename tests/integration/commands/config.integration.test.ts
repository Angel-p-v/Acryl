import { mkdtemp, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi, type MockInstance } from 'vitest';

import { runConfigCommand } from '../../../src/commands/config.js';

describe('comando mm config (integración)', () => {
  let output: string[];
  let spy: MockInstance<typeof console.log>;

  beforeEach(() => {
    output = [];
    spy = vi.spyOn(console, 'log').mockImplementation((...args: unknown[]) => {
      output.push(args.map((arg) => String(arg)).join(' '));
    });
  });

  afterEach(() => {
    spy.mockRestore();
  });

  async function writeConfig(content: string): Promise<string> {
    const dir = await mkdtemp(path.join(tmpdir(), 'mm-cmd-'));
    const file = path.join(dir, 'config.yaml');
    await writeFile(file, content, 'utf8');
    return file;
  }

  it('muestra solo la ruta con --path', async () => {
    await writeConfig('player:\n  default: mpv\n');

    await runConfigCommand({ path: true });

    expect(output.join('\n')).toContain('config.yaml');
  });

  it('valida el archivo y confirma con --validate', async () => {
    await writeConfig('player:\n  default: mpv\n');

    await runConfigCommand({ validate: true });

    expect(output.join('\n')).toMatch(/Configuración válida/);
  });

  it('muestra la configuración efectiva en texto', async () => {
    const file = await writeConfig(
      [
        'library:',
        '  roots:',
        '    - path: /media/anime',
        '      type: anime',
        'player:',
        '  default: vlc',
        '',
      ].join('\n'),
    );

    await runConfigCommand({ config: file });

    const text = output.join('\n');
    expect(text).toContain('Configuración efectiva');
    expect(text).toContain('/media/anime');
    expect(text).toContain('vlc');
    expect(text).toContain(file);
  });

  it('usa el archivo indicado con --config', async () => {
    const file = await writeConfig('player:\n  default: vlc\n');

    await runConfigCommand({ config: file, path: true });

    expect(output.join('\n')).toContain(file);
  });

  it('devuelve JSON válido con --json', async () => {
    await writeConfig('player:\n  default: mpv\n');

    await runConfigCommand({ json: true });

    const parsed: unknown = JSON.parse(output.join('\n'));
    expect(parsed).toMatchObject({ source: expect.any(String) });
  });

  it('indica que usa los valores por defecto si no hay archivo', async () => {
    await runConfigCommand({});

    const text = output.join('\n');
    expect(text).toContain('Configuración efectiva');
  });

  it('--path advierte que el archivo no existe cuando falta', async () => {
    const dir = await mkdtemp(path.join(tmpdir(), 'mm-cmd-'));
    const file = path.join(dir, 'no-existe.yaml');
    // No se escribe el archivo a propósito.

    await runConfigCommand({ config: file, path: true });

    const text = output.join('\n');
    expect(text).toContain(file);
    expect(text).toContain('no existe');
    expect(text).toContain('se usará por defecto');
  });

  it('--path muestra solo la ruta cuando el archivo existe', async () => {
    const file = await writeConfig('player:\n  default: mpv\n');

    await runConfigCommand({ config: file, path: true });

    const text = output.join('\n');
    expect(text).toContain(file);
    expect(text).not.toContain('no existe');
  });
});
