import { afterEach, describe, expect, it, vi } from 'vitest';

import { parseListOptions } from '../../../src/commands/list.js';
import {
  formatBytes,
  formatEpisode,
  mediaTypeLabel,
  printMediaList,
  toMediaFileJson,
  toMediaJson,
} from '../../../src/commands/output.js';

import type { MediaFileRow, MediaRow } from '../../../src/core/db/types.js';

const movie: MediaRow = {
  id: 7,
  type: 'movie',
  title: 'Arrival',
  sortTitle: 'arrival',
  year: 2016,
  folderPath: '/media/movies/Arrival.mkv',
  createdAt: '2026-01-01 00:00:00',
  updatedAt: '2026-01-01 00:00:00',
};

const file: MediaFileRow = {
  id: 11,
  mediaId: 7,
  season: 1,
  episode: 2,
  title: null,
  filePath: '/media/movies/Arrival.mkv',
  sizeBytes: 1024,
  mtimeMs: 1,
  indexedAt: '2026-01-01 00:00:00',
};

describe('salida de la CLI', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('proyecta títulos y ficheros a JSON estable', () => {
    expect(toMediaJson(movie)).toEqual({
      id: 7,
      type: 'movie',
      title: 'Arrival',
      year: 2016,
      folderPath: '/media/movies/Arrival.mkv',
    });
    expect(toMediaFileJson(file)).toEqual({
      id: 11,
      season: 1,
      episode: 2,
      filePath: '/media/movies/Arrival.mkv',
      sizeBytes: 1024,
    });
  });

  it('formatea temporada y episodio', () => {
    expect(formatEpisode({ season: 1, episode: 2 })).toBe('T01E02');
    expect(formatEpisode({ season: null, episode: 3 })).toBe('E03');
    expect(formatEpisode({ season: 1, episode: null })).toBe('—');
  });

  it('traduce el tipo de contenido a una etiqueta legible', () => {
    expect(mediaTypeLabel('series')).toBe('serie');
    expect(mediaTypeLabel('anime')).toBe('anime');
    expect(mediaTypeLabel('movie')).toBe('película');
  });

  it.each([
    [0, '0 B'],
    [1023, '1023 B'],
    [1024, '1.0 KiB'],
    [10 * 1024, '10 KiB'],
    [1024 * 1024, '1.0 MiB'],
    [3 * 1024 * 1024 * 1024, '3.0 GiB'],
  ])('formatea %d bytes como %s', (bytes, expected) => {
    expect(formatBytes(bytes)).toBe(expected);
  });

  it('imprime los títulos con su año y su tipo', () => {
    const lines: string[] = [];
    vi.spyOn(console, 'log').mockImplementation((...args: unknown[]) => {
      lines.push(args.map((arg) => String(arg)).join(' '));
    });

    printMediaList([movie, { ...movie, id: 8, type: 'series', title: 'Dark', year: null }]);

    expect(lines[0]).toContain('Arrival');
    expect(lines[0]).toContain('(2016)');
    expect(lines[1]).toContain('Dark');
    expect(lines[1]).not.toContain('(null)');
  });

  it('normaliza las opciones de listado y omite las ausentes', () => {
    expect(parseListOptions({ type: 'series', sort: 'recent', limit: 10, offset: 5 })).toEqual({
      type: 'series',
      sort: 'recent',
      limit: 10,
      offset: 5,
    });
    expect(parseListOptions({})).toEqual({});
  });
});
