import { existsSync } from 'node:fs';
import { chmod, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterEach, beforeEach, describe, expect, it, vi, type MockInstance } from 'vitest';

import { runIndexCommand } from '../../../src/commands/index.js';
import { runPlayCommand } from '../../../src/commands/play.js';
import { openLibrary } from '../../../src/core/library/open.js';
import { PlayerError } from '../../../src/shared/errors.js';

const FIXTURES = fileURLToPath(new URL('../../fixtures/library', import.meta.url));

describe('comando mm play (integración)', () => {
  let workDir: string;
  let configPath: string;
  let outputPath: string;
  let spy: MockInstance<typeof console.log>;
  let output: string[];

  beforeEach(async () => {
    workDir = await mkdtemp(path.join(tmpdir(), 'mm-play-'));
    configPath = path.join(workDir, 'config.yaml');
    outputPath = path.join(workDir, 'player-args.txt');

    const scriptPath = path.join(workDir, 'fake player.sh');
    await writeFile(scriptPath, '#!/bin/sh\nprintf "%s\\n" "$@" > "$MM_FAKE_OUTPUT"\n', 'utf8');
    await chmod(scriptPath, 0o755);

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
        'player:',
        '  default: fake',
        '  onLaunch: foreground',
        '  players:',
        '    fake:',
        `      command: ${quote(scriptPath)}`,
        "      args: ['--flag', '{file}']",
        '      env:',
        `        MM_FAKE_OUTPUT: ${quote(outputPath)}`,
        '',
      ].join('\n'),
      'utf8',
    );

    output = [];
    spy = vi.spyOn(console, 'log').mockImplementation((...args: unknown[]) => {
      output.push(args.map((arg) => String(arg)).join(' '));
    });

    await runIndexCommand({ config: configPath, json: true });
    output.length = 0;
  });

  afterEach(async () => {
    spy.mockRestore();
    await rm(workDir, { recursive: true, force: true });
  });

  it('reproduce el primer fichero de una serie con el reproductor configurado', async () => {
    await runPlayCommand('severance', { config: configPath, json: true });

    const args = (await readFile(outputPath, 'utf8')).trimEnd().split('\n');
    expect(args[0]).toBe('--flag');
    expect(args[1]).toContain('Severance.S01E01');
    expect(output.join('\n')).toContain('"mode": "foreground"');
  });

  it('permite elegir el fichero por id', async () => {
    const opened = openLibrary(path.join(workDir, 'library.db'));
    const secondFile = opened.library.find('severance').files[1];
    opened.close();
    expect(secondFile).toBeDefined();

    await runPlayCommand('severance', {
      config: configPath,
      file: secondFile!.id,
      json: true,
    });

    const args = (await readFile(outputPath, 'utf8')).trimEnd().split('\n');
    expect(args[1]).toContain('Severance.S01E02');
  });

  it('con --dry-run muestra el comando y no lanza el proceso', async () => {
    await runPlayCommand('severance', { config: configPath, dryRun: true, json: true });

    const payload = JSON.parse(output.join('\n')) as {
      mode: string;
      args: string[];
      file: { filePath: string };
      pid: null;
    };
    expect(payload.mode).toBe('foreground');
    expect(payload.args[0]).toBe('--flag');
    expect(payload.file.filePath).toContain('S01E01');
    expect(payload.pid).toBeNull();
    expect(existsSync(outputPath)).toBe(false);
  });

  it('resuelve un reproductor alternativo', async () => {
    await expect(
      runPlayCommand('severance', { config: configPath, player: 'no-existe', dryRun: true }),
    ).rejects.toThrow(PlayerError);
  });
});
