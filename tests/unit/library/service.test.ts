import { describe, expect, it, vi } from 'vitest';

import { createLibraryService } from '../../../src/core/library/service.js';
import { AmbiguousError, NotFoundError } from '../../../src/shared/errors.js';

import type { FileRepository } from '../../../src/core/db/repositories/fileRepo.js';
import type {
  ListMediaOptions,
  MediaRepository,
  SearchMediaOptions,
} from '../../../src/core/db/repositories/mediaRepo.js';
import type {
  MetadataRepository,
  SetMetadataInput,
} from '../../../src/core/db/repositories/metadataRepo.js';
import type { PlaybackRepository } from '../../../src/core/db/repositories/playbackRepo.js';
import type { PreferenceRepository } from '../../../src/core/db/repositories/preferencesRepo.js';
import type { TagRepository } from '../../../src/core/db/repositories/tagRepo.js';
import type { IndexRunRow, MediaFileRow, MediaRow } from '../../../src/core/db/types.js';
import type { MetadataService } from '../../../src/core/metadata/service.js';

const dark: MediaRow = {
  id: 1,
  type: 'series',
  title: 'Dark',
  sortTitle: 'dark',
  year: 2017,
  folderPath: '/media/series/Dark',
  createdAt: '2026-01-01 00:00:00',
  updatedAt: '2026-01-01 00:00:00',
};

const darkMatter: MediaRow = {
  ...dark,
  id: 2,
  title: 'Dark Matter',
  sortTitle: 'dark matter',
  folderPath: '/media/series/Dark Matter',
};

const amelie: MediaRow = {
  ...dark,
  id: 3,
  type: 'movie',
  title: 'Amélie',
  sortTitle: 'amélie',
  year: 2001,
  folderPath: '/media/movies/Amelie.mkv',
};

function mediaRow(id: number, title: string): MediaRow {
  return { ...dark, id, title, sortTitle: title.toLocaleLowerCase() };
}

function fakeMediaRepository(overrides: Partial<MediaRepository> = {}): MediaRepository {
  const notImplemented = (): never => {
    throw new Error('método no esperado en el doble de prueba');
  };

  return {
    upsert: notImplemented,
    findById: () => undefined,
    findByFolderPath: () => undefined,
    list: () => [],
    search: () => [],
    remove: () => false,
    removeEmpty: () => 0,
    count: () => 0,
    listFolders: () => [],
    updateFolderPath: notImplemented,
    ...overrides,
  };
}

function fakeFileRepository(files: Partial<Record<number, MediaFileRow[]>> = {}): FileRepository {
  const notImplemented = (): never => {
    throw new Error('método no esperado en el doble de prueba');
  };

  return {
    upsertMany: notImplemented,
    findByPath: () => undefined,
    listByMedia: (mediaId) => files[mediaId] ?? [],
    listAllPaths: () => [],
    removeByMedia: () => 0,
    removeMissing: () => 0,
    count: () => Object.values(files).reduce((total, list) => total + (list?.length ?? 0), 0),
    updateFilePath: notImplemented,
  };
}

function fakeIndexRunRepository(lastRun: IndexRunRow | undefined): {
  latest: () => IndexRunRow | undefined;
  start: () => number;
  complete: () => void;
  fail: () => void;
  list: () => IndexRunRow[];
} {
  return {
    start: () => 1,
    complete: () => undefined,
    fail: () => undefined,
    latest: () => lastRun,
    list: () => (lastRun ? [lastRun] : []),
  };
}

function fakePlaybackRepository(): PlaybackRepository {
  return {
    save: () => ({
      mediaFileId: 1,
      positionSeconds: 0,
      durationSeconds: null,
      completedAt: null,
      lastPlayedAt: '',
      playCount: 0,
      updatedAt: '',
    }),
    get: () => undefined,
    remove: () => false,
    listInProgress: () => [],
    listRecent: () => [],
    count: () => 0,
  };
}

function fakePreferenceRepository(): PreferenceRepository {
  return {
    get: () => undefined,
    isFavorite: () => false,
    setFavorite: () => ({ mediaId: 0, favorite: false, createdAt: '', updatedAt: '' }),
    remove: () => false,
    listFavorites: () => [],
    countFavorites: () => 0,
  };
}

function fakeTagRepository(): TagRepository {
  return {
    create: () => ({ id: 1, name: 'test', color: null, createdAt: '' }),
    get: () => undefined,
    getByName: () => undefined,
    list: () => [],
    update: () => false,
    remove: () => false,
    assign: () => undefined,
    unassign: () => undefined,
    unassignAll: () => undefined,
    listByMedia: () => [],
    listMediaByTag: () => [],
    countByTag: () => 0,
  };
}

