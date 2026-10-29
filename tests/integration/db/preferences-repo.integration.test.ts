import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { closeDatabase, openDatabase } from '../../../src/core/db/connection.js';
import { createMediaRepository } from '../../../src/core/db/repositories/mediaRepo.js';
import { createPreferenceRepository } from '../../../src/core/db/repositories/preferencesRepo.js';

import type { Db } from '../../../src/core/db/connection.js';
import type { MediaRow } from '../../../src/core/db/types.js';

const T1 = '2026-10-23T10:00:00.000Z';
const T2 = '2026-10-23T11:00:00.000Z';

describe('repositorio de preferencias (integración)', () => {
  let db: Db;
  let media: MediaRow;
  let other: MediaRow;

  beforeEach(() => {
    db = openDatabase(':memory:');
    const mediaRepo = createMediaRepository(db);
    media = mediaRepo.upsert({
      type: 'series',
      title: 'Severance',
      year: 2022,
      folderPath: '/library/Severance',
    });
    other = mediaRepo.upsert({
      type: 'movie',
      title: 'Arrival',
      year: 2016,
      folderPath: '/library/Arrival',
    });
  });

  afterEach(() => {
    closeDatabase(db);
  });

  it('trata como no favorito lo que no tiene fila', () => {
    const repo = createPreferenceRepository(db);

    expect(repo.get(media.id)).toBeUndefined();
    expect(repo.isFavorite(media.id)).toBe(false);
    expect(repo.countFavorites()).toBe(0);
  });

  it('marca y desmarca un favorito conservando la fecha de creación', () => {
    const repo = createPreferenceRepository(db);

    const created = repo.setFavorite({ mediaId: media.id, favorite: true, updatedAt: T1 });
    expect(created).toEqual({
      mediaId: media.id,
      favorite: true,
      createdAt: T1,
      updatedAt: T1,
    });
    expect(repo.isFavorite(media.id)).toBe(true);

    const updated = repo.setFavorite({ mediaId: media.id, favorite: false, updatedAt: T2 });
    expect(updated.favorite).toBe(false);
    expect(updated.createdAt).toBe(T1);
    expect(updated.updatedAt).toBe(T2);
    expect(repo.isFavorite(media.id)).toBe(false);
  });

  it('respeta createdAt cuando se indica aparte', () => {
    const repo = createPreferenceRepository(db);

    const row = repo.setFavorite({
      mediaId: media.id,
      favorite: true,
      createdAt: T1,
      updatedAt: T2,
    });

    expect(row.createdAt).toBe(T1);
    expect(row.updatedAt).toBe(T2);
  });

  it('lista y cuenta solo los favoritos, del más reciente al más antiguo', () => {
    const repo = createPreferenceRepository(db);
    repo.setFavorite({ mediaId: media.id, favorite: true, updatedAt: T1 });
    repo.setFavorite({ mediaId: other.id, favorite: true, updatedAt: T2 });
    repo.setFavorite({ mediaId: media.id, favorite: false, updatedAt: T2 });

    expect(repo.countFavorites()).toBe(1);
    expect(repo.listFavorites().map((row) => row.mediaId)).toEqual([other.id]);
    expect(repo.listFavorites(0)).toHaveLength(0);
  });

  it('elimina la preferencia y la borra en cascada con el título', () => {
    const repo = createPreferenceRepository(db);
    const mediaRepo = createMediaRepository(db);
    repo.setFavorite({ mediaId: media.id, favorite: true, updatedAt: T1 });
    repo.setFavorite({ mediaId: other.id, favorite: true, updatedAt: T1 });

    expect(repo.remove(media.id)).toBe(true);
    expect(repo.remove(media.id)).toBe(false);

    mediaRepo.remove(other.id);
    expect(repo.countFavorites()).toBe(0);
  });
});
