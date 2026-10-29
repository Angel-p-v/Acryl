import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterEach, beforeEach, describe, expect, it, vi, type MockInstance } from 'vitest';

import { runIndexCommand } from '../../../src/commands/index.js';

const FIXTURES = fileURLToPath(new URL('../../fixtures/library', import.meta.url));

describe('comando mm index (integración)', () => {
  let workDir: string;
  let configPath: string;
  let output: string[];
  let spy: MockInstance<typeof console.log>;

  beforeEach(async () => {
    workDir = await mkdtemp(path.join(tmpdir(), 'mm-cmd-index-'));
    configPath = path.join(workDir, 'config.yaml');
    const quote = (value: string): string => `'${value.replaceAll("'", "''")}'`;

    await writeFile(
      configPath,
      [
        'library:',
        '  roots:',
        `    - path: ${quote(path.join(FIXTURES, 'series'))}`,
        '      type: series',
        `    - path: ${quote(path.join(FIXTURES, 'anime'))}`,
        '      type: anime',
        `    - path: ${quote(path.join(FIXTURES, 'movies'))}`,
        '      type: movie',
        'database:',
        `  path: ${quote(path.join(workDir, 'library.db'))}`,
        '',
      ].join('\n'),
      'utf8',
    );

    output = [];
    spy = vi.spyOn(console, 'log').mockImplementation((...args: unknown[]) => {
      output.push(args.map((arg) => String(arg)).join(' '));
    });
  });

  afterEach(async () => {
    spy.mockRestore();
    await rm(workDir, { recursive: true, force: true });
  });

  it('indexa la biblioteca y resume el resultado', async () => {
    await runIndexCommand({ config: configPath });

    const text = output.join('\n');
    expect(text).toContain('Indexación completada');
    expect(text).toContain('8 ficheros vistos');
    expect(text).toContain('5 títulos nuevos');
  });

  it('emite JSON válido con --json', async () => {
    await runIndexCommand({ config: configPath, json: true });

    const stats = JSON.parse(output.join('\n')) as Record<string, unknown>;
    expect(stats).toMatchObject({
      dryRun: false,
      filesSeen: 8,
      filesAdded: 8,
      mediaAdded: 5,
    });
    expect(stats.runId).toEqual(expect.any(Number));
  });

  it('reutiliza el índice en una segunda ejecución', async () => {
    await runIndexCommand({ config: configPath, json: true });
    output = [];
    await runIndexCommand({ config: configPath, json: true });

    const stats = JSON.parse(output.join('\n')) as Record<string, unknown>;
    expect(stats).toMatchObject({ filesAdded: 0, filesUnchanged: 8, mediaAdded: 0 });
  });

  it('no escribe datos con --dry-run', async () => {
    await runIndexCommand({ config: configPath, dryRun: true, json: true });

    const stats = JSON.parse(output.join('\n')) as Record<string, unknown>;
    expect(stats).toMatchObject({ dryRun: true, runId: null, filesAdded: 8 });

    output = [];
    await runIndexCommand({ config: configPath, json: true });
    const after = JSON.parse(output.join('\n')) as Record<string, unknown>;
    expect(after).toMatchObject({ filesAdded: 8, filesUnchanged: 0 });
  });
});
