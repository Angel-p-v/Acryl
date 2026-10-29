import { cp, mkdtemp, rm, unlink, utimes } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { defaultConfig } from '../../../src/core/config/defaults.js';
import { closeDatabase, openDatabase, type Db } from '../../../src/core/db/connection.js';
import { createFileRepository } from '../../../src/core/db/repositories/fileRepo.js';
import { createIndexRunRepository } from '../../../src/core/db/repositories/indexRunRepo.js';
import { createMediaRepository } from '../../../src/core/db/repositories/mediaRepo.js';
import { indexLibrary, type IndexLibraryConfig } from '../../../src/core/indexer/service.js';
import { IndexError } from '../../../src/shared/errors.js';

const FIXTURES = fileURLToPath(new URL('../../fixtures/library', import.meta.url));

describe('indexLibrary (integración)', () => {
  let workDir: string;
  let libraryDir: string;
  let db: Db;

  beforeEach(async () => {
    workDir = await mkdtemp(path.join(tmpdir(), 'mm-index-'));
    libraryDir = path.join(workDir, 'library');
    await cp(FIXTURES, libraryDir, { recursive: true });
    db = openDatabase(path.join(workDir, 'library.db'));
  });

  afterEach(async () => {
    closeDatabase(db);
    await rm(workDir, { recursive: true, force: true });
  });

  function repositories(): Parameters<typeof indexLibrary>[0] {
    return {
      mediaRepo: createMediaRepository(db),
      fileRepo: createFileRepository(db),
      indexRunRepo: createIndexRunRepository(db),
    };
  }

  function library(overrides: Partial<IndexLibraryConfig> = {}): IndexLibraryConfig {
    return {
      roots: [
        { path: path.join(libraryDir, 'series'), type: 'series' },
        { path: path.join(libraryDir, 'anime'), type: 'anime' },
        { path: path.join(libraryDir, 'movies'), type: 'movie' },
      ],
      exclude: defaultConfig.library.exclude,
      scan: defaultConfig.library.scan,
      ...overrides,
    };
  }

  function repo() {
    return repositories();
  }

  it('indexa series, anime y películas ignorando muestras y ficheros ocultos', async () => {
    const stats = await indexLibrary(repo(), library());

    expect(stats).toMatchObject({
      dryRun: false,
      rootsScanned: 3,
      filesSeen: 8,
      filesAdded: 8,
      filesUnchanged: 0,
      filesRemoved: 0,
      mediaAdded: 5,
      mediaRemoved: 0,
      skipped: 0,
      warnings: [],
    });
    expect(stats.runId).not.toBeNull();
    expect(stats.durationMs).toBeGreaterThanOrEqual(0);

    const { mediaRepo, fileRepo } = repo();
    expect(mediaRepo.count()).toBe(5);
    expect(fileRepo.count()).toBe(8);

    const severance = mediaRepo.findByFolderPath(
      path.join(libraryDir, 'series', 'Severance (2022)'),
    );
    expect(severance).toMatchObject({ type: 'series', title: 'Severance', year: 2022 });
    expect(fileRepo.listByMedia(severance!.id)).toMatchObject([
      { season: 1, episode: 1 },
      { season: 1, episode: 2 },
    ]);

    const dark = mediaRepo.findByFolderPath(path.join(libraryDir, 'series', 'Dark'));
    expect(dark).toMatchObject({ title: 'Dark', year: null });
    expect(fileRepo.listByMedia(dark!.id)).toHaveLength(2);

    const frieren = mediaRepo.findByFolderPath(path.join(libraryDir, 'anime', 'Frieren'));
    expect(frieren).toMatchObject({ type: 'anime', title: 'Frieren' });
    expect(fileRepo.listByMedia(frieren!.id)).toMatchObject([
      { season: null, episode: 3 },
      { season: null, episode: 4 },
    ]);

    const arrival = mediaRepo.findByFolderPath(
      path.join(libraryDir, 'movies', 'Arrival.2016.1080p.BluRay.x264.mkv'),
    );
    expect(arrival).toMatchObject({ type: 'movie', title: 'Arrival', year: 2016 });
    expect(mediaRepo.search('sever')).toHaveLength(1);
  });

  it('es idempotente: una segunda ejecución no duplica nada', async () => {
    await indexLibrary(repo(), library());
    const stats = await indexLibrary(repo(), library());

    expect(stats).toMatchObject({
      filesSeen: 8,
      filesAdded: 0,
      filesUnchanged: 8,
      filesRemoved: 0,
      mediaAdded: 0,
      mediaRemoved: 0,
    });

    const { mediaRepo, fileRepo } = repo();
    expect(mediaRepo.count()).toBe(5);
    expect(fileRepo.count()).toBe(8);
  });

  it('reindexa un fichero cuando cambia su fecha de modificación', async () => {
    await indexLibrary(repo(), library());

    const target = path.join(libraryDir, 'series', 'Dark', 'Season 2', 'Dark.2x01.mkv');
    const future = new Date(Date.now() + 2000);
    await utimes(target, future, future);

    const stats = await indexLibrary(repo(), library());
    expect(stats).toMatchObject({ filesAdded: 1, filesUnchanged: 7 });
  });

  it('elimina los ficheros borrados y los títulos que se quedan vacíos', async () => {
    await indexLibrary(repo(), library());

    const episodeOne = path.join(
      libraryDir,
      'series',
      'Severance (2022)',
      'Season 01',
      'Severance.S01E01.1080p.mkv',
    );
    const episodeTwo = path.join(
      libraryDir,
      'series',
      'Severance (2022)',
      'Season 01',
      'Severance.S01E02.1080p.mkv',
    );
    await unlink(episodeOne);

    const first = await indexLibrary(repo(), library());
    expect(first).toMatchObject({ filesRemoved: 1, mediaRemoved: 0 });
    expect(repo().fileRepo.count()).toBe(7);

    await unlink(episodeTwo);
    const second = await indexLibrary(repo(), library());
    expect(second).toMatchObject({ filesRemoved: 1, mediaRemoved: 1 });
    expect(repo().mediaRepo.count()).toBe(4);
  });

  it('no escribe nada en modo dry-run', async () => {
    const stats = await indexLibrary(repo(), library(), { dryRun: true });

    expect(stats).toMatchObject({
      dryRun: true,
      runId: null,
      filesSeen: 8,
      filesAdded: 8,
      mediaAdded: 5,
    });

    const { mediaRepo, fileRepo, indexRunRepo } = repo();
    expect(mediaRepo.count()).toBe(0);
    expect(fileRepo.count()).toBe(0);
    expect(indexRunRepo.list()).toHaveLength(0);
  });

  it('en dry-run informa de lo que se eliminaría sin borrarlo', async () => {
    await indexLibrary(repo(), library());
    await unlink(
      path.join(libraryDir, 'anime', 'Frieren', '[SubsPlease] Frieren - 03 (1080p).mkv'),
    );
    await unlink(
      path.join(libraryDir, 'anime', 'Frieren', '[SubsPlease] Frieren - 04 (1080p).mkv'),
    );

    const stats = await indexLibrary(repo(), library(), { dryRun: true });

    expect(stats).toMatchObject({ filesRemoved: 2, mediaRemoved: 1 });
    expect(repo().fileRepo.count()).toBe(8);
    expect(repo().mediaRepo.count()).toBe(5);
  });

  it('con --root solo recorre esa raíz y no borra nada', async () => {
    await indexLibrary(repo(), library());
    await unlink(path.join(libraryDir, 'movies', 'Arrival.2016.1080p.BluRay.x264.mkv'));

    const stats = await indexLibrary(repo(), library(), {
      rootPath: path.join(libraryDir, 'series'),
    });

    expect(stats).toMatchObject({
      rootsScanned: 1,
      filesSeen: 4,
      filesRemoved: 0,
      mediaRemoved: 0,
    });
    // La fila de la película borrada sigue en la base de datos: no se ha recorrido
    // su raíz y borrarla sería destructivo.
    expect(repo().fileRepo.count()).toBe(8);

    const full = await indexLibrary(repo(), library());
    expect(full).toMatchObject({ filesRemoved: 1 });
  });

  it('marca la ejecución como completada con sus estadísticas', async () => {
    const stats = await indexLibrary(repo(), library());
    const run = repo().indexRunRepo.latest();

    expect(run).toMatchObject({
      id: stats.runId,
      status: 'completed',
      filesSeen: 8,
      filesAdded: 8,
      filesRemoved: 0,
    });
    expect(run?.finishedAt).not.toBeNull();
  });

  it('marca la ejecución como fallida si una raíz no existe', async () => {
    const broken = library({
      roots: [{ path: path.join(libraryDir, 'no-existe'), type: 'series' }],
    });

    await expect(indexLibrary(repo(), broken)).rejects.toThrow(IndexError);

    const run = repo().indexRunRepo.latest();
    expect(run).toMatchObject({ status: 'failed', filesSeen: 0 });
  });

  it('falla si no hay raíces configuradas', async () => {
    await expect(indexLibrary(repo(), library({ roots: [] }))).rejects.toThrow(
      /No hay ninguna raíz configurada/,
    );
  });

  it('falla si --root no coincide con ninguna raíz configurada', async () => {
    await expect(
      indexLibrary(repo(), library(), { rootPath: path.join(libraryDir, 'otra') }),
    ).rejects.toThrow(/no está configurada/);
  });

  it('avisa de rutas duplicadas entre raíces y conserva una sola fila', async () => {
    const duplicated = library({
      roots: [
        { path: path.join(libraryDir, 'series'), type: 'series' },
        { path: path.join(libraryDir, 'series'), type: 'anime' },
      ],
    });

    const stats = await indexLibrary(repo(), duplicated);

    expect(stats.filesSeen).toBe(8);
    expect(stats.warnings).toHaveLength(4);
    expect(repo().fileRepo.count()).toBe(4);
    expect(repo().mediaRepo.count()).toBe(2);
  });
});
