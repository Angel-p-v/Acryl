import { describe, expect, it } from 'vitest';

import { classifyFile } from '../../../src/core/indexer/classifier.js';
import { isVideoFile } from '../../../src/core/indexer/video.js';

import type { LibraryRoot } from '../../../src/core/config/schema.js';

const seriesRoot: LibraryRoot = { path: '/media/series', type: 'series' };
const animeRoot: LibraryRoot = { path: '/media/anime', type: 'anime' };
const movieRoot: LibraryRoot = { path: '/media/movies', type: 'movie' };

function classify(root: LibraryRoot, filePath: string): ReturnType<typeof classifyFile> {
  return classifyFile({ root, filePath, sizeBytes: 1000, mtimeMs: 1_700_000_000_000 });
}

describe('classifyFile', () => {
  it('asigna un episodio a la carpeta del título en una raíz de series', () => {
    expect(
      classify(seriesRoot, '/media/series/Severance (2022)/Season 1/Severance.S01E02.mkv'),
    ).toEqual({
      type: 'series',
      title: 'Severance',
      year: 2022,
      folderPath: '/media/series/Severance (2022)',
      filePath: '/media/series/Severance (2022)/Season 1/Severance.S01E02.mkv',
      sizeBytes: 1000,
      mtimeMs: 1_700_000_000_000,
      season: 1,
      episode: 2,
    });
  });

  it('usa la numeración absoluta en anime', () => {
    const result = classify(animeRoot, '/media/anime/Frieren/[SubsPlease] Frieren - 03.mkv');

    expect(result).toMatchObject({
      type: 'anime',
      title: 'Frieren',
      year: null,
      folderPath: '/media/anime/Frieren',
      season: null,
      episode: 3,
    });
  });

  it('trata cada película como un título independiente', () => {
    const result = classify(movieRoot, '/media/movies/Arrival.2016.1080p.mkv');

    expect(result).toMatchObject({
      type: 'movie',
      title: 'Arrival',
      year: 2016,
      folderPath: '/media/movies/Arrival.2016.1080p.mkv',
      season: null,
      episode: null,
    });
  });

  it('no fusiona dos películas de la misma carpeta', () => {
    const first = classify(movieRoot, '/media/movies/Arrival.2016.mkv');
    const second = classify(movieRoot, '/media/movies/Heat.1995.mkv');

    expect(first?.folderPath).not.toBe(second?.folderPath);
  });

  it('clasifica un fichero suelto con patrón de episodio en raíz de series', () => {
    expect(classify(seriesRoot, '/media/series/Severance.S01E02.mkv')).toMatchObject({
      type: 'series',
      title: 'Severance',
      season: 1,
      episode: 2,
    });
  });

  it('clasifica un fichero con sufijo v2 tras el episodio (S01E01v2)', () => {
    expect(classify(seriesRoot, '/media/series/Black Torch - S01E01v2.mkv')).toMatchObject({
      type: 'series',
      title: 'Black Torch',
      season: 1,
      episode: 1,
    });
  });

  it('clasifica un fichero con sufijo v2 en raíz de movies como serie', () => {
    expect(classify(movieRoot, '/media/movies/Black Torch - S01E01v2.mkv')).toMatchObject({
      type: 'series',
      title: 'Black Torch',
      season: 1,
      episode: 1,
    });
  });

  it('clasifica un fichero sin patrón de episodio como unknown', () => {
    expect(classify(seriesRoot, '/media/series/Severance/extra.mkv')).toMatchObject({
      type: 'unknown',
      title: 'Severance',
    });
  });

  it('clasifica un fichero sin título utilizable como unknown', () => {
    expect(classify(movieRoot, '/media/movies/1080p.mkv')).toMatchObject({
      type: 'unknown',
      title: '1080p',
    });
  });

  it('descarta rutas fuera de la raíz', () => {
    expect(classify(seriesRoot, '/media/movies/Arrival.2016.mkv')).toBeNull();
    expect(classify(seriesRoot, '/media/series')).toBeNull();
  });
});

describe('isVideoFile', () => {
  it.each(['a.mkv', 'a.MP4', 'a.avi', 'a.webm', 'a.m2ts'])('acepta %s', (file) => {
    expect(isVideoFile(file)).toBe(true);
  });

  it.each(['a.txt', 'a.srt', 'a', 'a.mkv.bak'])('rechaza %s', (file) => {
    expect(isVideoFile(file)).toBe(false);
  });
});
