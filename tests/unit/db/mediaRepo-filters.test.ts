import { describe, expect, it, vi, beforeAll, afterAll } from 'vitest';

import { openDatabase } from '../../../src/core/db/connection.js';
import { createMediaRepository } from '../../../src/core/db/repositories/mediaRepo.js';
import { createTagRepository } from '../../../src/core/db/repositories/tagRepo.js';

import type { Db } from '../../../src/core/db/connection.js';
import type { MediaRepository } from '../../../src/core/db/repositories/mediaRepo.js';
import type { PlaybackRepository } from '../../../src/core/db/repositories/playbackRepo.js';
import type { PreferenceRepository } from '../../../src/core/db/repositories/preferencesRepo.js';
import type { TagRepository } from '../../../src/core/db/repositories/tagRepo.js';

function createTestDb(): Db {
  const db = openDatabase(':memory:');
  db.pragma('journal_mode = WAL');
  db.pragma('foreign_keys = ON');
  db.pragma('busy_timeout = 5000');
  db.exec(`
    CREATE TABLE IF NOT EXISTS media (
      id INTEGER PRIMARY KEY,
      type TEXT NOT NULL,
      title TEXT NOT NULL,
      sort_title TEXT NOT NULL,
      year INTEGER,
      folder_path TEXT NOT NULL UNIQUE,
      created_at TEXT NOT NULL DEFAULT (datetime('now')),
      updated_at TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE TABLE IF NOT EXISTS media_files (
      id INTEGER PRIMARY KEY,
      media_id INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE,
      file_path TEXT NOT NULL,
      season INTEGER,
      episode INTEGER,
      title TEXT,
      size_bytes INTEGER,
      mtime_ms INTEGER,
      indexed_at TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE TABLE IF NOT EXISTS playback_state (
      media_file_id INTEGER PRIMARY KEY REFERENCES media_files(id) ON DELETE CASCADE,
      position_seconds REAL NOT NULL DEFAULT 0,
      duration_seconds REAL,
      completed_at TEXT,
      last_played_at TEXT NOT NULL,
      play_count INTEGER NOT NULL DEFAULT 0,
      updated_at TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS media_preferences (
      media_id INTEGER PRIMARY KEY REFERENCES media(id) ON DELETE CASCADE,
      favorite INTEGER NOT NULL DEFAULT 0 CHECK (favorite IN (0, 1)),
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_media_type ON media(type);
    CREATE INDEX IF NOT EXISTS idx_media_sort_title ON media(sort_title);
    CREATE INDEX IF NOT EXISTS idx_media_created_at ON media(created_at);
    CREATE INDEX IF NOT EXISTS idx_media_updated_at ON media(updated_at);
    CREATE INDEX IF NOT EXISTS idx_media_files_media_id ON media_files(media_id);
    CREATE INDEX IF NOT EXISTS idx_playback_state_last_played_at ON playback_state(last_played_at);
    CREATE INDEX IF NOT EXISTS idx_playback_state_completed_at ON playback_state(completed_at);
    CREATE INDEX IF NOT EXISTS idx_media_preferences_favorite ON media_preferences(favorite);
    CREATE TABLE IF NOT EXISTS tags (
      id INTEGER PRIMARY KEY,
      name TEXT NOT NULL UNIQUE,
      color TEXT,
      created_at TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE TABLE IF NOT EXISTS media_tags (
      media_id INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE,
      tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
      PRIMARY KEY (media_id, tag_id)
    );
    CREATE INDEX IF NOT EXISTS idx_media_tags_tag ON media_tags(tag_id);
    CREATE INDEX IF NOT EXISTS idx_media_tags_media ON media_tags(media_id);
    CREATE VIRTUAL TABLE IF NOT EXISTS media_fts USING fts5(title, content='media', content_rowid='id');
    CREATE TRIGGER IF NOT EXISTS media_fts_ai AFTER INSERT ON media BEGIN
      INSERT INTO media_fts(rowid, title) VALUES (new.id, new.title);
    END;
    CREATE TRIGGER IF NOT EXISTS media_fts_ad AFTER DELETE ON media BEGIN
      INSERT INTO media_fts(media_fts, rowid, title) VALUES ('delete', old.id, old.title);
    END;
    CREATE TRIGGER IF NOT EXISTS media_fts_au AFTER UPDATE ON media BEGIN
      INSERT INTO media_fts(media_fts, rowid, title) VALUES ('delete', old.id, old.title);
      INSERT INTO media_fts(rowid, title) VALUES (new.id, new.title);
    END;
  `);
  return db;
}

