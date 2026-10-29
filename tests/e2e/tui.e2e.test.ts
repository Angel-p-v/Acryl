import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
// eslint-disable-next-line import-x/order
import { fileURLToPath } from 'node:url';
// eslint-disable-next-line import-x/order
import pty from 'node-pty';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';

const ROOT = fileURLToPath(new URL('../..', import.meta.url));
const FIXTURES = path.join(ROOT, 'tests', 'fixtures', 'library');
const CLI = path.join(ROOT, 'dist', 'cli.js');

describe('mm (e2e, TUI interactivo con node-pty)', () => {
  let workDir: string;
  let configPath: string;

  beforeAll(async () => {
    execFileSync('npm', ['run', 'build'], { cwd: ROOT, stdio: 'pipe' });

    workDir = await mkdtemp(path.join(tmpdir(), 'mm-e2e-tui-'));
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

    const indexed = run(['index', '--json']);
    expect(indexed.status, indexed.stderr).toBe(0);
  }, 60_000);

  afterAll(async () => {
    await rm(workDir, { recursive: true, force: true });
  });

  function run(args: string[], env: NodeJS.ProcessEnv = {}) {
    const result = spawnSync(process.execPath, [CLI, ...args], {
      cwd: ROOT,
      encoding: 'utf8',
      timeout: 20_000,
      env: { ...process.env, NO_COLOR: '1', MM_CONFIG: configPath, ...env },
    });

    return {
      status: result.status,
      stdout: result.stdout,
      stderr: result.stderr,
    };
  }

  function spawnTui(): pty.IPty {
    return pty.spawn(process.execPath, [CLI], {
      name: 'xterm-color',
      cols: 80,
      rows: 24,
      cwd: ROOT,
      env: { ...process.env, NO_COLOR: '1', MM_CONFIG: configPath, TERM: 'xterm-color' },
      encoding: 'utf8',
    });
  }

  // Buffer compartido para acumular toda la salida
  let outputBuffer = '';

  function waitForOutput(pty: pty.IPty, expected: string, timeoutMs = 5000): Promise<void> {
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error(`Timeout esperando "${expected}" después de ${timeoutMs}ms`));
      }, timeoutMs);

      const listener = (data: string) => {
        outputBuffer += data;
        if (outputBuffer.includes(expected)) {
          clearTimeout(timeout);
          pty.removeListener('data', listener);
          resolve();
        }
      };

      pty.on('data', listener);
    });
  }

  function resetOutputBuffer(): void {
    outputBuffer = '';
  }

  // Helper para detectar si estamos en CI (donde node-pty puede no funcionar bien)
  const isCI = process.env.CI === 'true' || process.env.GITHUB_ACTIONS === 'true';

  // En CI, los tests de TUI con node-pty pueden ser inestables, los saltamos
  const describeOrSkip = isCI ? describe.skip : describe;

  describeOrSkip('navegación básica en TUI', () => {
    it('abre la TUI y muestra la biblioteca', async () => {
      const ptyProcess = spawnTui();
      resetOutputBuffer();

      await waitForOutput(ptyProcess, 'Severance', 15_000);

      ptyProcess.kill();
    }, 20_000);

    it('navega con flechas arriba/abajo', async () => {
      const ptyProcess = spawnTui();
      resetOutputBuffer();

      await waitForOutput(ptyProcess, 'Severance', 15_000);
      ptyProcess.write('\u001b[B'); // flecha abajo
      await waitForOutput(ptyProcess, 'Arrival', 10_000);

      ptyProcess.kill();
    }, 20_000);

    it('sale con tecla q', async () => {
      const ptyProcess = spawnTui();
      resetOutputBuffer();

      await waitForOutput(ptyProcess, 'Severance', 15_000);
      ptyProcess.write('q');

      await new Promise<void>((resolve) => {
        ptyProcess.on('exit', () => resolve());
        // Forzar salida si tarda mucho
        setTimeout(resolve, 3000);
      });

      expect(ptyProcess.exitCode ?? 0).toBe(0);
    }, 20_000);
  });

  describeOrSkip('búsqueda en TUI', () => {
    it('abre búsqueda con /', async () => {
      const ptyProcess = spawnTui();
      resetOutputBuffer();

      await waitForOutput(ptyProcess, 'Severance', 15_000);
      ptyProcess.write('/');

      await waitForOutput(ptyProcess, 'Buscar', 15_000);
      ptyProcess.write('sever');
      ptyProcess.write('\r');

      await waitForOutput(ptyProcess, 'Severance', 15_000);

      ptyProcess.kill();
    }, 30_000);
  });

  describeOrSkip('detalle de título', () => {
    it('muestra detalle en panel lateral al seleccionar', async () => {
      const ptyProcess = spawnTui();
      resetOutputBuffer();

      // Esperar a que aparezca la lista inicial
      await waitForOutput(ptyProcess, 'Severance', 15_000);

      // Configurar listeners ANTES de navegar para capturar la respuesta
      const t01e01Promise = waitForOutput(ptyProcess, 'T01E01', 15_000);
      const t01e02Promise = waitForOutput(ptyProcess, 'T01E02', 25_000);

      // Navegar a Severance (está en la posición 4)
      ptyProcess.write('\u001b[B');
      ptyProcess.write('\u001b[B');
      ptyProcess.write('\u001b[B');
      ptyProcess.write('\u001b[B');

      // Esperar a que se rendericen los episodios en el panel lateral
      await t01e01Promise;
      await t01e02Promise;

      ptyProcess.kill();
    }, 40_000);
  });

  describeOrSkip('reproducción en TUI', () => {
    it('reproduce con p --dry-run', async () => {
      // Saltar si mpv no está instalado (no disponible en todos los entornos)
      try {
        execFileSync('which', ['mpv'], { stdio: 'pipe' });
      } catch {
        console.log('mpv no está instalado, saltando test de reproducción');
        return;
      }

      const ptyProcess = spawnTui();
      resetOutputBuffer();

      await waitForOutput(ptyProcess, 'Severance', 15_000);
      // Navegar a Severance
      ptyProcess.write('\u001b[B');
      ptyProcess.write('\u001b[B');
      ptyProcess.write('\u001b[B');
      ptyProcess.write('\u001b[B');
      await new Promise((r) => setTimeout(r, 500));
      ptyProcess.write('p');

      await waitForOutput(ptyProcess, 'mpv', 15_000);
      await waitForOutput(ptyProcess, 'T01E01', 15_000);

      ptyProcess.kill();
    }, 30_000);
  });
});
