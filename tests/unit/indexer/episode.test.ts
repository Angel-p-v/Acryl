import { describe, expect, it } from 'vitest';

import { parseEpisode, type EpisodeInfo } from '../../../src/core/indexer/parsers/episode.js';

type Case = {
  file: string;
  expected: EpisodeInfo | null;
};

const cases: Case[] = [
  { file: 'Severance.S01E02.1080p.mkv', expected: { season: 1, episode: 2 } },
  { file: 'severance_s01e02.mkv', expected: { season: 1, episode: 2 } },
  { file: 'Show.S01.E02.mkv', expected: { season: 1, episode: 2 } },
  { file: 'Show S01 E02.mkv', expected: { season: 1, episode: 2 } },
  { file: 'Show.S01EP02.mkv', expected: { season: 1, episode: 2 } },
  { file: 'Show.S01-E02.mkv', expected: { season: 1, episode: 2 } },
  { file: 'Show.1x02.mkv', expected: { season: 1, episode: 2 } },
  { file: 'Show.01x02.720p.mkv', expected: { season: 1, episode: 2 } },
  { file: 'Show.Season 1 Episode 2.mkv', expected: { season: 1, episode: 2 } },
  { file: 'Show.Season.1.Ep.2.mkv', expected: { season: 1, episode: 2 } },
  { file: 'Show.T1E2.mkv', expected: { season: 1, episode: 2 } },
  { file: 'Show.T02E15.mkv', expected: { season: 2, episode: 15 } },
  { file: 'Show.E03.mkv', expected: { season: null, episode: 3 } },
  { file: 'Show.Ep.12.mkv', expected: { season: null, episode: 12 } },
  { file: 'Show.Episode 7 1080p.mkv', expected: { season: null, episode: 7 } },
  { file: '[SubsPlease] Frieren - 03 (1080p).mkv', expected: { season: null, episode: 3 } },
  { file: 'Frieren - 03 - The Journey.mkv', expected: { season: null, episode: 3 } },
  { file: 'Frieren - 07v2 [1080p].mkv', expected: { season: null, episode: 7 } },
  { file: 'Show.S00E01.mkv', expected: { season: 0, episode: 1 } },
  { file: 'Show.S01E00.mkv', expected: { season: 1, episode: 0 } },
  { file: 'Show.S01E02.1x03.mkv', expected: { season: 1, episode: 2 } },
  { file: 'Movie.2019.1080p.mkv', expected: null },
  { file: 'Show.1920x1080.mkv', expected: null },
  { file: 'Show - 1080p.mkv', expected: null },
  { file: 'Show.S100E02.mkv', expected: null },
  { file: 'Show.mkv', expected: null },
];

describe('parseEpisode', () => {
  it.each(cases)('interpreta "$file" como $expected', ({ file, expected }) => {
    expect(parseEpisode(file)).toEqual(expected);
  });

  it('no lanza excepciones con nombres vacíos o raros', () => {
    expect(() => parseEpisode('')).not.toThrow();
    expect(() => parseEpisode('.mkv')).not.toThrow();
    expect(parseEpisode('')).toBeNull();
  });

  it('acepta la ruta completa y solo mira el nombre del fichero', () => {
    expect(parseEpisode('/media/series/Show/Season 2/Show.S02E05.mkv')).toEqual({
      season: 2,
      episode: 5,
    });
  });
});
