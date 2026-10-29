import { describe, expect, it } from 'vitest';

import {
  applyStartPlaceholder,
  calculatePlayback,
  findNextEpisode,
  isEpisodic,
  nowIso,
  resolveResumeDecision,
} from '../../../src/core/playback/index.js';

import type { PlaybackConfig } from '../../../src/core/config/schema.js';
import type { MediaFileRow, PlaybackStateRow } from '../../../src/core/db/types.js';

const baseConfig: PlaybackConfig = {
  tracking: 'auto',
  resume: true,
  resumeMinSeconds: 30,
  completedThreshold: 0.9,
  completedTailSeconds: 30,
} as const satisfies PlaybackConfig;

const noResumeConfig: PlaybackConfig = { ...baseConfig, resume: false };

const baseNow = '2026-10-23T10:00:00.000Z';

function makeState(overrides: Partial<PlaybackStateRow> = {}): PlaybackStateRow {
  return {
    mediaFileId: 1,
    positionSeconds: 0,
    durationSeconds: null,
    completedAt: null,
    lastPlayedAt: baseNow,
    playCount: 0,
    updatedAt: baseNow,
    ...overrides,
  };
}

function makeFile(overrides: Partial<MediaFileRow> = {}): MediaFileRow {
  return {
    id: 1,
    mediaId: 1,
    season: 1,
    episode: 1,
    title: null,
    filePath: '/library/Show/S01E01.mkv',
    sizeBytes: null,
    mtimeMs: null,
    indexedAt: baseNow,
    ...overrides,
  };
}