function fakeMetadataRepository(): MetadataRepository {
  return {
    get: () => undefined,
    searchByExternalId: () => undefined,
    set: () => ({
      mediaId: 0,
      provider: 'tmdb',
      externalId: 0,
      title: '',
      overview: null,
      posterPath: null,
      backdropPath: null,
      voteAverage: null,
      firstAirDate: null,
      releaseDate: null,
      originalLanguage: null,
      fetchedAt: '',
      updatedAt: '',
      localPosterPath: null,
    }),
    remove: () => false,
    removeAll: () => 0,
    listExpired: () => [],
  };
}

function fakeMetadataService(): MetadataService {
  return {
    searchTmdb: async () => {
      await Promise.resolve();
      return [];
    },
    getTmdbDetails: async () => {
      await Promise.resolve();
      return {
        mediaId: 0,
        provider: 'tmdb',
        externalId: 0,
        title: '',
        overview: null,
        posterPath: null,
        backdropPath: null,
        voteAverage: null,
        firstAirDate: null,
        releaseDate: null,
        originalLanguage: null,
        fetchedAt: '',
        updatedAt: '',
      };
    },
    searchTvdb: async () => {
      await Promise.resolve();
      return [];
    },
    getTvdbDetails: async () => {
      await Promise.resolve();
      return {
        mediaId: 0,
        provider: 'tvdb',
        externalId: 0,
        title: '',
        overview: null,
        posterPath: null,
        backdropPath: null,
        voteAverage: null,
        firstAirDate: null,
        releaseDate: null,
        originalLanguage: null,
        fetchedAt: '',
        updatedAt: '',
      };
    },
    search: async () => {
      await Promise.resolve();
      return [];
    },
    getDetails: async () => {
      await Promise.resolve();
      return {
        mediaId: 0,
        provider: 'tmdb',
        externalId: 0,
        title: '',
        overview: null,
        posterPath: null,
        backdropPath: null,
        voteAverage: null,
        firstAirDate: null,
        releaseDate: null,
        originalLanguage: null,
        fetchedAt: '',
        updatedAt: '',
      };
    },
    hasProviderConfigured: () => false,
    isProviderConfigured: () => false,
  } as unknown as MetadataService;
}

function service(
  overrides: Partial<MediaRepository> = {},
  files: Partial<Record<number, MediaFileRow[]>> = {},
) {
  return createLibraryService({
    mediaRepo: fakeMediaRepository(overrides),
    fileRepo: fakeFileRepository(files),
    indexRunRepo: fakeIndexRunRepository(undefined),
    playbackRepo: fakePlaybackRepository(),
    preferenceRepo: fakePreferenceRepository(),
    tagRepo: fakeTagRepository(),
    metadataRepo: fakeMetadataRepository(),
    metadataService: fakeMetadataService(),
    metadataConfig: { cacheTtlHours: 24, autoFetch: false },
  });
}

