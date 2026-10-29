import { describe, expect, it } from 'vitest';

import { createLaunchPlan, isExecutable } from '../../../src/core/player/launcher.js';
import { selectMediaFile } from '../../../src/core/player/selection.js';
import { PlayerError } from '../../../src/shared/errors.js';

import type { PlayerConfig } from '../../../src/core/config/schema.js';
import type { MediaFileRow } from '../../../src/core/db/types.js';

const playerConfig: PlayerConfig = {
  default: 'fake',
  onLaunch: 'detach',
  players: {
    fake: {
      command: 'fake-player',
      args: ['--input={file}', '--title', '{file}.json'],
    },
    quiet: {
      command: 'quiet-player',
      args: ['{file}'],
      env: { QUIET: '1' },
    },
  },
};

function mediaFile(id: number, season: number | null, episode: number | null): MediaFileRow {
  return {
    id,
    mediaId: 1,
    season,
    episode,
    title: null,
    filePath: `/media/Show/S${String(season).padStart(2, '0')}E${String(episode).padStart(2, '0')}.mkv`,
    sizeBytes: 1,
    mtimeMs: 1,
    indexedAt: '2026-01-01 00:00:00',
  };
}

describe('createLaunchPlan', () => {
  it('sustituye {file} en cada argumento y conserva las rutas con espacios', () => {
    const filePath = '/media/Series/Show (2022)/S01E01.mkv';

    const plan = createLaunchPlan({ playerConfig, filePath });

    expect(plan).toEqual({
      player: 'fake',
      command: 'fake-player',
      args: ['--input=/media/Series/Show (2022)/S01E01.mkv', '--title', `${filePath}.json`],
      mode: 'detach',
      filePath,
      subtitlePath: null,
    });
  });

  it('permite elegir reproductor y modo sin mutar la configuración', () => {
    const plan = createLaunchPlan({
      playerConfig,
      filePath: '/media/movie.mkv',
      playerName: 'quiet',
      mode: 'foreground',
    });

    expect(plan).toMatchObject({
      player: 'quiet',
      command: 'quiet-player',
      args: ['/media/movie.mkv'],
      mode: 'foreground',
      env: { QUIET: '1' },
    });
    expect(playerConfig.players.quiet?.env).toEqual({ QUIET: '1' });
  });

  it('falla si el reproductor no existe y enumera los disponibles', () => {
    expect(() =>
      createLaunchPlan({ playerConfig, filePath: '/media/movie.mkv', playerName: 'mpv' }),
    ).toThrow(PlayerError);
    expect(() =>
      createLaunchPlan({ playerConfig, filePath: '/media/movie.mkv', playerName: 'mpv' }),
    ).toThrow('Disponibles: fake, quiet');
  });

  it('exige el marcador {file} en la plantilla', () => {
    const invalid: PlayerConfig = {
      ...playerConfig,
      players: { broken: { command: 'broken', args: ['--fullscreen'] } },
      default: 'broken',
    };

    expect(() => createLaunchPlan({ playerConfig: invalid, filePath: '/media/movie.mkv' })).toThrow(
      'debe incluir el marcador {file}',
    );
  });

  it('rechaza una ruta de fichero vacía', () => {
    expect(() => createLaunchPlan({ playerConfig, filePath: '   ' })).toThrow(
      'No hay ningún fichero que reproducir',
    );
  });

  it('sustituye {start} con startSeconds y filtra argumentos vacíos', () => {
    const withStart: PlayerConfig = {
      ...playerConfig,
      players: {
        ...playerConfig.players,
        startplayer: {
          command: 'start-player',
          args: ['--start={start}', '{file}'],
        },
      },
      default: 'startplayer',
    };

    const plan = createLaunchPlan({
      playerConfig: withStart,
      filePath: '/media/video.mkv',
      startSeconds: 90.7,
    });

    expect(plan.args).toEqual(['--start=90', '/media/video.mkv']);
  });

  it('sustituye {start} con string vacío cuando startSeconds es null', () => {
    const withStart: PlayerConfig = {
      ...playerConfig,
      players: {
        ...playerConfig.players,
        startplayer: {
          command: 'start-player',
          args: ['--start={start}', '{file}'],
        },
      },
      default: 'startplayer',
    };

    const plan = createLaunchPlan({
      playerConfig: withStart,
      filePath: '/media/video.mkv',
    });

    expect(plan.args).toEqual(['--start=', '/media/video.mkv']);
  });

  it('añade --sub-file a los args cuando se indica subtitlePath', () => {
    const plan = createLaunchPlan({
      playerConfig,
      filePath: '/media/episodio.mkv',
      subtitlePath: '/media/episodio.srt',
    });

    expect(plan.subtitlePath).toBe('/media/episodio.srt');
    expect(plan.args).toEqual([
      '--input=/media/episodio.mkv',
      '--title',
      '/media/episodio.mkv.json',
      '--sub-file=/media/episodio.srt',
    ]);
  });

  it('subtituloPath null no añade --sub-file', () => {
    const plan = createLaunchPlan({
      playerConfig,
      filePath: '/media/episodio.mkv',
      subtitlePath: null,
    });

    expect(plan.subtitlePath).toBeNull();
    expect(plan.args).toEqual([
      '--input=/media/episodio.mkv',
      '--title',
      '/media/episodio.mkv.json',
    ]);
  });

  it('combina startSeconds y subtitlePath', () => {
    const withStart: PlayerConfig = {
      ...playerConfig,
      players: {
        ...playerConfig.players,
        startplayer: {
          command: 'start-player',
          args: ['--start={start}', '{file}'],
        },
      },
      default: 'startplayer',
    };

    const plan = createLaunchPlan({
      playerConfig: withStart,
      filePath: '/media/video.mkv',
      startSeconds: 45,
      subtitlePath: '/media/video.srt',
    });

    expect(plan.args).toEqual(['--start=45', '/media/video.mkv', '--sub-file=/media/video.srt']);
    expect(plan.subtitlePath).toBe('/media/video.srt');
  });
});

describe('isExecutable', () => {
  it('devuelve false para comando vacío', async () => {
    const result = await isExecutable('');
    expect(result).toBe(false);
  });
});

describe('selectMediaFile', () => {
  it('elige el primer fichero cuando no se indica ninguno', () => {
    const files = [mediaFile(1, 1, 1), mediaFile(2, 1, 2)];

    expect(selectMediaFile(files).id).toBe(1);
  });

  it('permite elegir un fichero por id', () => {
    const files = [mediaFile(1, 1, 1), mediaFile(2, 1, 2)];

    expect(selectMediaFile(files, 2).episode).toBe(2);
  });

  it('falla si el id no pertenece al título', () => {
    expect(() => selectMediaFile([mediaFile(1, 1, 1)], 99)).toThrow('Ficheros disponibles: #1');
  });

  it('falla si el título no tiene ficheros', () => {
    expect(() => selectMediaFile([])).toThrow('no tiene ficheros indexados');
  });
});
