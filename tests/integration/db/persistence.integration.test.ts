import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import {
  closeDatabase,
  openDatabase,
  runMigrations,
  type Db,
} from '../../../src/core/db/connection.js';
import { migrations } from '../../../src/core/db/migrations/index.js';
import { createFileRepository } from '../../../src/core/db/repositories/fileRepo.js';
import { createIndexRunRepository } from '../../../src/core/db/repositories/indexRunRepo.js';
import { createMediaRepository } from '../../../src/core/db/repositories/mediaRepo.js';
import { DbError } from '../../../src/shared/errors.js';

describe('capa de persistencia (integración)', () => {
  let db: Db;

  beforeEach(() => {
    db = openDatabase(':memory:');
  });

  afterEach(() => {
    closeDatabase(db);
  });

  describe('migraciones', () => {
    it('crea el esquema esperado', () => {
      const tables = db
        .prepare<[], { name: string }>(
          "SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name",
        )
        .all()
        .map((row) => row.name);

      expect(tables).toContain('media');
      expect(tables).toContain('media_files');
      expect(tables).toContain('index_runs');
      expect(tables).toContain('media_fts');
    });

    it('es idempotente: aplicarlas dos veces no falla', () => {
      expect(() => runMigrations(db)).not.toThrow();
      expect(() => runMigrations(db)).not.toThrow();

      const applied = db
        .prepare<[], { total: number }>('SELECT COUNT(*) AS total FROM schema_migrations')
        .get();

      expect(applied?.total).toBe(migrations.length);
    });

    it('registra la versión aplicada', () => {
      const row = db
        .prepare<[], { version: number; name: string }>(
          'SELECT version, name FROM schema_migrations ORDER BY version',
        )
        .get();

      expect(row?.version).toBe(1);
      expect(row?.name).toBe('init');
    });
  });

  describe('repositorio de títulos', () => {
    it('inserta y lee un título', () => {
      const repo = createMediaRepository(db);

      const media = repo.upsert({
        type: 'anime',
        title: 'Cowboy Bebop',
        folderPath: '/media/Cowboy Bebop',
        year: 1998,
      });

      expect(media.id).toBeGreaterThan(0);
      expect(media.title).toBe('Cowboy Bebop');
      expect(media.type).toBe('anime');
      expect(repo.findById(media.id)?.title).toBe('Cowboy Bebop');
    });

    it('actualiza en lugar de duplicar cuando la carpeta ya existe', () => {
      const repo = createMediaRepository(db);

      repo.upsert({ type: 'anime', title: 'Akira', folderPath: '/media/Akira' });
      repo.upsert({ type: 'anime', title: 'Akira (1988)', folderPath: '/media/Akira' });

      expect(repo.count('anime')).toBe(1);
      expect(repo.findByFolderPath('/media/Akira')?.title).toBe('Akira (1988)');
    });

    it('filtra por tipo y ordena por título', () => {
      const repo = createMediaRepository(db);
      repo.upsert({ type: 'anime', title: 'Zeta', folderPath: '/media/Zeta' });
      repo.upsert({ type: 'movie', title: 'Alien', folderPath: '/media/Alien' });
      repo.upsert({ type: 'anime', title: 'Akira', folderPath: '/media/Akira' });

      const anime = repo.list({ type: 'anime' });

      expect(anime.map((item) => item.title)).toEqual(['Akira', 'Zeta']);
    });

    it('encuentra por búsqueda de texto', () => {
      const repo = createMediaRepository(db);
      repo.upsert({ type: 'anime', title: 'Cowboy Bebop', folderPath: '/media/cb' });
      repo.upsert({ type: 'movie', title: 'Alien', folderPath: '/media/alien' });

      const results = repo.search('cowboy');

      expect(results).toHaveLength(1);
      expect(results[0]?.title).toBe('Cowboy Bebop');
    });

    it('busca por prefijo', () => {
      const repo = createMediaRepository(db);
      repo.upsert({ type: 'anime', title: 'Cowboy Bebop', folderPath: '/media/cb' });

      expect(repo.search('beb')).toHaveLength(1);
    });

    it('devuelve vacío para consultas sin términos', () => {
      const repo = createMediaRepository(db);

      expect(repo.search('   ')).toEqual([]);
    });

    it('borra un título y sus ficheros en cascada', () => {
      const mediaRepo = createMediaRepository(db);
      const fileRepo = createFileRepository(db);
      const media = mediaRepo.upsert({ type: 'anime', title: 'Akira', folderPath: '/media/akira' });
      fileRepo.upsertMany([{ mediaId: media.id, filePath: '/media/akira/e01.mkv', episode: 1 }]);

      expect(mediaRepo.remove(media.id)).toBe(true);
      expect(fileRepo.count()).toBe(0);
    });
  });

  describe('repositorio de ficheros', () => {
    it('guarda varios ficheros en una transacción', () => {
      const mediaRepo = createMediaRepository(db);
      const fileRepo = createFileRepository(db);
      const media = mediaRepo.upsert({
        type: 'series',
        title: 'Severance',
        folderPath: '/media/sev',
      });

      const inserted = fileRepo.upsertMany([
        { mediaId: media.id, filePath: '/media/sev/s01e01.mkv', season: 1, episode: 1 },
        { mediaId: media.id, filePath: '/media/sev/s01e02.mkv', season: 1, episode: 2 },
      ]);

      expect(inserted).toBe(2);
      expect(fileRepo.count(media.id)).toBe(2);
    });

    it('actualiza los metadatos de un fichero existente', () => {
      const mediaRepo = createMediaRepository(db);
      const fileRepo = createFileRepository(db);
      const media = mediaRepo.upsert({
        type: 'series',
        title: 'Severance',
        folderPath: '/media/sev',
      });

      fileRepo.upsertMany([
        { mediaId: media.id, filePath: '/media/sev/s01e01.mkv', season: 1, episode: 1 },
      ]);
      fileRepo.upsertMany([
        {
          mediaId: media.id,
          filePath: '/media/sev/s01e01.mkv',
          season: 1,
          episode: 1,
          title: 'Cap. 1',
          sizeBytes: 1024,
        },
      ]);

      const file = fileRepo.findByPath('/media/sev/s01e01.mkv');
      expect(file?.title).toBe('Cap. 1');
      expect(file?.sizeBytes).toBe(1024);
      expect(fileRepo.count()).toBe(1);
    });

    it('elimina los ficheros que ya no existen en disco', () => {
      const mediaRepo = createMediaRepository(db);
      const fileRepo = createFileRepository(db);
      const media = mediaRepo.upsert({
        type: 'series',
        title: 'Severance',
        folderPath: '/media/sev',
      });
      fileRepo.upsertMany([
        { mediaId: media.id, filePath: '/media/sev/s01e01.mkv' },
        { mediaId: media.id, filePath: '/media/sev/s01e02.mkv' },
      ]);

      const removed = fileRepo.removeMissing(new Set(['/media/sev/s01e01.mkv']));

      expect(removed).toBe(1);
      expect(fileRepo.listAllPaths()).toEqual(['/media/sev/s01e01.mkv']);
    });

    it('no borra nada cuando todos los ficheros siguen en disco', () => {
      const mediaRepo = createMediaRepository(db);
      const fileRepo = createFileRepository(db);
      const media = mediaRepo.upsert({
        type: 'series',
        title: 'Severance',
        folderPath: '/media/sev',
      });
      fileRepo.upsertMany([{ mediaId: media.id, filePath: '/media/sev/s01e01.mkv' }]);

      expect(fileRepo.removeMissing(new Set(['/media/sev/s01e01.mkv']))).toBe(0);
    });

    it('lista los ficheros de un título y todos los títulos', () => {
      const mediaRepo = createMediaRepository(db);
      const fileRepo = createFileRepository(db);
      const media = mediaRepo.upsert({
        type: 'series',
        title: 'Severance',
        folderPath: '/media/sev',
      });
      fileRepo.upsertMany([
        { mediaId: media.id, filePath: '/media/sev/s01e01.mkv', season: 1, episode: 1 },
        { mediaId: media.id, filePath: '/media/sev/s01e02.mkv', season: 1, episode: 2 },
      ]);

      expect(fileRepo.listByMedia(media.id)).toHaveLength(2);
      expect(fileRepo.listAllPaths()).toHaveLength(2);
      expect(fileRepo.count()).toBe(2);
      expect(fileRepo.count(media.id)).toBe(2);
    });

    it('elimina los ficheros de un título', () => {
      const mediaRepo = createMediaRepository(db);
      const fileRepo = createFileRepository(db);
      const media = mediaRepo.upsert({
        type: 'series',
        title: 'Severance',
        folderPath: '/media/sev',
      });
      fileRepo.upsertMany([{ mediaId: media.id, filePath: '/media/sev/s01e01.mkv' }]);

      expect(fileRepo.removeByMedia(media.id)).toBe(1);
      expect(fileRepo.count()).toBe(0);
    });
  });

  describe('repositorio de títulos: conteo y carpetas', () => {
    it('cuenta por tipo y en total', () => {
      const repo = createMediaRepository(db);
      repo.upsert({ type: 'anime', title: 'Akira', folderPath: '/media/akira' });
      repo.upsert({ type: 'movie', title: 'Alien', folderPath: '/media/alien' });

      expect(repo.count('anime')).toBe(1);
      expect(repo.count('movie')).toBe(1);
      expect(repo.count()).toBe(2);
    });

    it('lista las carpetas de la biblioteca', () => {
      const repo = createMediaRepository(db);
      repo.upsert({ type: 'anime', title: 'Akira', folderPath: '/media/akira' });

      expect(repo.listFolders()).toEqual(['/media/akira']);
    });

    it('elimina los títulos que se quedan sin ficheros', () => {
      const mediaRepo = createMediaRepository(db);
      const fileRepo = createFileRepository(db);
      const withFile = mediaRepo.upsert({
        type: 'series',
        title: 'Severance',
        folderPath: '/media/sev',
      });
      mediaRepo.upsert({ type: 'movie', title: 'Alien', folderPath: '/media/alien' });
      fileRepo.upsertMany([{ mediaId: withFile.id, filePath: '/media/sev/s01e01.mkv' }]);

      expect(mediaRepo.removeEmpty()).toBe(1);
      expect(mediaRepo.count()).toBe(1);
      expect(mediaRepo.removeEmpty()).toBe(0);
    });
  });

  describe('repositorio de ejecuciones', () => {
    it('registra el inicio y el final de una indexación', () => {
      const repo = createIndexRunRepository(db);

      const id = repo.start();
      repo.complete(id, { filesSeen: 10, filesAdded: 8, filesRemoved: 1 });

      const run = repo.latest();
      expect(run?.status).toBe('completed');
      expect(run?.filesSeen).toBe(10);
      expect(run?.filesAdded).toBe(8);
      expect(run?.filesRemoved).toBe(1);
    });

    it('marca una ejecución como fallida', () => {
      const repo = createIndexRunRepository(db);

      const id = repo.start();
      repo.fail(id, { filesSeen: 3 });

      expect(repo.latest()?.status).toBe('failed');
    });
  });
});

