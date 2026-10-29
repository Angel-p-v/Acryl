import { cleanup, render } from 'ink-testing-library';
import React from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { defaultConfig } from '../../../src/core/config/defaults.js';
import { NotFoundError, PlayerError } from '../../../src/shared/errors.js';
import { App } from '../../../src/ui/App.js';

import type { AppConfig } from '../../../src/core/config/schema.js';
import type { MediaFileRow, MediaRow } from '../../../src/core/db/types.js';
import type { LibraryService } from '../../../src/core/library/service.js';
import type { MetadataService } from '../../../src/core/metadata/service.js';
import type { LaunchPlan, PlayerHandle } from '../../../src/core/player/launcher.js';

const config: AppConfig = {
  ...defaultConfig,
  library: {
    ...defaultConfig.library,
    roots: [{ path: '/media/series', type: 'series' }],
  },
  keybindings: { ...defaultConfig.keybindings },
};

const severance: MediaRow = {
  id: 1,
  type: 'series',
  title: 'Severance',
  sortTitle: 'severance',
  year: 2022,
  folderPath: '/media/series/Severance (2022)',
  createdAt: '2026-01-01 00:00:00',
  updatedAt: '2026-01-01 00:00:00',
};

const dark: MediaRow = {
  ...severance,
  id: 2,
  title: 'Dark',
  sortTitle: 'dark',
  year: 2017,
  folderPath: '/media/series/Dark',
};

const frieren: MediaRow = {
  ...severance,
  id: 3,
  type: 'anime',
  title: 'Frieren',
  sortTitle: 'frieren',
  year: 2023,
  folderPath: '/media/anime/Frieren',
};

const arrival: MediaRow = {
  ...severance,
  id: 4,
  type: 'movie',
  title: 'Arrival',
  sortTitle: 'arrival',
  year: 2016,
  folderPath: '/media/movies/Arrival.mkv',
};

const bladeRunner: MediaRow = {
  ...arrival,
  id: 5,
  title: 'Blade Runner 2049',
  sortTitle: 'blade runner 2049',
  year: 2017,
  folderPath: '/media/movies/Blade.Runner.2049.mkv',
};

const defaultRows: MediaRow[] = [severance, dark, frieren, arrival, bladeRunner];

const files: Record<number, MediaFileRow[]> = {
  1: [
    {
      id: 1,
      mediaId: 1,
      season: 1,
      episode: 1,
      title: null,
      filePath: '/media/series/Severance/S01E01.mkv',
      sizeBytes: 100,
      mtimeMs: 1,
      indexedAt: '2026-01-01 00:00:00',
    },
    {
      id: 2,
      mediaId: 1,
      season: 1,
      episode: 2,
      title: null,
      filePath: '/media/series/Severance/S01E02.mkv',
      sizeBytes: 100,
      mtimeMs: 1,
      indexedAt: '2026-01-01 00:00:00',
    },
  ],
};