function setupTestData(db: Db) {
  const insertMedia = db.prepare(`
    INSERT INTO media (type, title, sort_title, year, folder_path)
    VALUES (@type, @title, @sort_title, @year, @folder_path)
  `);

  const insertFile = db.prepare(`
    INSERT INTO media_files (media_id, file_path, season, episode)
    VALUES (@media_id, @file_path, @season, @episode)
  `);

  const insertPlayback = db.prepare(`
    INSERT INTO playback_state (media_file_id, position_seconds, duration_seconds, completed_at, last_played_at, play_count, updated_at)
    VALUES (@media_file_id, @position_seconds, @duration_seconds, @completed_at, @last_played_at, @play_count, @updated_at)
  `);

  const insertPreference = db.prepare(`
    INSERT INTO media_preferences (media_id, favorite, created_at, updated_at)
    VALUES (@media_id, @favorite, @created_at, @updated_at)
  `);

  const now = new Date().toISOString();

  // Media 1: series, favorite, in progress (not completed)
  const m1 = insertMedia.run({
    type: 'series',
    title: 'Series A',
    sort_title: 'series a',
    year: 2020,
    folder_path: '/a',
  });
  const f1 = insertFile.run({
    media_id: m1.lastInsertRowid,
    file_path: '/a/ep1.mkv',
    season: 1,
    episode: 1,
  });
  insertPlayback.run({
    media_file_id: f1.lastInsertRowid,
    position_seconds: 100,
    duration_seconds: 3600,
    completed_at: null,
    last_played_at: now,
    play_count: 1,
    updated_at: now,
  });
  insertPreference.run({
    media_id: m1.lastInsertRowid,
    favorite: 1,
    created_at: now,
    updated_at: now,
  });

  // Media 2: series, favorite, completed
  const m2 = insertMedia.run({
    type: 'series',
    title: 'Series B',
    sort_title: 'series b',
    year: 2021,
    folder_path: '/b',
  });
  const f2 = insertFile.run({
    media_id: m2.lastInsertRowid,
    file_path: '/b/ep1.mkv',
    season: 1,
    episode: 1,
  });
  insertPlayback.run({
    media_file_id: f2.lastInsertRowid,
    position_seconds: 3600,
    duration_seconds: 3600,
    completed_at: now,
    last_played_at: now,
    play_count: 1,
    updated_at: now,
  });
  insertPreference.run({
    media_id: m2.lastInsertRowid,
    favorite: 1,
    created_at: now,
    updated_at: now,
  });

  // Media 3: anime, not favorite, in progress
  const m3 = insertMedia.run({
    type: 'anime',
    title: 'Anime C',
    sort_title: 'anime c',
    year: 2022,
    folder_path: '/c',
  });
  const f3 = insertFile.run({
    media_id: m3.lastInsertRowid,
    file_path: '/c/ep1.mkv',
    season: 1,
    episode: 1,
  });
  insertPlayback.run({
    media_file_id: f3.lastInsertRowid,
    position_seconds: 500,
    duration_seconds: 3600,
    completed_at: null,
    last_played_at: now,
    play_count: 1,
    updated_at: now,
  });

  // Media 4: movie, not favorite, completed
  const m4 = insertMedia.run({
    type: 'movie',
    title: 'Movie D',
    sort_title: 'movie d',
    year: 2023,
    folder_path: '/d',
  });
  const f4 = insertFile.run({
    media_id: m4.lastInsertRowid,
    file_path: '/d/movie.mkv',
    season: null,
    episode: null,
  });
  insertPlayback.run({
    media_file_id: f4.lastInsertRowid,
    position_seconds: 7200,
    duration_seconds: 7200,
    completed_at: now,
    last_played_at: now,
    play_count: 1,
    updated_at: now,
  });

  // Media 5: series, not favorite, never played (unwatched)
  const m5 = insertMedia.run({
    type: 'series',
    title: 'Series E',
    sort_title: 'series e',
    year: 2024,
    folder_path: '/e',
  });
  const f5 = insertFile.run({
    media_id: m5.lastInsertRowid,
    file_path: '/e/ep1.mkv',
    season: 1,
    episode: 1,
  });

  // Media 6: anime, favorite, never played
  const m6 = insertMedia.run({
    type: 'anime',
    title: 'Anime F',
    sort_title: 'anime f',
    year: 2024,
    folder_path: '/f',
  });
  const f6 = insertFile.run({
    media_id: m6.lastInsertRowid,
    file_path: '/f/ep1.mkv',
    season: 1,
    episode: 1,
  });
  insertPreference.run({
    media_id: m6.lastInsertRowid,
    favorite: 1,
    created_at: now,
    updated_at: now,
  });

  // Tags: tag 1 = "favorito" (m1, m2, m6), tag 2 = "por-ver" (m3, m5)
  const insertTag = db.prepare(
    `INSERT INTO tags (name, color, created_at) VALUES (@name, @color, @created_at)`,
  );
  const insertMediaTag = db.prepare(
    `INSERT INTO media_tags (media_id, tag_id) VALUES (@media_id, @tag_id)`,
  );

  const t1 = insertTag.run({ name: 'favorito', color: '#ffd700', created_at: now });
  const t2 = insertTag.run({ name: 'por-ver', color: '#ff6b6b', created_at: now });

  for (const mid of [m1.lastInsertRowid, m2.lastInsertRowid, m6.lastInsertRowid]) {
    insertMediaTag.run({ media_id: mid, tag_id: t1.lastInsertRowid });
  }
  for (const mid of [m3.lastInsertRowid, m5.lastInsertRowid]) {
    insertMediaTag.run({ media_id: mid, tag_id: t2.lastInsertRowid });
  }

  db.pragma('optimize');
  return { m1, m2, m3, m4, m5, m6, f1, f2, f3, f4, f5, f6 };
}

