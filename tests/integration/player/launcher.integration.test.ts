import { existsSync } from 'node:fs';
import { chmod, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { createLaunchPlan, isExecutable, launchPlayer } from '../../../src/core/player/launcher.js';
import { PlayerError } from '../../../src/shared/errors.js';

import type { PlayerConfig } from '../../../src/core/config/schema.js';

describe('launcher de reproductores (integración)', () => {
  let workDir: string;
  let outputPath: string;

  beforeEach(async () => {
    workDir = await mkdtemp(path.join(tmpdir(), 'mm-player-'));
    outputPath = path.join(workDir, 'args.txt');
  });

  afterEach(async () => {
    await rm(workDir, { recursive: true, force: true });
  });

  async function fakePlayer(body: string): Promise<string> {
    const scriptPath = path.join(workDir, 'fake player.sh');
    await writeFile(scriptPath, `#!/bin/sh\n${body}\n`, 'utf8');
    await chmod(scriptPath, 0o755);
    return scriptPath;
  }

  function configFor(command: string, mode: 'detach' | 'foreground'): PlayerConfig {
    return {
      default: 'fake',
      onLaunch: mode,
      players: {
        fake: {
          command,
          args: ['--flag', '{file}'],
          env: { MM_FAKE_OUTPUT: outputPath },
        },
      },
    };
  }

  it('ejecuta el argv real sin interpretar metacaracteres de shell', async () => {
    const script = await fakePlayer('printf "%s\\n" "$@" > "$MM_FAKE_OUTPUT"');
    const filePath = path.join(workDir, 'A $(touch hacked).mkv');
    const plan = createLaunchPlan({ playerConfig: configFor(script, 'detach'), filePath });

    const handle = await launchPlayer(plan);
    await handle.wait();

    const lines = (await readFile(outputPath, 'utf8')).trimEnd().split('\n');
    expect(lines).toEqual(['--flag', filePath]);
    expect(existsSync(path.join(workDir, 'hacked'))).toBe(false);
  });

  it('espera al proceso en modo foreground y propaga un código de salida distinto de 0', async () => {
    const script = await fakePlayer('exit 3');
    const plan = createLaunchPlan({
      playerConfig: configFor(script, 'foreground'),
      filePath: '/media/movie.mkv',
    });

    const handle = await launchPlayer(plan);

    await expect(handle.wait()).rejects.toThrow(PlayerError);
    await expect(handle.wait()).rejects.toThrow('terminó con código 3');
  });

  it('falla con un mensaje accionable si el binario no existe', async () => {
    const plan = createLaunchPlan({
      playerConfig: {
        default: 'missing',
        onLaunch: 'detach',
        players: { missing: { command: 'mm-player-que-no-existe', args: ['{file}'] } },
      },
      filePath: '/media/movie.mkv',
    });

    await expect(launchPlayer(plan)).rejects.toThrow('Instálalo o ajusta player.players');
  });

  it('detecta ejecutables por ruta y por PATH', async () => {
    const script = await fakePlayer('exit 0');

    await expect(isExecutable(script)).resolves.toBe(true);
    await expect(isExecutable('sh')).resolves.toBe(true);
    await expect(isExecutable('mm-player-que-no-existe')).resolves.toBe(false);
  });
});