function fakeLibrary(rows: MediaRow[] = defaultRows): LibraryService {
  return {
    list: (options = {}) => {
      let filtered =
        options.type === undefined ? rows : rows.filter((row) => row.type === options.type);

      // Simular filtro de favoritos: para el test, Arrival y Blade Runner 2049 son favoritos
      if (options.favorites) {
        filtered = filtered.filter(
          (row) => row.title === 'Arrival' || row.title === 'Blade Runner 2049',
        );
      }
      // Simular filtro de en curso
      if (options.inProgress) {
        filtered = filtered.filter((row) => row.title === 'Severance');
      }
      // Simular filtro de no vistos
      if (options.unwatched) {
        filtered = filtered.filter(
          (row) => row.title !== 'Arrival' && row.title !== 'Blade Runner 2049',
        );
      }
      // Simular filtro de vistos
      if (options.watched) {
        filtered = filtered.filter(
          (row) => row.title === 'Arrival' || row.title === 'Blade Runner 2049',
        );
      }

      const offset = options.offset ?? 0;
      const limit = options.limit ?? filtered.length;
      return filtered.slice(offset, offset + limit);
    },
    count: (type) =>
      type === undefined ? rows.length : rows.filter((row) => row.type === type).length,
    search: (query, options = {}) => {
      const normalized = query.toLocaleLowerCase();
      return rows
        .filter((row) => row.title.toLocaleLowerCase().includes(normalized))
        .filter((row) => options.type === undefined || row.type === options.type)
        .slice(0, options.limit ?? 20);
    },
    find: (query) => {
      const media = /^\d+$/.test(query)
        ? rows.find((row) => row.id === Number(query))
        : rows.find((row) => row.title.toLocaleLowerCase() === query.toLocaleLowerCase());
      if (media === undefined) throw new NotFoundError(`No existe «${query}».`);
      return { media, files: files[media.id] ?? [] };
    },
    stats: () => ({
      media: {
        total: rows.length,
        series: rows.filter((row) => row.type === 'series').length,
        anime: rows.filter((row) => row.type === 'anime').length,
        movie: rows.filter((row) => row.type === 'movie').length,
        unknown: rows.filter((row) => row.type === 'unknown').length,
      },
      files: Object.values(files).reduce((total, list) => total + list.length, 0),
      lastRun: null,
      inProgress: 1, // Severance en curso
    }),
    listInProgress: () =>
      [defaultRows.find((r) => r.title === 'Severance')].filter(Boolean) as MediaRow[],
    countFavorites: () =>
      rows.filter((row) => row.title === 'Arrival' || row.title === 'Blade Runner 2049').length,
    listTags: () => [],
    getTagByName: () => undefined,
    assignTag: () => undefined,
    unassignTag: () => undefined,
    listTagsForMedia: () => [],
    getMetadata: () => undefined,
    setMetadata: () => ({
      mediaId: 0,
      provider: 'tmdb',
      externalId: 0,
      title: null,
      overview: null,
      posterPath: null,
      backdropPath: null,
      voteAverage: null,
      localPosterPath: null,
      firstAirDate: null,
      releaseDate: null,
      originalLanguage: null,
      fetchedAt: '',
      updatedAt: '',
    }),
    removeMetadata: () => false,
    getOrFetchMetadata: async () => {
      await Promise.resolve();
      return undefined;
    },
    refreshExpiredMetadata: async () => {
      await Promise.resolve();
      return { refreshed: 0, failed: 0 };
    },
    fetchMissingMetadata: async () => {
      await Promise.resolve();
      return { fetched: 0, failed: 0 };
    },
    getExpiredMetadataCount: () => 0,
    hasMetadataProviderConfigured: () => false,
    clearAllMetadata: () => 0,
    metadataService: {
      updateConfig: () => undefined,
    } as unknown as MetadataService,
  };
}

async function press(stdin: ReturnType<typeof render>['stdin'], value: string): Promise<void> {
  stdin.write(value);
  // Esperar a que React/Ink procese el evento y re-renderice antes de leer el frame.
  await new Promise((resolve) => setTimeout(resolve, 100));
}

async function typeText(stdin: ReturnType<typeof render>['stdin'], text: string): Promise<void> {
  for (const char of text) {
    await press(stdin, char);
  }
}

async function flush(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 20));
}

function frameOf(lastFrame: () => string | undefined): string {
  return lastFrame() ?? '';
}