describe('openDatabase', () => {
  it('crea la carpeta de la base de datos si no existe', async () => {
    const dir = await mkdtemp(path.join(tmpdir(), 'mm-db-'));
    const dbPath = path.join(dir, 'nested', 'library.db');

    const db = openDatabase(dbPath);
    closeDatabase(db);

    expect(db.open).toBe(false);
    await rm(dir, { recursive: true, force: true });
  });

  it('lanza DbError si la ruta no es válida', async () => {
    const dir = await mkdtemp(path.join(tmpdir(), 'mm-db-'));
    // Un fichero normal donde debería ir una carpeta: mkdir falla con ENOTDIR.
    const blocker = path.join(dir, 'bloqueado');
    await writeFile(blocker, 'no soy una carpeta', 'utf8');

    expect(() => openDatabase(path.join(blocker, 'library.db'))).toThrow(DbError);

    await rm(dir, { recursive: true, force: true });
  });

  it('lanza DbError si la base de datos no se puede abrir', async () => {
    const dir = await mkdtemp(path.join(tmpdir(), 'mm-db-'));
    // Una carpeta no es un fichero de base de datos válido.
    const asDirectory = path.join(dir, 'carpeta.db');
    await mkdir(asDirectory, { recursive: true });

    expect(() => openDatabase(asDirectory)).toThrow(DbError);

    await rm(dir, { recursive: true, force: true });
  });
});

describe('runMigrations', () => {
  it('lanza DbError si una migración falla', () => {
    const db = openDatabase(':memory:');

    expect(() =>
      runMigrations(db, [{ version: 99, name: 'rota', sql: 'esto no es SQL válido;' }]),
    ).toThrow(DbError);

    // La migración fallida no queda registrada, así que se puede reintentar.
    const applied = db
      .prepare<[], { total: number }>('SELECT COUNT(*) AS total FROM schema_migrations')
      .get();
    expect(applied?.total).toBe(migrations.length);

    closeDatabase(db);
  });
});