describe('createLibraryService', () => {
  describe('list', () => {
    it('delega en el repositorio con los filtros recibidos', () => {
      const list = vi.fn((_options?: ListMediaOptions) => [dark, darkMatter]);
      const library = service({ list });

      expect(library.list({ type: 'series', limit: 2 })).toEqual([dark, darkMatter]);
      expect(list).toHaveBeenCalledWith({ type: 'series', limit: 2 });
    });
  });

  describe('search', () => {
    it('delega en el repositorio y respeta tipo y límite', () => {
      const search = vi.fn((_query: string, _options?: SearchMediaOptions) => [dark]);
      const library = service({ search });

      expect(library.search('dar', { type: 'series', limit: 5 })).toEqual([dark]);
      expect(search).toHaveBeenCalledWith('dar', { type: 'series', limit: 5 });
    });
  });

  describe('find', () => {
    it('resuelve por id numérico y adjunta sus ficheros', () => {
      const file: MediaFileRow = {
        id: 10,
        mediaId: 1,
        season: 1,
        episode: 2,
        title: null,
        filePath: '/media/series/Dark/S01E02.mkv',
        sizeBytes: 100,
        mtimeMs: 1,
        indexedAt: '2026-01-01 00:00:00',
      };
      const library = service({ findById: (id) => (id === 1 ? dark : undefined) }, { 1: [file] });

      expect(library.find('1')).toEqual({ media: dark, files: [file] });
    });

    it('prefiere la coincidencia exacta de título sobre los resultados parciales', () => {
      const library = service({
        search: () => [darkMatter, dark],
      });

      expect(library.find('Dark').media).toEqual(dark);
    });

    it('compara títulos sin acentos ni mayúsculas', () => {
      const library = service({
        search: () => [amelie],
      });

      expect(library.find('amelie').media).toEqual(amelie);
    });

    it('falla con un error explícito si no hay coincidencias', () => {
      const library = service();

      expect(() => library.find('inexistente')).toThrow(NotFoundError);
    });

    it('falla si la consulta está vacía', () => {
      const library = service();

      expect(() => library.find('   ')).toThrow(/Indica un id o un título/);
    });

    it('pide desambiguar cuando hay varios títulos y enumera los ids', () => {
      const library = service({
        search: () => [mediaRow(7, 'Cowboy Bebop'), mediaRow(8, 'Cowboy Bebop: Knockin')],
      });

      expect(() => library.find('cowboy')).toThrow(AmbiguousError);
      expect(() => library.find('cowboy')).toThrow(/#7 Cowboy Bebop/);
      expect(() => library.find('cowboy')).toThrow(/Usa el id/);
    });

    it('acepta un número que no es id cuando coincide con un título', () => {
      const movie = mediaRow(42, '1917');
      const library = service({
        findById: () => undefined,
        search: () => [movie],
      });

      expect(library.find('1917').media).toEqual(movie);
    });
  });

  describe('stats', () => {
    it('agrega conteos y la última ejecución de indexado', () => {
      const lastRun: IndexRunRow = {
        id: 4,
        startedAt: '2026-01-01 00:00:00',
        finishedAt: '2026-01-01 00:00:03',
        filesSeen: 10,
        filesAdded: 2,
        filesRemoved: 1,
        status: 'completed',
      };
      const library = createLibraryService({
        mediaRepo: fakeMediaRepository({
          count: (type) => (type === undefined ? 3 : type === 'series' ? 2 : 0),
        }),
        fileRepo: fakeFileRepository({ 1: [], 2: [] }),
        indexRunRepo: fakeIndexRunRepository(lastRun),
        playbackRepo: fakePlaybackRepository(),
        preferenceRepo: fakePreferenceRepository(),
        tagRepo: fakeTagRepository(),
        metadataRepo: fakeMetadataRepository(),
        metadataService: fakeMetadataService(),
        metadataConfig: { cacheTtlHours: 24, autoFetch: false },
      });

      expect(library.stats()).toEqual({
        media: { total: 3, series: 2, anime: 0, movie: 0, unknown: 0 },
        files: 0,
        lastRun,
        inProgress: 0,
      });
    });

    it('devuelve lastRun null si nunca se ha indexado', () => {
      const library = service();

      expect(library.stats().lastRun).toBeNull();
      expect(library.stats().inProgress).toBe(0);
    });
  });

  describe('getOrFetchMetadata', () => {
    it('busca por título y guarda metadatos cuando no hay previos', async () => {
      const searchResult = {
        provider: 'tvmaze' as const,
        externalId: 123,
        title: 'Dark',
        overview: 'Sinopsis',
        posterPath: '/poster.jpg',
        backdropPath: null,
        voteAverage: 8.5,
        releaseDate: '2017-12-01',
        firstAirDate: null,
        originalLanguage: 'de',
        localPosterPath: null,
      };
      const details = {
        provider: 'tvmaze' as const,
        externalId: 123,
        title: 'Dark',
        overview: 'Sinopsis de Dark',
        posterPath: '/poster.jpg',
        backdropPath: null,
        voteAverage: 8.5,
        releaseDate: '2017-12-01',
        firstAirDate: null,
        originalLanguage: 'de',
        localPosterPath: null,
      };

      let setCalled = false;
      const library = createLibraryService({
        mediaRepo: fakeMediaRepository({
          findById: () => ({ ...dark, id: 1, type: 'series' }),
        }),
        fileRepo: fakeFileRepository(),
        indexRunRepo: fakeIndexRunRepository(undefined),
        playbackRepo: fakePlaybackRepository(),
        preferenceRepo: fakePreferenceRepository(),
        tagRepo: fakeTagRepository(),
        metadataRepo: {
          ...fakeMetadataRepository(),
          get: () => undefined,
          set: (input: SetMetadataInput) => {
            setCalled = true;
            expect(input.mediaId).toBe(1);
            expect(input.provider).toBe('tvmaze');
            expect(input.externalId).toBe(123);
            return {
              ...details,
              mediaId: input.mediaId,
              firstAirDate: input.releaseDate ?? null,
              fetchedAt: '',
              updatedAt: '',
            };
          },
        },
        metadataService: {
          ...fakeMetadataService(),
          hasProviderConfigured: () => true,
          isProviderConfigured: () => true,
          search: () => Promise.resolve([searchResult]),
          getDetails: () => Promise.resolve(details),
        } as unknown as MetadataService,
        metadataConfig: { cacheTtlHours: 24, autoFetch: false },
      });

      const result = await library.getOrFetchMetadata(1);
      expect(setCalled).toBe(true);
      expect(result?.title).toBe('Dark');
      expect(result?.externalId).toBe(123);
      expect(result?.provider).toBe('tvmaze');
    });

    it('retorna undefined cuando no hay proveedor ni metadatos previos', async () => {
      const library = createLibraryService({
        mediaRepo: fakeMediaRepository({
          findById: () => ({ ...dark, id: 1, type: 'series' }),
        }),
        fileRepo: fakeFileRepository(),
        indexRunRepo: fakeIndexRunRepository(undefined),
        playbackRepo: fakePlaybackRepository(),
        preferenceRepo: fakePreferenceRepository(),
        tagRepo: fakeTagRepository(),
        metadataRepo: {
          ...fakeMetadataRepository(),
          get: () => undefined,
        },
        metadataService: {
          ...fakeMetadataService(),
          hasProviderConfigured: () => false,
        } as unknown as MetadataService,
        metadataConfig: { cacheTtlHours: 24, autoFetch: false },
      });

      const result = await library.getOrFetchMetadata(1);
      expect(result).toBeUndefined();
    });

    it('usa TMDB para obtener metadatos cuando hay API key configurada y no hay previos', async () => {
      const searchResult = {
        provider: 'tvmaze' as const,
        externalId: 123,
        title: 'Dark',
        overview: 'Sinopsis',
        posterPath: '/poster.jpg',
        backdropPath: null,
        voteAverage: 8.5,
        releaseDate: '2017-12-01',
        firstAirDate: null,
        originalLanguage: 'de',
        localPosterPath: null,
      };
      const details = {
        provider: 'tvmaze' as const,
        externalId: 123,
        title: 'Dark',
        overview: 'Sinopsis de Dark',
        posterPath: '/poster.jpg',
        backdropPath: null,
        voteAverage: 8.5,
        releaseDate: '2017-12-01',
        firstAirDate: null,
        originalLanguage: 'de',
        localPosterPath: '/cache/123-poster.jpg',
      };

      let setCalled = false;
      const library = createLibraryService({
        mediaRepo: fakeMediaRepository({
          findById: () => ({ ...dark, id: 1, type: 'series' }),
        }),
        fileRepo: fakeFileRepository(),
        indexRunRepo: fakeIndexRunRepository(undefined),
        playbackRepo: fakePlaybackRepository(),
        preferenceRepo: fakePreferenceRepository(),
        tagRepo: fakeTagRepository(),
        metadataRepo: {
          ...fakeMetadataRepository(),
          get: () => undefined,
          set: (input: SetMetadataInput) => {
            setCalled = true;
            expect(input.provider).toBe('tvmaze');
            expect(input.externalId).toBe(123);
            expect(input.localPosterPath).toBe('/cache/123-poster.jpg');
            return {
              ...details,
              mediaId: input.mediaId,
              firstAirDate: null,
              fetchedAt: '',
              updatedAt: '',
            };
          },
        },
        metadataService: {
          ...fakeMetadataService(),
          hasProviderConfigured: () => true,
          isProviderConfigured: (provider: 'tvmaze' | 'jikan' | 'default') => provider === 'tvmaze',
          search: () => Promise.resolve([searchResult]),
          getDetails: () => Promise.resolve(details),
        } as unknown as MetadataService,
        metadataConfig: { cacheTtlHours: 24, autoFetch: false },
      });

      const result = await library.getOrFetchMetadata(1);
      expect(setCalled).toBe(true);
      expect(result?.title).toBe('Dark');
      expect(result?.provider).toBe('tvmaze');
      expect(result?.localPosterPath).toBe('/cache/123-poster.jpg');
    });

    it('busca por título usando TMDB cuando hay metadatos del proveedor default sin póster y se configura API key de TMDB', async () => {
      const existing = {
        mediaId: 1,
        provider: 'default' as const,
        externalId: 0,
        title: 'Dark',
        overview: null,
        posterPath: null,
        backdropPath: null,
        voteAverage: null,
        firstAirDate: null,
        releaseDate: null,
        originalLanguage: null,
        localPosterPath: null,
        fetchedAt: new Date().toISOString(),
        updatedAt: '',
      };

      const searchResult = {
        provider: 'tvmaze' as const,
        externalId: 123,
        title: 'Dark',
        overview: 'Sinopsis de Dark',
        posterPath: '/poster.jpg',
        backdropPath: null,
        voteAverage: 8.5,
        releaseDate: '2017-12-01',
        firstAirDate: null,
        originalLanguage: 'de',
        localPosterPath: null,
      };
      const details = {
        provider: 'tvmaze' as const,
        externalId: 123,
        title: 'Dark',
        overview: 'Sinopsis de Dark',
        posterPath: '/poster.jpg',
        backdropPath: null,
        voteAverage: 8.5,
        releaseDate: '2017-12-01',
        firstAirDate: null,
        originalLanguage: 'de',
        localPosterPath: '/cache/123-poster.jpg',
      };

      let searchCalled = false;
      let getDetailsCalled = false;
      let setCalled = false;
      const library = createLibraryService({
        mediaRepo: fakeMediaRepository({
          findById: () => ({ ...dark, id: 1, type: 'series' }),
        }),
        fileRepo: fakeFileRepository(),
        indexRunRepo: fakeIndexRunRepository(undefined),
        playbackRepo: fakePlaybackRepository(),
        preferenceRepo: fakePreferenceRepository(),
        tagRepo: fakeTagRepository(),
        metadataRepo: {
          ...fakeMetadataRepository(),
          get: () => existing,
          set: (input: SetMetadataInput) => {
            setCalled = true;
            expect(input.provider).toBe('tvmaze');
            expect(input.externalId).toBe(123);
            expect(input.localPosterPath).toBe('/cache/123-poster.jpg');
            return { ...details, mediaId: input.mediaId, fetchedAt: '', updatedAt: '' };
          },
        },
        metadataService: {
          ...fakeMetadataService(),
          hasProviderConfigured: () => true,
          isProviderConfigured: (provider: 'tvmaze' | 'jikan' | 'default') => provider === 'tvmaze',
          search: (query: string) => {
            searchCalled = true;
            expect(query).toBe('Dark');
            return Promise.resolve([searchResult]);
          },
          getDetails: () => {
            getDetailsCalled = true;
            return Promise.resolve(details);
          },
        } as unknown as MetadataService,
        metadataConfig: { cacheTtlHours: 24, autoFetch: false },
      });

      const result = await library.getOrFetchMetadata(1);
      expect(searchCalled).toBe(true);
      expect(getDetailsCalled).toBe(true);
      expect(setCalled).toBe(true);
      expect(result?.title).toBe('Dark');
      expect(result?.provider).toBe('tvmaze');
      expect(result?.localPosterPath).toBe('/cache/123-poster.jpg');
    });

    it('no fuerza refresco si los metadatos no expiraron y ya tienen póster (TTL vigente)', async () => {
      const existing = {
        mediaId: 1,
        provider: 'tmdb' as const,
        externalId: 123,
        title: 'Título existente',
        overview: null,
        posterPath: '/poster.jpg',
        backdropPath: null,
        voteAverage: null,
        firstAirDate: null,
        releaseDate: null,
        originalLanguage: null,
        localPosterPath: '/cache/123-poster.jpg', // Ya tiene póster
        fetchedAt: new Date().toISOString(), // Acabado de obtener
        updatedAt: '',
      };

      let setCalled = false;
      const library = createLibraryService({
        mediaRepo: fakeMediaRepository({
          findById: () => ({ ...dark, id: 1, type: 'series' }),
        }),
        fileRepo: fakeFileRepository(),
        indexRunRepo: fakeIndexRunRepository(undefined),
        playbackRepo: fakePlaybackRepository(),
        preferenceRepo: fakePreferenceRepository(),
        tagRepo: fakeTagRepository(),
        metadataRepo: {
          ...fakeMetadataRepository(),
          get: () => existing,
          set: () => {
            setCalled = true;
            return existing;
          },
        },
        metadataService: {
          ...fakeMetadataService(),
          hasProviderConfigured: () => true,
          search: () => Promise.resolve([]),
          getDetails: () =>
            Promise.resolve({
              provider: 'tmdb' as const,
              externalId: 123,
              title: 'Nuevo título',
            }),
        } as unknown as MetadataService,
        metadataConfig: { cacheTtlHours: 24, autoFetch: false },
      });

      const result = await library.getOrFetchMetadata(1);
      expect(result).toBe(existing); // Devuelve los existentes sin intentar refrescar
      expect(setCalled).toBe(false);
    });

    it('fuerza refresco de póster si los metadatos no expiraron pero no tienen póster y hay proveedor externo', async () => {
      const existing = {
        mediaId: 1,
        provider: 'tmdb' as const,
        externalId: 123,
        title: 'Título existente',
        overview: null,
        posterPath: null,
        backdropPath: null,
        voteAverage: null,
        firstAirDate: null,
        releaseDate: null,
        originalLanguage: null,
        localPosterPath: null, // Sin póster
        fetchedAt: new Date().toISOString(), // Acabado de obtener (no expirado)
        updatedAt: '',
      };

      const details = {
        provider: 'tmdb' as const,
        externalId: 123,
        title: 'Título existente',
        overview: 'Sinopsis obtenida de TMDB',
        posterPath: '/poster.jpg',
        backdropPath: null,
        voteAverage: 8.5,
        releaseDate: '2017-12-01',
        firstAirDate: null,
        originalLanguage: 'de',
        localPosterPath: '/cache/123-poster.jpg',
      };

      let setCalled = false;
      const library = createLibraryService({
        mediaRepo: fakeMediaRepository({
          findById: () => ({ ...dark, id: 1, type: 'series' }),
        }),
        fileRepo: fakeFileRepository(),
        indexRunRepo: fakeIndexRunRepository(undefined),
        playbackRepo: fakePlaybackRepository(),
        preferenceRepo: fakePreferenceRepository(),
        tagRepo: fakeTagRepository(),
        metadataRepo: {
          ...fakeMetadataRepository(),
          get: () => existing,
          set: (input: SetMetadataInput) => {
            setCalled = true;
            expect(input.localPosterPath).toBe('/cache/123-poster.jpg');
            return {
              ...details,
              mediaId: input.mediaId,
              firstAirDate: null,
              fetchedAt: '',
              updatedAt: '',
            };
          },
        },
        metadataService: {
          ...fakeMetadataService(),
          hasProviderConfigured: () => true,
          isProviderConfigured: () => true,
          search: () => Promise.resolve([]),
          getDetails: () => Promise.resolve(details),
        } as unknown as MetadataService,
        metadataConfig: { cacheTtlHours: 24, autoFetch: false },
      });

      const result = await library.getOrFetchMetadata(1);
      expect(setCalled).toBe(true); // Se forzó el refresco para obtener el póster
      expect(result?.localPosterPath).toBe('/cache/123-poster.jpg');
      expect(result?.overview).toBe('Sinopsis obtenida de TMDB');
    });
  });

  describe('clearAllMetadata', () => {
    it('llama a metadataRepo.removeAll y retorna el número de filas eliminadas', () => {
      const library = createLibraryService({
        mediaRepo: fakeMediaRepository(),
        fileRepo: fakeFileRepository(),
        indexRunRepo: fakeIndexRunRepository(undefined),
        playbackRepo: fakePlaybackRepository(),
        preferenceRepo: fakePreferenceRepository(),
        tagRepo: fakeTagRepository(),
        metadataRepo: {
          ...fakeMetadataRepository(),
          removeAll: () => 42,
        },
        metadataService: fakeMetadataService(),
        metadataConfig: { cacheTtlHours: 24, autoFetch: false },
      });

      expect(library.clearAllMetadata()).toBe(42);
    });

    it('expone metadataService desde LibraryService', () => {
      const svc = { updateConfig: vi.fn() } as unknown as MetadataService;
      const library = createLibraryService({
        mediaRepo: fakeMediaRepository(),
        fileRepo: fakeFileRepository(),
        indexRunRepo: fakeIndexRunRepository(undefined),
        playbackRepo: fakePlaybackRepository(),
        preferenceRepo: fakePreferenceRepository(),
        tagRepo: fakeTagRepository(),
        metadataRepo: fakeMetadataRepository(),
        metadataService: svc,
        metadataConfig: { cacheTtlHours: 24, autoFetch: false },
      });

      expect(library.metadataService).toBe(svc);
    });
  });

  describe('fetchMissingMetadata', () => {
    it('retorna cero cuando no hay proveedor configurado', async () => {
      const library = createLibraryService({
        mediaRepo: fakeMediaRepository({
          list: () => [{ ...dark, id: 1 }],
        }),
        fileRepo: fakeFileRepository(),
        indexRunRepo: fakeIndexRunRepository(undefined),
        playbackRepo: fakePlaybackRepository(),
        preferenceRepo: fakePreferenceRepository(),
        tagRepo: fakeTagRepository(),
        metadataRepo: fakeMetadataRepository(),
        metadataService: {
          ...fakeMetadataService(),
          hasProviderConfigured: () => false,
        } as unknown as MetadataService,
        metadataConfig: { cacheTtlHours: 24, autoFetch: false },
      });

      const result = await library.fetchMissingMetadata();
      expect(result).toEqual({ fetched: 0, failed: 0 });
    });

    it('obtiene metadatos para títulos que no los tienen', async () => {
      const details = {
        mediaId: 0,
        provider: 'tvmaze' as const,
        externalId: 123,
        title: 'Dark',
        overview: 'Sinopsis',
        posterPath: null,
        backdropPath: null,
        voteAverage: 8.5,
        firstAirDate: null,
        releaseDate: '2017-12-01',
        originalLanguage: 'de',
        fetchedAt: '',
        updatedAt: '',
        localPosterPath: null,
      };

      let fetchCalled = 0;
      const library = createLibraryService({
        mediaRepo: fakeMediaRepository({
          list: () => [
            { ...dark, id: 1 },
            { ...dark, id: 2, title: 'Other' },
          ],
          findById: (id) => (id === 1 ? { ...dark, id: 1, type: 'series' } : undefined),
        }),
        fileRepo: fakeFileRepository(),
        indexRunRepo: fakeIndexRunRepository(undefined),
        playbackRepo: fakePlaybackRepository(),
        preferenceRepo: fakePreferenceRepository(),
        tagRepo: fakeTagRepository(),
        metadataRepo: {
          ...fakeMetadataRepository(),
          get: (mediaId) => (mediaId === 2 ? details : undefined),
          set: () => {
            fetchCalled++;
            return details;
          },
        },
        metadataService: {
          ...fakeMetadataService(),
          hasProviderConfigured: () => true,
          isProviderConfigured: () => true,
          search: () => [
            {
              provider: 'tvmaze' as const,
              externalId: 123,
              title: 'Dark',
              overview: null,
              posterPath: null,
              backdropPath: null,
              voteAverage: null,
              releaseDate: null,
              originalLanguage: null,
              localPosterPath: null,
            },
          ],
          getDetails: () => details,
        } as unknown as MetadataService,
        metadataConfig: { cacheTtlHours: 24, autoFetch: false },
      });

      const result = await library.fetchMissingMetadata();
      // Solo el título 1 no tiene metadatos (el 2 ya los tiene via get)
      expect(fetchCalled).toBe(1);
      expect(result.fetched).toBe(1);
      expect(result.failed).toBe(0);
    });

    it('no intenta obtener metadatos para títulos que ya los tienen', async () => {
      const details = {
        mediaId: 1,
        provider: 'tmdb' as const,
        externalId: 123,
        title: 'Dark',
        overview: null,
        posterPath: null,
        backdropPath: null,
        voteAverage: null,
        firstAirDate: null,
        releaseDate: null,
        originalLanguage: null,
        fetchedAt: '',
        updatedAt: '',
        localPosterPath: null,
      };

      let setCalled = 0;
      const library = createLibraryService({
        mediaRepo: fakeMediaRepository({
          list: () => [{ ...dark, id: 1 }],
        }),
        fileRepo: fakeFileRepository(),
        indexRunRepo: fakeIndexRunRepository(undefined),
        playbackRepo: fakePlaybackRepository(),
        preferenceRepo: fakePreferenceRepository(),
        tagRepo: fakeTagRepository(),
        metadataRepo: {
          ...fakeMetadataRepository(),
          get: () => details, // Ya tiene metadatos
          set: () => {
            setCalled++;
            return details;
          },
        },
        metadataService: {
          ...fakeMetadataService(),
          hasProviderConfigured: () => true,
        } as unknown as MetadataService,
        metadataConfig: { cacheTtlHours: 24, autoFetch: false },
      });

      const result = await library.fetchMissingMetadata();
      expect(setCalled).toBe(0);
      expect(result.fetched).toBe(0);
      expect(result.failed).toBe(0);
    });

    it('getOrFetchMetadata con forceRefresh ignora TTL y busca en API externa aunque existan metadatos', async () => {
      const existingMetadata = {
        mediaId: 1,
        provider: 'tvmaze' as const,
        externalId: 123,
        title: 'Dark',
        overview: 'Sinopsis vieja',
        posterPath: 'https://static.tvmaze.com/uploads/images/original/123.jpg',
        backdropPath: null,
        voteAverage: null,
        firstAirDate: null,
        releaseDate: null,
        originalLanguage: null,
        fetchedAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        localPosterPath: '/cache/123-poster.jpg', // Tiene póster, no necesita refresco
      };

      const refreshedMetadata = {
        ...existingMetadata,
        overview: 'Sinopsis fresca de Dark',
        posterPath: 'https://static.tvmaze.com/uploads/images/original/456.jpg',
      };

      const searchCalledWith: string[] = [];
      const library = createLibraryService({
        mediaRepo: fakeMediaRepository({
          findById: () => ({ ...dark, id: 1, type: 'series' }),
        }),
        fileRepo: fakeFileRepository(),
        indexRunRepo: fakeIndexRunRepository(undefined),
        playbackRepo: fakePlaybackRepository(),
        preferenceRepo: fakePreferenceRepository(),
        tagRepo: fakeTagRepository(),
        metadataRepo: {
          ...fakeMetadataRepository(),
          get: () => existingMetadata,
          set: (input) => {
            Object.assign(existingMetadata, input);
            return existingMetadata;
          },
        },
        metadataService: {
          ...fakeMetadataService(),
          hasProviderConfigured: () => true,
          isProviderConfigured: () => true,
          search: (query: string) => {
            searchCalledWith.push(query);
            return [
              {
                provider: 'tvmaze' as const,
                externalId: 456,
                title: 'Dark',
                overview: 'Sinopsis fresca de Dark',
                posterPath: 'https://static.tvmaze.com/uploads/images/original/456.jpg',
                backdropPath: null,
                voteAverage: 8.5,
                releaseDate: '2017-12-01',
                originalLanguage: 'de',
                localPosterPath: null,
              },
            ];
          },
          getDetails: () => refreshedMetadata,
        } as unknown as MetadataService,
        metadataConfig: { cacheTtlHours: 24, autoFetch: false },
      });

      // Sin forceRefresh: no se debe buscar porque los metadatos no expiran y tienen póster
      const result1 = await library.getOrFetchMetadata(1);
      expect(searchCalledWith).toHaveLength(0);
      expect(result1?.overview).toBe('Sinopsis vieja');

      // Con forceRefresh y sin externalId previo: se busca por título
      const libraryNoExternal = createLibraryService({
        mediaRepo: fakeMediaRepository({
          findById: () => ({ ...dark, id: 1, type: 'series' }),
        }),
        fileRepo: fakeFileRepository(),
        indexRunRepo: fakeIndexRunRepository(undefined),
        playbackRepo: fakePlaybackRepository(),
        preferenceRepo: fakePreferenceRepository(),
        tagRepo: fakeTagRepository(),
        metadataRepo: {
          ...fakeMetadataRepository(),
          // Sin metadatos previos: fuerza búsqueda por título
          get: () => undefined,
          set: () => refreshedMetadata,
        },
        metadataService: {
          ...fakeMetadataService(),
          hasProviderConfigured: () => true,
          isProviderConfigured: () => true,
          search: (query: string) => {
            searchCalledWith.push(query);
            return [
              {
                provider: 'tvmaze' as const,
                externalId: 456,
                title: 'Dark',
                overview: 'Sinopsis fresca de Dark',
                posterPath: 'https://static.tvmaze.com/uploads/images/original/456.jpg',
                backdropPath: null,
                voteAverage: 8.5,
                releaseDate: '2017-12-01',
                originalLanguage: 'de',
                localPosterPath: null,
              },
            ];
          },
          getDetails: () => refreshedMetadata,
        } as unknown as MetadataService,
        metadataConfig: { cacheTtlHours: 24, autoFetch: false },
      });

      const result2 = await libraryNoExternal.getOrFetchMetadata(1, { forceRefresh: true });
      expect(searchCalledWith).toHaveLength(1);
      expect(result2?.overview).toBe('Sinopsis fresca de Dark');
    });

    it('getOrFetchMetadata limpia el título sucio antes de buscar en API externa', async () => {
      const searchCalledWith: string[] = [];
      const library = createLibraryService({
        mediaRepo: fakeMediaRepository({
          findById: () => ({ ...dark, id: 1, type: 'series' }),
        }),
        fileRepo: fakeFileRepository(),
        indexRunRepo: fakeIndexRunRepository(undefined),
        playbackRepo: fakePlaybackRepository(),
        preferenceRepo: fakePreferenceRepository(),
        tagRepo: fakeTagRepository(),
        metadataRepo: {
          ...fakeMetadataRepository(),
          get: () => undefined, // No hay metadatos previos
          set: () => ({
            mediaId: 1,
            provider: 'tvmaze' as const,
            externalId: 456,
            title: 'Dark',
            overview: '',
            posterPath: null,
            backdropPath: null,
            voteAverage: null,
            firstAirDate: null,
            releaseDate: null,
            originalLanguage: null,
            fetchedAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            localPosterPath: null,
          }),
        },
        metadataService: {
          ...fakeMetadataService(),
          hasProviderConfigured: () => true,
          isProviderConfigured: () => true,
          search: (query: string) => {
            searchCalledWith.push(query);
            return [];
          },
          getDetails: () => ({
            provider: 'tvmaze' as const,
            externalId: 456,
            title: 'Dark',
            overview: '',
            posterPath: null,
            backdropPath: null,
            voteAverage: null,
            releaseDate: null,
            originalLanguage: null,
            localPosterPath: null,
          }),
        } as unknown as MetadataService,
        metadataConfig: { cacheTtlHours: 24, autoFetch: false },
      });

      await library.getOrFetchMetadata(1, { forceRefresh: true });

      // El título 'Dark [Judas] S01E01 1080p' debe limpiarse a 'Dark'
      expect(searchCalledWith).toHaveLength(1);
      expect(searchCalledWith[0]).toBe('Dark');
    });
  });
});