describe('playback domain', () => {
  describe('calculatePlayback', () => {
    it('sin estado devuelve valores por defecto', () => {
      const calc = calculatePlayback(undefined, baseConfig, baseNow);
      expect(calc.positionSeconds).toBe(0);
      expect(calc.isCompleted).toBe(false);
      expect(calc.canResume).toBe(false);
    });

    it('marca completado por umbral (90 %)', () => {
      const state = makeState({ positionSeconds: 2700, durationSeconds: 3000 });
      expect(calculatePlayback(state, baseConfig, baseNow).isCompleted).toBe(true);
    });

    it('marca completado por cola (últimos 30 s)', () => {
      const state = makeState({ positionSeconds: 2971, durationSeconds: 3000 });
      expect(calculatePlayback(state, baseConfig, baseNow).isCompleted).toBe(true);
    });

    it('no completa si no supera umbral ni cola', () => {
      const state = makeState({ positionSeconds: 2500, durationSeconds: 3000 });
      expect(calculatePlayback(state, baseConfig, baseNow).isCompleted).toBe(false);
    });

    it('sin duración no completa automáticamente', () => {
      const state = makeState({ positionSeconds: 500, durationSeconds: null });
      expect(calculatePlayback(state, baseConfig, baseNow).isCompleted).toBe(false);
    });

    it('completado explícito (completedAt) tiene prioridad', () => {
      const state = makeState({ completedAt: baseNow, positionSeconds: 100 });
      expect(calculatePlayback(state, baseConfig, baseNow).isCompleted).toBe(true);
    });

    it('canResume respeta config.resume, umbral y completado', () => {
      expect(
        calculatePlayback(
          makeState({ positionSeconds: 60, durationSeconds: 3000 }),
          baseConfig,
          baseNow,
        ).canResume,
      ).toBe(true);
      expect(
        calculatePlayback(
          makeState({ positionSeconds: 20, durationSeconds: 3000 }),
          baseConfig,
          baseNow,
        ).canResume,
      ).toBe(false);
      expect(
        calculatePlayback(
          makeState({ positionSeconds: 60, durationSeconds: 3000 }),
          noResumeConfig,
          baseNow,
        ).canResume,
      ).toBe(false);
      expect(
        calculatePlayback(
          makeState({ positionSeconds: 2700, durationSeconds: 3000, completedAt: baseNow }),
          baseConfig,
          baseNow,
        ).canResume,
      ).toBe(false);
    });

    it('canResume false cuando position alcanza duration; true si duration es null', () => {
      expect(
        calculatePlayback(
          makeState({ positionSeconds: 60, durationSeconds: null }),
          baseConfig,
          baseNow,
        ).canResume,
      ).toBe(true);
      expect(
        calculatePlayback(
          makeState({ positionSeconds: 3000, durationSeconds: 3000 }),
          baseConfig,
          baseNow,
        ).canResume,
      ).toBe(false);
    });

    it('progressPercent null sin duración y clamp 0-100', () => {
      expect(
        calculatePlayback(
          makeState({ positionSeconds: 1500, durationSeconds: 3000 }),
          baseConfig,
          baseNow,
        ).progressPercent,
      ).toBe(50);
      expect(
        calculatePlayback(
          makeState({ positionSeconds: -100, durationSeconds: 3000 }),
          baseConfig,
          baseNow,
        ).progressPercent,
      ).toBe(0);
      expect(
        calculatePlayback(
          makeState({ positionSeconds: 4000, durationSeconds: 3000 }),
          baseConfig,
          baseNow,
        ).progressPercent,
      ).toBe(100);
      expect(
        calculatePlayback(
          makeState({ positionSeconds: 100, durationSeconds: null }),
          baseConfig,
          baseNow,
        ).progressPercent,
      ).toBeNull();
    });
  });

  describe('resolveResumeDecision', () => {
    it('--restart siempre gana', () => {
      expect(
        resolveResumeDecision({
          restart: true,
          resume: true,
          hasSavedState: true,
          canResume: true,
          positionSeconds: 100,
        }),
      ).toBe('restart');
    });

    it('--resume fuerza reanudar y lanza si no hay estado', () => {
      expect(() =>
        resolveResumeDecision({
          resume: true,
          restart: false,
          hasSavedState: false,
          canResume: false,
          positionSeconds: 0,
        }),
      ).toThrow('No hay posición guardada reanudable');
      expect(() =>
        resolveResumeDecision({
          resume: true,
          restart: false,
          hasSavedState: true,
          canResume: false,
          positionSeconds: 100,
        }),
      ).toThrow('No hay posición guardada reanudable');
    });

    it('--resume funciona con estado reanudable', () => {
      expect(
        resolveResumeDecision({
          resume: true,
          restart: false,
          hasSavedState: true,
          canResume: true,
          positionSeconds: 100,
        }),
      ).toBe('resume');
    });

    it('por defecto reanuda si config.resume y estado reanudable', () => {
      expect(
        resolveResumeDecision({
          restart: false,
          resume: false,
          hasSavedState: true,
          canResume: true,
          positionSeconds: 100,
        }),
      ).toBe('resume');
      expect(
        resolveResumeDecision({
          restart: false,
          resume: false,
          hasSavedState: true,
          canResume: false,
          positionSeconds: 100,
        }),
      ).toBe('none');
      expect(
        resolveResumeDecision({
          restart: false,
          resume: false,
          hasSavedState: false,
          canResume: true,
          positionSeconds: 0,
        }),
      ).toBe('none');
    });
  });

  describe('findNextEpisode', () => {
    const files: MediaFileRow[] = [
      makeFile({ id: 1, season: 1, episode: 1, filePath: '/lib/S01E01.mkv' }),
      makeFile({ id: 2, season: 1, episode: 2, filePath: '/lib/S01E02.mkv' }),
      makeFile({ id: 3, season: 1, episode: 3, filePath: '/lib/S01E03.mkv' }),
      makeFile({ id: 4, season: 2, episode: 1, filePath: '/lib/S02E01.mkv' }),
      makeFile({ id: 5, season: 0, episode: 1, filePath: '/lib/Special.mkv' }), // especial
    ];

    const repo = {
      get(id: number): PlaybackStateRow | undefined {
        if (id === 2) return makeState({ completedAt: baseNow });
        if (id === 3) return makeState({ positionSeconds: 100 });
        return undefined;
      },
    };

    it('encuentra el siguiente no completado en la misma temporada', () => {
      const current = files[0]!; // S01E01
      const next = findNextEpisode(current, files, repo, baseConfig);
      expect(next?.file.id).toBe(3); // S01E03 (E02 completado)
    });

    it('salta a la siguiente temporada si la actual está completa', () => {
      const current = files[2]!; // S01E03
      const next = findNextEpisode(current, files, repo, baseConfig);
      expect(next?.file.id).toBe(4); // S02E01
    });

    it('devuelve null si todo está completado', () => {
      const repoAllDone = {
        get(id: number): PlaybackStateRow | undefined {
          if (id === 4) return makeState({ completedAt: baseNow });
          return undefined;
        },
      };
      const current = files[2]!; // S01E03
      expect(findNextEpisode(current, files, repoAllDone, baseConfig)).toBeNull();
    });

    it('ignora especiales (season = 0) en la progresión regular', () => {
      const current = files[3]!; // S02E01
      const next = findNextEpisode(current, files, repo, baseConfig);
      expect(next).toBeNull(); // no hay S03, y el especial no cuenta
    });

    it('null en películas (sin season/episode)', () => {
      const movieFile = makeFile({ season: null, episode: null });
      expect(findNextEpisode(movieFile, files, repo, baseConfig)).toBeNull();
    });

    it('null cuando mediaId es undefined', () => {
      const noMediaId = makeFile({
        season: 1,
        episode: 1,
        mediaId: undefined as unknown as number,
      });
      expect(findNextEpisode(noMediaId, files, repo, baseConfig)).toBeNull();
    });

    it('null cuando todos los candidatos de la temporada están completados', () => {
      const allDoneRepo = {
        get(id: number): PlaybackStateRow | undefined {
          if (id === 2) return makeState({ completedAt: baseNow });
          if (id === 3) return makeState({ completedAt: baseNow });
          return undefined;
        },
      };
      const current = files[0]!; // S01E01, candidatos E02 y E03 ambos completados
      expect(findNextEpisode(current, files, allDoneRepo, baseConfig)).toBeNull();
    });
  });

  describe('isEpisodic', () => {
    it('true para series y anime', () => {
      expect(isEpisodic('series')).toBe(true);
      expect(isEpisodic('anime')).toBe(true);
      expect(isEpisodic('movie')).toBe(false);
    });
  });

  describe('applyStartPlaceholder', () => {
    it('sustituye {start} por segundos enteros', () => {
      expect(applyStartPlaceholder(['mpv', '--start={start}', '{file}'], 90.7)).toEqual([
        'mpv',
        '--start=90',
        '{file}',
      ]);
    });

    it('elimina el argumento con {start} si no hay reanudación', () => {
      expect(applyStartPlaceholder(['mpv', '--start={start}', '{file}'], null)).toEqual([
        'mpv',
        '{file}',
      ]);
    });

    it('sustituye todas las apariciones', () => {
      expect(applyStartPlaceholder(['mpv', '{start}', '--opt={start}', '{file}'], 5)).toEqual([
        'mpv',
        '5',
        '--opt=5',
        '{file}',
      ]);
    });
  });

  describe('nowIso', () => {
    it('devuelve ISO-8601 UTC', () => {
      expect(nowIso(new Date('2026-10-23T10:15:30.123Z'))).toBe('2026-10-23T10:15:30.123Z');
    });

    it('usa el reloj actual cuando no se pasa now', () => {
      const result = nowIso();
      expect(result).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}.\d{3}Z$/);
    });
  });
});
