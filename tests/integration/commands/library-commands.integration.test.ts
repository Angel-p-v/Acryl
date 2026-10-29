import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { runDbOptimizeCommand, runDbStatsCommand } from '../../../src/commands/db.js';
import { runIndexCommand } from '../../../src/commands/index.js';
import { runListCommand } from '../../../src/commands/list.js';
import { runSearchCommand } from '../../../src/commands/search.js';
import { runShowCommand } from '../../../src/commands/show.js';
import { NotFoundError } from '../../../src/shared/errors.js';

const FIXTURES = fileURLToPath(new URL('../../fixtures/library', import.meta.url));

describe('comandos de consulta (integración)', () => {
  let workDir: string;
  let configPath: string;

  beforeEach(async () => {
    workDir = await mkdtemp(path.join(tmpdir(), 'mm-library-'));
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

    await capture(() => runIndexCommand({ config: configPath, json: true }));
  });

  afterEach(async () => {
    await rm(workDir, { recursive: true, force: true });
  });

  it('lista la biblioteca con una cabecera y todos los títulos', async () => {
    const output = await capture(() => runListCommand({ config: configPath }));

    expect(output).toContain('5 títulos');
    expect(output).toContain('Severance');
    expect(output).toContain('Dark');
    expect(output).toContain('Frieren');
    expect(output).toContain('Arrival');
    expect(output).toContain('Blade Runner 2049');
  });

  it('filtra por tipo y pagina los resultados', async () => {
    const movies = await capture(() =>
      runListCommand({ config: configPath, type: 'movie', json: true }),
    );
    const moviePayload = JSON.parse(movies) as { total: number; items: { title: string }[] };

    expect(moviePayload.total).toBe(2);
    expect(moviePayload.items.map((item) => item.title)).toEqual(['Arrival', 'Blade Runner 2049']);

    const page = await capture(() =>
      runListCommand({ config: configPath, limit: 2, offset: 2, sort: 'title', json: true }),
    );
    const pagePayload = JSON.parse(page) as {
      total: number;
      count: number;
      offset: number;
      limit: number;
      items: { title: string }[];
    };

    expect(pagePayload).toMatchObject({ total: 5, count: 2, offset: 2, limit: 2 });
    expect(pagePayload.items.map((item) => item.title)).toEqual(['Dark', 'Frieren']);
  });

  it('busca por texto y devuelve JSON estable', async () => {
    const json = await capture(() => runSearchCommand('sever', { config: configPath, json: true }));

    expect(JSON.stringify(JSON.parse(json), null, 2)).toMatchInlineSnapshot(`
      "{
        "query": "sever",
        "count": 1,
        "results": [
          {
            "id": 2,
            "type": "series",
            "title": "Severance",
            "year": 2022,
            "folderPath": "${path.join(FIXTURES, 'series', 'Severance (2022)')}"
          }
        ]
      }"
    `);
    expect(json).not.toContain('\u001b[');
  });

  it('combina búsqueda y filtro de tipo', async () => {
    const movies = await capture(() =>
      runSearchCommand('arrival', { config: configPath, type: 'movie', json: true }),
    );
    const series = await capture(() =>
      runSearchCommand('arrival', { config: configPath, type: 'series', json: true }),
    );

    expect((JSON.parse(movies) as { count: number }).count).toBe(1);
    expect((JSON.parse(series) as { count: number }).count).toBe(0);
  });

  it('muestra el detalle de un título por su nombre', async () => {
    const output = await capture(() => runShowCommand('severance', { config: configPath }));

    expect(output).toContain('Severance (2022)');
    expect(output).toContain('serie');
    expect(output).toContain('2 ficheros');
    expect(output).toContain('T01E01');
    expect(output).toContain('T01E02');
  });

  it('resuelve el mismo título por id y devuelve JSON estable', async () => {
    const json = await capture(() => runShowCommand('2', { config: configPath, json: true }));
    const payload = JSON.parse(json) as {
      id: number;
      title: string;
      fileCount: number;
      files: { season: number | null; episode: number | null }[];
    };

    expect(payload).toMatchObject({ id: 2, title: 'Severance', fileCount: 2 });
    expect(payload.files).toEqual([
      expect.objectContaining({ season: 1, episode: 1 }),
      expect.objectContaining({ season: 1, episode: 2 }),
    ]);
  });

  it('avisa cuando una búsqueda no tiene resultados', async () => {
    const output = await capture(() => runSearchCommand('zzzz', { config: configPath }));

    expect(output).toContain('Sin resultados para «zzzz»');
  });

  it('avisa cuando una página queda fuera del total', async () => {
    const output = await capture(() =>
      runListCommand({ config: configPath, offset: 99, json: false }),
    );

    expect(output).toContain('No hay resultados en esta página');
    expect(output).toContain('5 títulos en total');
  });

  it('explica que la biblioteca está vacía y que hay que indexar', async () => {
    await removeDatabase(workDir);

    const output = await capture(() => runListCommand({ config: configPath }));

    expect(output).toContain('No hay títulos indexados');
    expect(output).toContain('mm index');
  });

  it('resume una base de datos recién creada sin ejecuciones', async () => {
    await removeDatabase(workDir);

    const json = await capture(() => runDbStatsCommand({ config: configPath, json: true }));
    const payload = JSON.parse(json) as {
      media: { total: number };
      files: number;
      lastRun: unknown;
    };

    expect(payload).toMatchObject({ media: { total: 0 }, files: 0, lastRun: null });
  });

  it('falla con NotFoundError si el título no existe', async () => {
    await expect(
      capture(() => runShowCommand('no existe', { config: configPath })),
    ).rejects.toThrow(NotFoundError);
  });

  it('resume el tamaño y el contenido del índice', async () => {
    const output = await capture(() => runDbStatsCommand({ config: configPath }));

    expect(output).toContain('Base de datos');
    expect(output).toContain('5 títulos · 8 ficheros');
    expect(output).toContain('2 series · 1 anime · 2 películas');
    expect(output).toContain('#1 completed');
  });

  it('emite estadísticas en JSON con la última ejecución', async () => {
    const json = await capture(() => runDbStatsCommand({ config: configPath, json: true }));
    const payload = JSON.parse(json) as {
      database: { path: string; sizeBytes: number };
      media: { total: number; series: number; anime: number; movie: number };
      files: number;
      lastRun: { status: string; filesSeen: number; filesAdded: number };
    };

    expect(payload.database.path).toBe(path.join(workDir, 'library.db'));
    expect(payload.database.sizeBytes).toBeGreaterThan(0);
    expect(payload.media).toEqual({ total: 5, series: 2, anime: 1, movie: 2, unknown: 0 });
    expect(payload.files).toBe(8);
    expect(payload.lastRun).toMatchObject({
      status: 'completed',
      filesSeen: 8,
      filesAdded: 8,
    });
  });

  it('optimiza y compacta la base de datos', async () => {
    const json = await capture(() =>
      runDbOptimizeCommand({ config: configPath, vacuum: true, json: true }),
    );
    const payload = JSON.parse(json) as {
      optimized: boolean;
      vacuum: boolean;
      beforeBytes: number;
      afterBytes: number;
      durationMs: number;
    };

    expect(payload.optimized).toBe(true);
    expect(payload.vacuum).toBe(true);
    expect(payload.beforeBytes).toBeGreaterThan(0);
    expect(payload.afterBytes).toBeGreaterThan(0);
    expect(payload.durationMs).toBeGreaterThanOrEqual(0);
  });

  it('muestra el resultado de optimizar en texto', async () => {
    const output = await capture(() => runDbOptimizeCommand({ config: configPath }));

    expect(output).toContain('Base de datos optimizada');
    expect(output).toContain('PRAGMA optimize');
  });
});

/** Borra el índice para simular una instalación recién creada. */
async function removeDatabase(workDir: string): Promise<void> {
  const databasePath = path.join(workDir, 'library.db');
  await Promise.all(
    ['', '-wal', '-shm'].map((suffix) => rm(`${databasePath}${suffix}`, { force: true })),
  );
}

/** Ejecuta un comando capturando lo que escribe en stdout. */
async function capture(run: () => Promise<void>): Promise<string> {
  const lines: string[] = [];
  const spy = vi.spyOn(console, 'log').mockImplementation((...args: unknown[]) => {
    lines.push(args.map((arg) => String(arg)).join(' '));
  });

  try {
    await run();
  } finally {
    spy.mockRestore();
  }

  return lines.join('\n');
}