describe('App (TUI)', () => {
  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it('renderiza la biblioteca con secciones, títulos y detalles', () => {
    const { lastFrame, unmount } = render(<App config={config} library={fakeLibrary()} />);
    const frame = frameOf(lastFrame);

    expect(frame).toContain('media-manager');
    expect(frame).toContain('Biblioteca');
    expect(frame).toContain('Severance');
    expect(frame).toContain('Ficheros');
    expect(frame).toContain('[q]');
    unmount();
  });

  it('cambia de sección con Tab y filtra los títulos', async () => {
    const { lastFrame, stdin, unmount } = render(<App config={config} library={fakeLibrary()} />);

    // Tab 1: 'in-progress', Tab 2: 'favorites', Tab 3: 'series'
    await press(stdin, '\t');
    await press(stdin, '\t');
    await press(stdin, '\t');

    const frame = frameOf(lastFrame);
    expect(frame).toContain('Dark');
    expect(frame).not.toContain('Arrival');
    expect(frame).not.toContain('Blade Runner 2049');
    unmount();
  });

  it('mueve la selección con las flechas y actualiza el detalle', async () => {
    const { lastFrame, stdin, unmount } = render(<App config={config} library={fakeLibrary()} />);

    await press(stdin, '\u001b[B');

    const frame = frameOf(lastFrame);
    expect(frame).toContain('Dark');
    expect(frame).toContain('Dark (2017)');
    unmount();
  });

  it('abre el buscador, filtra al escribir y vuelve con Escape', async () => {
    const { lastFrame, stdin } = render(<App config={config} library={fakeLibrary()} />);

    await press(stdin, '/');
    expect(frameOf(lastFrame)).toContain('Buscar');

    await typeText(stdin, 'sever');
    const searchFrame = frameOf(lastFrame);
    expect(searchFrame).toContain('Severance');
    expect(searchFrame).not.toContain('Arrival');

    await press(stdin, '\u001b');
    expect(frameOf(lastFrame)).toContain('Biblioteca');
  });

  it('no cierra al escribir «q» dentro del buscador', async () => {
    const onQuit = vi.fn();
    const { stdin, unmount } = render(
      <App config={config} library={fakeLibrary()} onQuit={onQuit} />,
    );

    await press(stdin, '/');
    await press(stdin, 'q');

    expect(onQuit).not.toHaveBeenCalled();
    unmount();
  });

  it('muestra y oculta la ayuda con «?»', async () => {
    const { lastFrame, stdin, unmount } = render(<App config={config} library={fakeLibrary()} />);

    await press(stdin, '?');
    expect(frameOf(lastFrame)).toContain('Atajos de teclado');

    await press(stdin, '?');
    expect(frameOf(lastFrame)).not.toContain('Atajos de teclado');
    unmount();
  });

  it('abre la vista de ajustes con «s»', async () => {
    const { lastFrame, stdin, unmount } = render(<App config={config} library={fakeLibrary()} />);

    await press(stdin, 's');

    const frame = frameOf(lastFrame);
    expect(frame).toContain('Añadir raíz');
    expect(frame).toContain('/media/series');
    expect(frame).toContain('Base de datos');
    unmount();
  });

  it('vuelve a la biblioteca desde ajustes con Tab', async () => {
    const { lastFrame, stdin, unmount } = render(<App config={config} library={fakeLibrary()} />);

    await press(stdin, 's');
    expect(frameOf(lastFrame)).toContain('Añadir raíz');

    await press(stdin, '\t');
    const frame = frameOf(lastFrame);
    expect(frame).toContain('Biblioteca');
    expect(frame).toContain('Severance');
    expect(frame).not.toContain('Configuración efectiva');
    unmount();
  });

  it('muestra el hint de teclas para volver desde ajustes', async () => {
    const { lastFrame, stdin, unmount } = render(<App config={config} library={fakeLibrary()} />);

    await press(stdin, 's');

    const frame = frameOf(lastFrame);
    expect(frame).toContain('Presiona «Escape» o «Tab» para volver a la biblioteca.');
    unmount();
  });

  it('lanza la reproducción sin bloquear la TUI y sobrevive a la espera', async () => {
    let resolveLaunch: ((handle: PlayerHandle) => void) | undefined;
    const launchPlayer = vi.fn(
      (_plan: LaunchPlan) =>
        new Promise<PlayerHandle>((resolve) => {
          resolveLaunch = resolve;
        }),
    );
    const { lastFrame, stdin, unmount } = render(
      <App config={config} library={fakeLibrary()} launchPlayer={launchPlayer} />,
    );

    await press(stdin, 'p');

    expect(launchPlayer).toHaveBeenCalledOnce();
    expect(launchPlayer.mock.calls[0]?.[0]).toMatchObject({
      player: 'mpv',
      mode: 'detach',
      filePath: '/media/series/Severance/S01E01.mkv',
    });
    expect(frameOf(lastFrame)).toContain('Abriendo Severance · T01E01 en mpv');

    // La TUI sigue respondiendo mientras el reproductor arranca.
    await press(stdin, '\u001b[B');
    expect(frameOf(lastFrame)).toContain('Dark');

    resolveLaunch?.({ pid: 42, wait: () => Promise.resolve() });
    await flush();
    expect(frameOf(lastFrame)).toContain('▶ Severance · T01E01 · mpv');
    unmount();
  });

  it('muestra el error del reproductor en la barra de estado', async () => {
    const launchPlayer = vi.fn(() => Promise.reject(new PlayerError('mpv no está instalado')));
    const { lastFrame, stdin, unmount } = render(
      <App config={config} library={fakeLibrary()} launchPlayer={launchPlayer} />,
    );

    await press(stdin, 'p');
    await flush();

    expect(frameOf(lastFrame)).toContain('mpv no está instalado');
    unmount();
  });

  it('respeta los atajos personalizados de la configuración', async () => {
    const onQuit = vi.fn();
    const customConfig: AppConfig = {
      ...config,
      keybindings: { ...config.keybindings, quit: 'x' },
    };
    const { stdin, unmount } = render(
      <App config={customConfig} library={fakeLibrary()} onQuit={onQuit} />,
    );

    await press(stdin, 'q');
    expect(onQuit).not.toHaveBeenCalled();

    await press(stdin, 'x');
    expect(onQuit).toHaveBeenCalledOnce();
    unmount();
  });

  it('cierra con «q» fuera del buscador', async () => {
    const onQuit = vi.fn();
    const { stdin, unmount } = render(
      <App config={config} library={fakeLibrary()} onQuit={onQuit} />,
    );

    await press(stdin, 'q');

    expect(onQuit).toHaveBeenCalledOnce();
    unmount();
  });

  it('arranca en menos de 300 ms con 10 000 títulos', () => {
    const rows: MediaRow[] = Array.from({ length: 10_000 }, (_, index) => ({
      ...severance,
      id: index + 1,
      title: `Título ${String(index + 1).padStart(5, '0')}`,
      sortTitle: `titulo ${String(index + 1).padStart(5, '0')}`,
      folderPath: `/media/series/Titulo-${index + 1}`,
    }));

    const started = performance.now();
    const { lastFrame, unmount } = render(<App config={config} library={fakeLibrary(rows)} />);
    const frame = frameOf(lastFrame);
    const elapsed = performance.now() - started;

    expect(frame).toContain('Título 00001');
    expect(elapsed).toBeLessThan(300);
    unmount();
  });

  it('Enter abre la vista de detalles de reproducción para el título seleccionado', async () => {
    const { lastFrame, stdin, unmount } = render(<App config={config} library={fakeLibrary()} />);

    // Severance está seleccionado por defecto (índice 0)
    await press(stdin, '\r');

    const frame = frameOf(lastFrame);
    expect(frame).toContain('Reproducir');
    expect(frame).toContain('Severance');
    expect(frame).toContain('[Esc]');
    unmount();
  });

  it('q vuelve a la biblioteca desde la vista de detalles de reproducción', async () => {
    const { lastFrame, stdin, unmount } = render(<App config={config} library={fakeLibrary()} />);

    await press(stdin, '\r');
    expect(frameOf(lastFrame)).toContain('Reproducir');

    await press(stdin, 'q');
    expect(frameOf(lastFrame)).toContain('Biblioteca');
    expect(frameOf(lastFrame)).toContain('Severance');
    unmount();
  });

  it('Escape vuelve a la biblioteca desde la vista de detalles de reproducción', async () => {
    const { lastFrame, stdin, unmount } = render(<App config={config} library={fakeLibrary()} />);

    await press(stdin, '\r');
    expect(frameOf(lastFrame)).toContain('Reproducir');

    await press(stdin, '\u001b');
    expect(frameOf(lastFrame)).toContain('Biblioteca');
    expect(frameOf(lastFrame)).toContain('Severance');
    unmount();
  });

  it('p desde la vista de detalles de reproducción lanza el reproductor con el fichero seleccionado', async () => {
    const launchPlayer = vi.fn(
      (_plan: LaunchPlan) =>
        new Promise<PlayerHandle>((_resolve) => {
          // Nunca se resuelve: el test solo verifica que se llamó.
        }),
    );
    const { lastFrame, stdin, unmount } = render(
      <App config={config} library={fakeLibrary()} launchPlayer={launchPlayer} />,
    );

    // Abrir vista de detalles de reproducción (Enter sobre Severance)
    await press(stdin, '\r');
    expect(frameOf(lastFrame)).toContain('Reproducir');

    // 'p' reproduce el fichero seleccionado (S01E01 por defecto)
    await press(stdin, 'p');

    expect(launchPlayer).toHaveBeenCalledOnce();
    expect(launchPlayer.mock.calls[0]?.[0]).toMatchObject({
      player: 'mpv',
      mode: 'detach',
      filePath: '/media/series/Severance/S01E01.mkv',
    });
    expect(frameOf(lastFrame)).toContain('Abriendo Severance · T01E01 en mpv');

    unmount();
  });

  it('navega entre episodios con flechas arriba/abajo y cambia el fichero seleccionado', async () => {
    const { lastFrame, stdin, unmount } = render(<App config={config} library={fakeLibrary()} />);

    // Abrir vista de detalles de reproducción (Enter sobre Severance)
    await press(stdin, '\r');
    expect(frameOf(lastFrame)).toContain('Reproducir');

    // Severance tiene 2 episodios: S01E01 y S01E02
    // Por defecto está seleccionado el primero (S01E01) marcado con ▸
    let frame = frameOf(lastFrame);
    expect(frame).toContain('▸ T01E01');
    expect(frame).toContain('   T01E02');

    // Moverse al segundo episodio con flecha abajo: el ▸ se mueve a T01E02
    await press(stdin, '\u001b[B');
    frame = frameOf(lastFrame);
    expect(frame).toContain('▸ T01E02');
    expect(frame).toContain('   T01E01');

    // Volver al primero con flecha arriba
    await press(stdin, '\u001b[A');
    frame = frameOf(lastFrame);
    expect(frame).toContain('▸ T01E01');
    expect(frame).toContain('   T01E02');

    unmount();
  });

  it('muestra "sin pistas de audio detectadas" cuando ffprobe no está disponible', async () => {
    const { lastFrame, stdin, unmount } = render(<App config={config} library={fakeLibrary()} />);

    await press(stdin, '\r');
    await new Promise((resolve) => setTimeout(resolve, 150));

    const frame = frameOf(lastFrame);

    // ffprobe no está instalado en el entorno de test, por tanto no hay pistas detectadas
    expect(frame).toContain('sin pistas de audio detectadas');
    expect(frame).toContain('sin pistas de subtítulos detectadas');

    unmount();
  });

  it('muestra máximo 12 episodios por página con paginación', async () => {
    const libraryManyEpisodes = {
      ...fakeLibrary(),
      find: (query: string) => {
        const rows = fakeLibrary().list();
        const media = /^\d+$/.test(query)
          ? rows.find((row) => row.id === Number(query))
          : rows.find((row) => row.title.toLocaleLowerCase() === query.toLocaleLowerCase());
        if (media === undefined) throw new NotFoundError(`No existe «${query}».`);
        // Crear 20 episodios para probar paginación
        const files: MediaFileRow[] = Array.from({ length: 20 }, (_, i) => ({
          id: i + 1,
          mediaId: media.id,
          season: 1,
          episode: i + 1,
          title: null,
          filePath: `/media/series/${media.title}/S01E${String(i + 1).padStart(2, '0')}.mkv`,
          sizeBytes: 100,
          mtimeMs: 1,
          indexedAt: '2026-01-01 00:00:00',
        }));
        return { media, files };
      },
    };

    const { lastFrame, stdin, unmount } = render(
      <App config={config} library={libraryManyEpisodes} />,
    );

    await press(stdin, '\r');
    let frame = frameOf(lastFrame);

    // Primera página: debe mostrar episodios 1-12
    expect(frame).toContain('T01E01');
    expect(frame).toContain('T01E12');
    expect(frame).not.toContain('T01E13');

    // Navegar al episodio 13 con flecha abajo 12 veces
    for (let i = 0; i < 12; i++) {
      await press(stdin, '\u001b[B');
    }
    frame = frameOf(lastFrame);

    // Ahora debería mostrar la página 2 (episodios 13-20)
    expect(frame).toContain('T01E13');
    // El episodio 1 ya no debería estar visible
    const episodeLines = frame.split('\n').filter((line) => line.includes('T01E01'));
    expect(episodeLines.length).toBe(0);

    unmount();
  });

  it('el scrolling de paginación mueve la ventana al llegar al límite', async () => {
    const libraryManyEpisodes = {
      ...fakeLibrary(),
      find: (query: string) => {
        const rows = fakeLibrary().list();
        const media = /^\d+$/.test(query)
          ? rows.find((row) => row.id === Number(query))
          : rows.find((row) => row.title.toLocaleLowerCase() === query.toLocaleLowerCase());
        if (media === undefined) throw new NotFoundError(`No existe «${query}».`);
        const files: MediaFileRow[] = Array.from({ length: 15 }, (_, i) => ({
          id: i + 1,
          mediaId: media.id,
          season: 1,
          episode: i + 1,
          title: null,
          filePath: `/media/series/${media.title}/S01E${String(i + 1).padStart(2, '0')}.mkv`,
          sizeBytes: 100,
          mtimeMs: 1,
          indexedAt: '2026-01-01 00:00:00',
        }));
        return { media, files };
      },
    };

    const { lastFrame, stdin, unmount } = render(
      <App config={config} library={libraryManyEpisodes} />,
    );

    await press(stdin, '\r');
    let frame = frameOf(lastFrame);
    expect(frame).toContain('T01E12');
    expect(frame).not.toContain('T01E13');

    // Ir al episodio 13 (índice 12)
    for (let i = 0; i < 12; i++) {
      await press(stdin, '\u001b[B');
    }
    frame = frameOf(lastFrame);
    expect(frame).toContain('T01E13');

    // Volver al episodio 1
    for (let i = 0; i < 12; i++) {
      await press(stdin, '\u001b[A');
    }
    frame = frameOf(lastFrame);
    expect(frame).toContain('T01E01');

    unmount();
  });
});
