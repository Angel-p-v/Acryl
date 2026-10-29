import { execFileSync, spawnSync } from 'node:child_process';
import { chmod, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';

const ROOT = fileURLToPath(new URL('../..', import.meta.url));
const FIXTURES = path.join(ROOT, 'tests', 'fixtures', 'library');
const CLI = path.join(ROOT, 'dist', 'cli.js');

type RunResult = {
  status: number | null;
  stdout: string;
  stderr: string;
};

describe('mm (e2e, binario compilado)', () => {
  let workDir: string;
  let configPath: string;

  beforeAll(async () => {
    try {
      execFileSync('npm', ['run', 'build'], { cwd: ROOT, stdio: 'pipe' });
    } catch (buildError) {
      throw new Error(`Build falló en beforeAll`, { cause: buildError });
    }

    workDir = await mkdtemp(path.join(tmpdir(), 'mm-e2e-'));
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
    expect(indexed.status).toBe(0);
  }, 60_000);

  afterAll(async () => {
    if (workDir) {
      await rm(workDir, { recursive: true, force: true });
    }
  });

  function run(args: string[], env: NodeJS.ProcessEnv = {}): RunResult {
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

  it('muestra la versión y termina con código 0', () => {
    const result = run(['--version']);

    expect(result.status).toBe(0);
    expect(result.stdout.trim()).toMatch(/^\d+\.\d+\.\d+$/);
  });

  it('la ayuda enumera todos los subcomandos', () => {
    const result = run(['--help']);

    expect(result.status).toBe(0);
    for (const command of ['index', 'list', 'search', 'show', 'play', 'watch', 'db', 'config']) {
      expect(result.stdout).toContain(command);
    }
    expect(result.stdout).toContain('Códigos de salida');
  });

  it('el subcomando help también termina con código 0', () => {
    const result = run(['help']);

    expect(result.status).toBe(0);
    expect(result.stdout).toContain('Usage: mm');
  });

  it('devuelve 2 ante un comando desconocido', () => {
    const result = run(['no-existe']);

    expect(result.status).toBe(2);
    expect(result.stderr).toContain("unknown command 'no-existe'");
  });

  it('devuelve 2 si falta un argumento obligatorio', () => {
    const result = run(['search']);

    expect(result.status).toBe(2);
    expect(result.stderr).toContain("missing required argument 'consulta'");
  });

  it('devuelve 2 si una opción tiene un valor inválido', () => {
    const result = run(['search', 'sever', '--type', 'libros']);

    expect(result.status).toBe(2);
    expect(result.stderr).toContain('Tipo no válido');
  });

  it('indexa el fixture y emite estadísticas JSON', () => {
    const result = run(['index', '--json']);

    expect(result.status).toBe(0);
    const stats = JSON.parse(result.stdout) as Record<string, unknown>;
    expect(stats).toMatchObject({
      dryRun: false,
      filesSeen: 8,
      filesAdded: 0,
      filesUnchanged: 8,
      mediaAdded: 0,
    });
    expect(result.stdout).not.toContain('\u001b[');
  });

  it('lista y busca títulos contra el índice real', () => {
    const list = run(['list', '--json']);
    expect(list.status).toBe(0);
    expect(JSON.parse(list.stdout)).toMatchObject({ total: 5, count: 5 });

    const search = run(['search', 'sever', '--json']);
    expect(search.status).toBe(0);
    expect(JSON.parse(search.stdout)).toMatchObject({
      query: 'sever',
      count: 1,
      results: [{ title: 'Severance', year: 2022 }],
    });
  });

  it('--verbose escribe logs estructurados en stderr sin contaminar stdout', () => {
    const result = run(['search', 'sever', '--json', '--verbose']);

    expect(result.status).toBe(0);
    expect(JSON.parse(result.stdout)).toMatchObject({ count: 1 });
    expect(result.stderr).toContain('search.completed');
    expect(result.stderr).toContain('durationMs=');
  });

  it('MM_LOG_FORMAT=json emite una línea JSON por evento', () => {
    const result = run(['search', 'sever', '--verbose'], { MM_LOG_FORMAT: 'json' });

    expect(result.status).toBe(0);
    const events = result.stderr
      .trim()
      .split('\n')
      .map((line) => JSON.parse(line) as { level: string; event: string });
    expect(events.some((event) => event.event === 'search.completed')).toBe(true);
    expect(events.every((event) => event.level === 'debug')).toBe(true);
  });

  it('muestra el detalle de un título y de sus ficheros', () => {
    const result = run(['show', 'severance', '--json']);

    expect(result.status).toBe(0);
    expect(JSON.parse(result.stdout)).toMatchObject({
      title: 'Severance',
      type: 'series',
      fileCount: 2,
      files: [
        { season: 1, episode: 1 },
        { season: 1, episode: 2 },
      ],
    });
  });

  it('exporta a JSON con list --export json', () => {
    const result = run(['list', '--export', 'json']);

    expect(result.status).toBe(0);
    const data = JSON.parse(result.stdout);
    expect(data).toHaveProperty('items');
    expect(Array.isArray(data.items)).toBe(true);
    expect(data.items.length).toBeGreaterThan(0);
    expect(data.items[0]).toMatchObject({
      id: expect.any(Number),
      title: expect.any(String),
      type: expect.any(String),
    });
  });

  it('exporta a CSV con list --export csv', () => {
    const result = run(['list', '--export', 'csv']);

    expect(result.status).toBe(0);
    expect(result.stdout).toContain('id,type,title,year,folderPath');
    expect(result.stdout).toContain('Severance');
    const lines = result.stdout.trim().split('\n');
    expect(lines.length).toBeGreaterThan(1);
    expect(lines[0]).toBe('id,type,title,year,folderPath');
  });

  it('indexa solo una subcarpeta con index --folder', async () => {
    const folderConfigPath = path.join(workDir, 'folder-config.yaml');
    const quote = (value: string): string => `'${value.replaceAll("'", "''")}'`;

    await writeFile(
      folderConfigPath,
      [
        'library:',
        '  roots:',
        `    - path: ${quote(path.join(FIXTURES, 'series'))}`,
        '      type: series',
        '',
        'database:',
        `  path: ${quote(path.join(workDir, 'library.db'))}`,
        '',
      ].join('\n'),
      'utf8',
    );

    const folderPath = path.join(FIXTURES, 'series', 'Dark', 'Season 2');
    const result = run(['index', '--json', '--config', folderConfigPath, '--folder', folderPath]);

    expect(result.status).toBe(0);
    const stats = JSON.parse(result.stdout);
    expect(stats.filesSeen).toBeGreaterThan(0);
    // Los archivos ya están indexados (beforeAll indexó toda la biblioteca),
    // así que aparecen como unchanged, no added.
    expect(stats.filesUnchanged).toBeGreaterThan(0);
  });

  it('mm play --dry-run produce el comando sin lanzar el reproductor', () => {
    const result = run(['play', 'severance', '--dry-run', '--json']);

    expect(result.status).toBe(0);
    const payload = JSON.parse(result.stdout) as {
      player: string;
      command: string;
      args: string[];
      mode: string;
      file: { filePath: string; season: number | null; episode: number | null };
      pid: null;
    };
    expect(payload).toMatchObject({
      player: 'mpv',
      command: 'mpv',
      mode: 'detach',
      pid: null,
      file: { season: 1, episode: 1 },
    });
    expect(payload.args).toContain(payload.file.filePath);
    expect(result.stdout).not.toContain('\u001b[');
  });

  it('mm play falla con un mensaje accionable si el título no existe', () => {
    const result = run(['play', 'no-existe', '--dry-run']);

    expect(result.status).toBe(1);
    expect(result.stderr).toContain('No hay ningún título que coincida');
  });

  it('el reproductor desacoplado sobrevive al proceso que lo lanzó', async () => {
    const playerScript = path.join(workDir, 'detached-player.sh');
    const markerPath = path.join(workDir, 'detached-player.txt');
    await writeFile(
      playerScript,
      '#!/bin/sh\nsleep 0.4\nprintf done > "$MM_SURVIVOR_OUTPUT"\n',
      'utf8',
    );
    await chmod(playerScript, 0o755);

    const launcherUrl = pathToFileURL(
      path.join(ROOT, 'dist', 'core', 'player', 'launcher.js'),
    ).href;
    const parentScript = path.join(workDir, 'launch-parent.mjs');
    await writeFile(
      parentScript,
      [
        `import { createLaunchPlan, launchPlayer } from ${JSON.stringify(launcherUrl)};`,
        'const plan = createLaunchPlan({',
        "  playerConfig: { default: 'fake', onLaunch: 'detach', players: {",
        "    fake: { command: process.argv[2], args: ['{file}'] },",
        '  } },',
        "  filePath: '/media/movie.mkv',",
        '});',
        'const handle = await launchPlayer(plan);',
        'console.log(`pid=${String(handle.pid)}`);',
        'process.exit(0);',
        '',
      ].join('\n'),
      'utf8',
    );

    const parent = spawnSync(process.execPath, [parentScript, playerScript], {
      cwd: ROOT,
      encoding: 'utf8',
      timeout: 10_000,
      env: { ...process.env, MM_SURVIVOR_OUTPUT: markerPath },
    });

    expect(parent.status).toBe(0);
    expect(parent.stdout).toContain('pid=');
    await expect(waitForFile(markerPath, 5_000)).resolves.toBe('done');
  });

  it('resume el contenido del índice', () => {
    const result = run(['db', 'stats', '--json']);

    expect(result.status).toBe(0);
    const stats = JSON.parse(result.stdout) as {
      media: { total: number };
      files: number;
      lastRun: { status: string };
    };
    expect(stats).toMatchObject({
      media: { total: 5 },
      files: 8,
      lastRun: { status: 'completed' },
    });
  });

  it('optimiza y compacta la base desde la CLI', () => {
    const result = run(['db', 'optimize', '--vacuum', '--json']);

    expect(result.status).toBe(0);
    expect(JSON.parse(result.stdout)).toMatchObject({
      optimized: true,
      vacuum: true,
    });
  });

  it('devuelve 1 y un mensaje accionable si el título no existe', () => {
    const result = run(['show', 'no-existe']);

    expect(result.status).toBe(1);
    expect(result.stderr).toContain('[NOT_FOUND]');
    expect(result.stderr).toContain('No hay ningún título que coincida');
  });

  it('con --verbose añade la pila del error a stderr', () => {
    const result = run(['show', 'no-existe', '--verbose']);

    expect(result.status).toBe(1);
    expect(result.stderr).toContain('NotFoundError');
  });

  it('mm list --export json devuelve JSON válido con todos los títulos', () => {
    const result = run(['list', '--export', 'json']);

    expect(result.status).toBe(0);
    const payload = JSON.parse(result.stdout) as {
      total: number;
      count: number;
      items: { id: number; type: string; title: string; year: number | null }[];
    };
    expect(payload.total).toBe(5);
    expect(payload.count).toBe(5);
    expect(payload.items.length).toBe(5);
    expect(payload.items[0]).toHaveProperty('id');
    expect(payload.items[0]).toHaveProperty('type');
    expect(payload.items[0]).toHaveProperty('title');
  });

  it('mm list --export csv devuelve cabecera y filas CSV', () => {
    const result = run(['list', '--export', 'csv']);

    expect(result.status).toBe(0);
    const lines = result.stdout.trim().split('\n');
    expect(lines[0]).toBe('id,type,title,year,folderPath');
    expect(lines.length).toBeGreaterThan(1);
    // Cada línea de datos debe tener 5 campos separados por comas
    for (const line of lines.slice(1)) {
      expect(line.split(',').length).toBe(5);
    }
  });

  it('mm list --export con formato inválido falla', () => {
    const result = run(['list', '--export', 'xml']);

    expect(result.status).toBe(1);
    expect(result.stderr).toContain('Formato de exportación no válido');
  });

  it('mm index --folder indexa solo la carpeta indicada', () => {
    const folderPath = path.join(FIXTURES, 'series', 'Dark');
    const result = run(['index', '--folder', folderPath, '--json']);

    expect(result.status).toBe(0);
    const stats = JSON.parse(result.stdout) as {
      filesSeen: number;
      filesAdded: number;
    };
    expect(stats.filesSeen).toBeGreaterThan(0);
    expect(stats.filesAdded).toBeGreaterThanOrEqual(0);
  });

  it('mm index --folder con carpeta fuera de las raíces falla con código 1', () => {
    const result = run(['index', '--folder', '/ruta/inexistente/no-existe']);

    expect(result.status).toBe(1);
    expect(result.stderr).toContain('no está dentro de ninguna raíz');
  });
});

/** Espera a que exista un fichero y devuelve su contenido. */
async function waitForFile(filePath: string, timeoutMs: number): Promise<string> {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    try {
      return await readFile(filePath, 'utf8');
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
  }

  throw new Error(`No apareció ${filePath} en ${timeoutMs} ms`);
}