describe('MediaRepository filters', () => {
  let db: Db;
  let mediaRepo: MediaRepository;
  let playbackRepo: PlaybackRepository;
  let preferenceRepo: PreferenceRepository;
  let tagRepo: TagRepository;

  beforeAll(() => {
    db = createTestDb();
    const data = setupTestData(db);
    playbackRepo = {
      listInProgress: vi.fn(() => [
        {
          media: { id: data.m1.lastInsertRowid },
          state: { completedAt: null },
          file: {
            id: 0,
            mediaId: 0,
            filePath: '',
            season: null,
            episode: null,
            title: null,
            sizeBytes: null,
            mtimeMs: null,
            indexedAt: '',
          },
        },
        {
          media: { id: data.m3.lastInsertRowid },
          state: { completedAt: null },
          file: {
            id: 0,
            mediaId: 0,
            filePath: '',
            season: null,
            episode: null,
            title: null,
            sizeBytes: null,
            mtimeMs: null,
            indexedAt: '',
          },
        },
      ]) as unknown as PlaybackRepository['listInProgress'],
      listRecent: vi.fn(() => [
        {
          media: { id: data.m1.lastInsertRowid },
          state: { completedAt: null },
          file: {
            id: 0,
            mediaId: 0,
            filePath: '',
            season: null,
            episode: null,
            title: null,
            sizeBytes: null,
            mtimeMs: null,
            indexedAt: '',
          },
        },
        {
          media: { id: data.m2.lastInsertRowid },
          state: { completedAt: new Date().toISOString() },
          file: {
            id: 0,
            mediaId: 0,
            filePath: '',
            season: null,
            episode: null,
            title: null,
            sizeBytes: null,
            mtimeMs: null,
            indexedAt: '',
          },
        },
        {
          media: { id: data.m3.lastInsertRowid },
          state: { completedAt: null },
          file: {
            id: 0,
            mediaId: 0,
            filePath: '',
            season: null,
            episode: null,
            title: null,
            sizeBytes: null,
            mtimeMs: null,
            indexedAt: '',
          },
        },
        {
          media: { id: data.m4.lastInsertRowid },
          state: { completedAt: new Date().toISOString() },
          file: {
            id: 0,
            mediaId: 0,
            filePath: '',
            season: null,
            episode: null,
            title: null,
            sizeBytes: null,
            mtimeMs: null,
            indexedAt: '',
          },
        },
      ]) as unknown as PlaybackRepository['listRecent'],
      count: vi.fn(() => 4),
      save: vi.fn(),
      get: vi.fn(),
      remove: vi.fn(),
    };
    preferenceRepo = {
      listFavorites: vi.fn(() => [
        {
          mediaId: data.m1.lastInsertRowid,
          favorite: true,
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
        {
          mediaId: data.m2.lastInsertRowid,
          favorite: true,
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
        {
          mediaId: data.m6.lastInsertRowid,
          favorite: true,
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
      ]) as unknown as PreferenceRepository['listFavorites'],
      countFavorites: vi.fn(() => 3),
      get: vi.fn(),
      isFavorite: vi.fn(),
      setFavorite: vi.fn(),
      remove: vi.fn(),
    };
    tagRepo = createTagRepository(db);
    mediaRepo = createMediaRepository(db, { playbackRepo, preferenceRepo, tagRepo });
  }, 60_000);

  afterAll(() => {
    db.close();
  });

  describe('favorites filter', () => {
    it('returns only favorites when favorites=true', () => {
      const results = mediaRepo.list({ favorites: true });
      const titles = results.map((r) => r.title).sort();
      expect(titles).toEqual(['Anime F', 'Series A', 'Series B']);
    });

    it('returns all when favorites=false/undefined', () => {
      const results = mediaRepo.list({ favorites: false });
      expect(results.length).toBe(6);
    });

    it('returns empty when no favorites exist', () => {
      const noFavRepo = createMediaRepository(db, {
        playbackRepo,
        preferenceRepo: {
          listFavorites: vi.fn(() => []),
          countFavorites: vi.fn(() => 0),
        },
      });
      const results = noFavRepo.list({ favorites: true });
      expect(results).toHaveLength(0);
    });

    it('combines with type filter', () => {
      const results = mediaRepo.list({ favorites: true, type: 'series' });
      const titles = results.map((r) => r.title).sort();
      expect(titles).toEqual(['Series A', 'Series B']);
    });
  });

  describe('inProgress filter', () => {
    it('returns only in-progress titles when inProgress=true', () => {
      const results = mediaRepo.list({ inProgress: true });
      const titles = results.map((r) => r.title).sort();
      expect(titles).toEqual(['Anime C', 'Series A']);
    });

    it('returns all when inProgress=false/undefined', () => {
      const results = mediaRepo.list({ inProgress: false });
      expect(results.length).toBe(6);
    });

    it('returns empty when no in-progress exist', () => {
      const noProgressRepo = createMediaRepository(db, {
        playbackRepo: {
          listInProgress: vi.fn(() => []),
          listRecent: vi.fn(() => []),
          count: vi.fn(() => 0),
        },
        preferenceRepo,
      });
      const results = noProgressRepo.list({ inProgress: true });
      expect(results).toHaveLength(0);
    });

    it('combines with type filter', () => {
      const results = mediaRepo.list({ inProgress: true, type: 'anime' });
      const titles = results.map((r) => r.title);
      expect(titles).toEqual(['Anime C']);
    });
  });

  describe('watched filter', () => {
    it('returns only completed titles when watched=true', () => {
      const results = mediaRepo.list({ watched: true });
      const titles = results.map((r) => r.title).sort();
      expect(titles).toEqual(['Movie D', 'Series B']);
    });

    it('returns all when watched=false/undefined', () => {
      const results = mediaRepo.list({ watched: false });
      expect(results.length).toBe(6);
    });

    it('returns empty when no completed exist', () => {
      // Create a fresh database without completed items
      const freshDb = createTestDb();
      const insertMedia = freshDb.prepare(`
        INSERT INTO media (type, title, sort_title, year, folder_path)
        VALUES (@type, @title, @sort_title, @year, @folder_path)
      `);
      const insertFile = freshDb.prepare(`
        INSERT INTO media_files (media_id, file_path, season, episode)
        VALUES (@media_id, @file_path, @season, @episode)
      `);
      const insertPlayback = freshDb.prepare(`
        INSERT INTO playback_state (media_file_id, position_seconds, duration_seconds, completed_at, last_played_at, play_count, updated_at)
        VALUES (@media_file_id, @position_seconds, @duration_seconds, @completed_at, @last_played_at, @play_count, @updated_at)
      `);
      const now = new Date().toISOString();

      // Add media with in-progress playback (no completed_at)
      const m1 = insertMedia.run({
        type: 'series',
        title: 'Test Series',
        sort_title: 'test series',
        year: 2024,
        folder_path: '/test1',
      });
      const f1 = insertFile.run({
        media_id: m1.lastInsertRowid,
        file_path: '/test1/ep1.mkv',
        season: 1,
        episode: 1,
      });
      insertPlayback.run({
        media_file_id: f1.lastInsertRowid,
        position_seconds: 100,
        duration_seconds: 3600,
        completed_at: null,
        last_played_at: now,
        play_count: 1,
        updated_at: now,
      });

      freshDb.pragma('optimize');

      const noCompleteRepo = createMediaRepository(freshDb, {
        playbackRepo: {
          listInProgress: vi.fn(() => []),
          listRecent: vi.fn(() => []),
          count: vi.fn(() => 0),
        },
        preferenceRepo: {
          listFavorites: vi.fn(() => []),
          countFavorites: vi.fn(() => 0),
        },
      });
      const results = noCompleteRepo.list({ watched: true });
      expect(results).toHaveLength(0);
    });

    it('combines with type filter', () => {
      const results = mediaRepo.list({ watched: true, type: 'movie' });
      expect(results.map((r) => r.title)).toEqual(['Movie D']);
    });
  });

  describe('unwatched filter', () => {
    it('returns only unwatched titles when unwatched=true', () => {
      const results = mediaRepo.list({ unwatched: true });
      const titles = results.map((r) => r.title).sort();
      // Series A (in progress, not completed), Anime C (in progress), Series E (never played), Anime F (never played)
      expect(titles).toEqual(['Anime C', 'Anime F', 'Series A', 'Series E']);
    });

    it('returns all when unwatched=false/undefined', () => {
      const results = mediaRepo.list({ unwatched: false });
      expect(results.length).toBe(6);
    });

    it('combines with type filter', () => {
      const results = mediaRepo.list({ unwatched: true, type: 'anime' });
      const titles = results.map((r) => r.title).sort();
      expect(titles).toEqual(['Anime C', 'Anime F']);
    });
  });

  describe('combined filters', () => {
    it('favorites + inProgress', () => {
      const results = mediaRepo.list({ favorites: true, inProgress: true });
      // Series A is both favorite and in progress
      expect(results.map((r) => r.title)).toEqual(['Series A']);
    });

    it('favorites + watched', () => {
      const results = mediaRepo.list({ favorites: true, watched: true });
      expect(results.map((r) => r.title).sort()).toEqual(['Series B']);
    });

    it('favorites + unwatched', () => {
      const results = mediaRepo.list({ favorites: true, unwatched: true });
      expect(results.map((r) => r.title).sort()).toEqual(['Anime F', 'Series A']);
    });

    it('type + favorites + inProgress', () => {
      const results = mediaRepo.list({ type: 'series', favorites: true, inProgress: true });
      expect(results.map((r) => r.title)).toEqual(['Series A']);
    });
  });

  describe('chunking / parameter limit', () => {
    it('handles >999 IDs via chunking (internal test via large fav set)', () => {
      // Use IDs that don't exist in the database (10000-11499) to verify chunking works
      const manyFavRepo = createMediaRepository(db, {
        playbackRepo,
        preferenceRepo: {
          listFavorites: vi.fn(() =>
            Array.from({ length: 1500 }, (_, i) => ({ mediaId: 10000 + i })),
          ),
          countFavorites: vi.fn(() => 1500),
        },
      });
      // Should not throw and should handle chunking internally
      const results = manyFavRepo.list({ favorites: true });
      // Results will be empty since those IDs don't exist in the database
      expect(results).toHaveLength(0);
    });
  });

  describe('missing dependencies', () => {
    it('favorites filter returns empty when preferenceRepo missing', () => {
      const noPrefRepo = createMediaRepository(db, { playbackRepo });
      const results = noPrefRepo.list({ favorites: true });
      expect(results).toHaveLength(0);
    });

    it('inProgress filter returns empty when playbackRepo missing', () => {
      const noPlaybackRepo = createMediaRepository(db, { preferenceRepo });
      const results = noPlaybackRepo.list({ inProgress: true });
      expect(results).toHaveLength(0);
    });

    it('watched filter returns empty when playbackRepo missing', () => {
      const noPlaybackRepo = createMediaRepository(db, { preferenceRepo });
      const results = noPlaybackRepo.list({ watched: true });
      expect(results).toHaveLength(0);
    });

    it('unwatched filter returns empty when playbackRepo missing', () => {
      const noPlaybackRepo = createMediaRepository(db, { preferenceRepo });
      const results = noPlaybackRepo.list({ unwatched: true });
      expect(results).toHaveLength(0);
    });
  });

  describe('pagination with filters', () => {
    it('respects limit and offset with favorites', () => {
      const page1 = mediaRepo.list({ favorites: true, limit: 2, offset: 0 });
      const page2 = mediaRepo.list({ favorites: true, limit: 2, offset: 2 });
      expect(page1.length).toBe(2);
      expect(page2.length).toBe(1);
      const allIds = [...page1, ...page2].map((r) => r.id).sort((a, b) => a - b);
      expect(new Set(allIds).size).toBe(3);
    });
  });

  describe('tags filter', () => {
    it('returns titles that have the specified tag', () => {
      const results = mediaRepo.list({ tags: [1] });
      const titles = results.map((r) => r.title).sort();
      expect(titles).toEqual(['Anime F', 'Series A', 'Series B']);
    });

    it('returns empty when no media has the tag', () => {
      const results = mediaRepo.list({ tags: [999] });
      expect(results).toHaveLength(0);
    });

    it('returns intersection when multiple tags specified (AND)', () => {
      // No media has both tag 1 (favorito) and tag 2 (por-ver)
      const results = mediaRepo.list({ tags: [1, 2] });
      expect(results).toHaveLength(0);
    });

    it('returns empty when tagRepo is missing', () => {
      const noTagRepo = createMediaRepository(db, { playbackRepo, preferenceRepo });
      const results = noTagRepo.list({ tags: [1] });
      expect(results).toHaveLength(0);
    });

    it('combines with type filter', () => {
      const results = mediaRepo.list({ tags: [1], type: 'series' });
      const titles = results.map((r) => r.title).sort();
      expect(titles).toEqual(['Series A', 'Series B']);
    });
  });
});
