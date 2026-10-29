import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { closeDatabase, openDatabase } from '../../../src/core/db/connection.js';
import { createFileRepository } from '../../../src/core/db/repositories/fileRepo.js';
import { createMediaRepository } from '../../../src/core/db/repositories/mediaRepo.js';
import { createPlaybackRepository } from '../../../src/core/db/repositories/playbackRepo.js';

import type { Db } from '../../../src/core/db/connection.js';
import type { MediaFileRow, MediaRow } from '../../../src/core/db/types.js';

const T1 = '2026-10-23T10:00:00.000Z';
const T2 = '2026-10-23T11:00:00.000Z';

describe('repositorio de reproducción (integración)', () => {
  let db: Db;

  beforeEach(() => {
    db = openDatabase(':memory:');
  });

  afterEach(() => {
    closeDatabase(db);
  });

  function seed(): { media: MediaRow; files: MediaFileRow[] } {
    const mediaRepo = createMediaRepository(db);
    const fileRepo = createFileRepository(db);
    const media = mediaRepo.upsert({
      type: 'series',
      title: 'Severance',
      year: 2022,
      folderPath: '/library/Severance',
    });
    fileRepo.upsertMany([
      { mediaId: media.id, season: 1, episode: 1, filePath: '/library/Severance/S01E01.mkv' },
      { mediaId: media.id, season: 1, episode: 2, filePath: '/library/Severance/S01E02.mkv' },
    ]);
    return { media, files: fileRepo.listByMedia(media.id) };
  }

  it('guarda y lee el estado con los valores por defecto', () => {
    const repo = createPlaybackRepository(db);
    const { files } = seed();

    const saved = repo.save({
      mediaFileId: files[0]!.id,
      positionSeconds: 120.5,
      durationSeconds: 2700,
      lastPlayedAt: T1,
      updatedAt: T1,
    });

    expect(saved).toEqual({
      mediaFileId: files[0]!.id,
      positionSeconds: 120.5,
      durationSeconds: 2700,
      completedAt: null,
      lastPlayedAt: T1,
      playCount: 0,
      updatedAt: T1,
    });
    expect(repo.get(files[0]!.id)).toEqual(saved);
    expect(repo.get(9999)).toBeUndefined();
  });

  it('actualiza la posición y el contador sobre la misma fila', () => {
    const repo = createPlaybackRepository(db);
    const { files } = seed();

    repo.save({ mediaFileId: files[0]!.id, positionSeconds: 10, lastPlayedAt: T1, updatedAt: T1 });
    const updated = repo.save({
      mediaFileId: files[0]!.id,
      positionSeconds: 300,
      durationSeconds: 2700,
      completedAt: T2,
      lastPlayedAt: T2,
      playCount: 2,
      updatedAt: T2,
    });

    expect(updated.positionSeconds).toBe(300);
    expect(updated.completedAt).toBe(T2);
    expect(updated.playCount).toBe(2);
    expect(repo.count()).toBe(1);
  });

  it('elimina el estado de un fichero', () => {
    const repo = createPlaybackRepository(db);
    const { files } = seed();
    repo.save({ mediaFileId: files[0]!.id, positionSeconds: 5, lastPlayedAt: T1, updatedAt: T1 });

    expect(repo.remove(files[0]!.id)).toBe(true);
    expect(repo.remove(files[0]!.id)).toBe(false);
    expect(repo.count()).toBe(0);
  });

  it('lista lo empezado y no completado por última reproducción', () => {
    const repo = createPlaybackRepository(db);
    const { media, files } = seed();

    repo.save({
      mediaFileId: files[0]!.id,
      positionSeconds: 100,
      lastPlayedAt: T1,
      updatedAt: T1,
    });
    repo.save({
      mediaFileId: files[1]!.id,
      positionSeconds: 200,
      lastPlayedAt: T2,
      updatedAt: T2,
    });

    const inProgress = repo.listInProgress();

    expect(inProgress.map((entry) => entry.file.id)).toEqual([files[1]!.id, files[0]!.id]);
    expect(inProgress[0]!.state.positionSeconds).toBe(200);
    expect(inProgress[0]!.media).toEqual({
      id: media.id,
      type: 'series',
      title: 'Severance',
      sortTitle: 'severance',
      year: 2022,
      folderPath: '/library/Severance',
      createdAt: expect.any(String),
      updatedAt: expect.any(String),
    });
    expect(inProgress[0]!.file).toEqual({
      id: files[1]!.id,
      mediaId: media.id,
      filePath: '/library/Severance/S01E02.mkv',
      season: 1,
      episode: 2,
      title: null,
      sizeBytes: null,
      mtimeMs: null,
      indexedAt: expect.any(String),
    });
  });

  it('excluye de «en curso» lo completado y lo que sigue en el segundo cero', () => {
    const repo = createPlaybackRepository(db);
    const { files } = seed();

    repo.save({
      mediaFileId: files[0]!.id,
      positionSeconds: 0,
      lastPlayedAt: T1,
      updatedAt: T1,
    });
    repo.save({
      mediaFileId: files[1]!.id,
      positionSeconds: 300,
      durationSeconds: 300,
      completedAt: T2,
      lastPlayedAt: T2,
      updatedAt: T2,
    });

    expect(repo.listInProgress()).toHaveLength(0);
    expect(repo.listRecent().map((entry) => entry.state.completedAt)).toEqual([T2, null]);
  });

  it('respeta el límite del historial', () => {
    const repo = createPlaybackRepository(db);
    const { files } = seed();

    repo.save({ mediaFileId: files[0]!.id, positionSeconds: 10, lastPlayedAt: T1, updatedAt: T1 });
    repo.save({ mediaFileId: files[1]!.id, positionSeconds: 20, lastPlayedAt: T2, updatedAt: T2 });

    expect(repo.listRecent(1)).toHaveLength(1);
    expect(repo.listRecent(1)[0]!.file.id).toBe(files[1]!.id);
  });

  it('borra el estado en cascada al eliminar el título', () => {
    const repo = createPlaybackRepository(db);
    const mediaRepo = createMediaRepository(db);
    const { media, files } = seed();
    repo.save({ mediaFileId: files[0]!.id, positionSeconds: 10, lastPlayedAt: T1, updatedAt: T1 });

    mediaRepo.remove(media.id);

    expect(repo.count()).toBe(0);
  });
});
