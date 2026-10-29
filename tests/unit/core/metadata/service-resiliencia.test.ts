import { describe, expect, it, vi } from 'vitest';

import { MetadataService } from '../../../../src/core/metadata/service.js';
import { MetadataError } from '../../../../src/shared/errors.js';

const logger = {
  debug: vi.fn(),
  warn: vi.fn(),
  error: vi.fn(),
};

const tvmazeConfig = { logger };
const jikanConfig = { logger };
const noKeyConfig = { logger };

function okResponse(status = 200, body: unknown = {}): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

function errorResponse(status: number): Response {
  return new Response(JSON.stringify({ error: 'error' }), { status });
}

describe('MetadataService resiliencia', () => {
  describe('searchTvMaze', () => {
    it('lanza MetadataError cuando TVMaze responde 429', async () => {
      globalThis.fetch = vi.fn().mockResolvedValueOnce(errorResponse(429));
      const service = new MetadataService(tvmazeConfig);
      await expect(service.searchTvMaze('sever')).rejects.toBeInstanceOf(MetadataError);
    });

    it('lanza MetadataError cuando TVMaze responde 500', async () => {
      globalThis.fetch = vi.fn().mockResolvedValueOnce(errorResponse(500));
      const service = new MetadataService(tvmazeConfig);
      await expect(service.searchTvMaze('sever')).rejects.toBeInstanceOf(MetadataError);
    });

    it('propaga error de red', async () => {
      globalThis.fetch = vi.fn().mockRejectedValueOnce(new TypeError('fetch failed'));
      const service = new MetadataService(tvmazeConfig);
      await expect(service.searchTvMaze('sever')).rejects.toThrow('fetch failed');
    });

    it('devuelve resultados mapeados con respuesta exitosa', async () => {
      globalThis.fetch = vi.fn().mockResolvedValueOnce(
        okResponse(200, {
          id: 123,
          name: 'Severance',
          type: 'Scripted',
          language: 'English',
          genres: ['Drama', 'Thriller'],
          status: 'Ended',
          runtime: 60,
          premiered: '2022-02-18',
          officialSite: null,
          schedule: { time: '21:00', days: ['Friday'] },
          rating: { average: 8.5 },
          weight: 90,
          network: { name: 'Apple TV+', country: { name: 'United States' } },
          webChannel: null,
          externals: { tvrage: null, thetvdb: null, imdb: 'tt1375666' },
          image: { medium: '/poster.jpg', original: '/poster.jpg' },
          summary: '<p>Una serie</p>',
          updated: 1234567890,
          _links: { self: { href: 'https://api.tvmaze.com/shows/123' } },
        }),
      );
      const service = new MetadataService(tvmazeConfig);
      const results = await service.searchTvMaze('sever');

      expect(results).toHaveLength(1);
      expect(results[0]).toMatchObject({
        provider: 'tvmaze',
        externalId: 123,
        title: 'Severance',
        overview: 'Una serie',
        posterPath: '/poster.jpg',
        voteAverage: 8.5,
      });
    });
  });

  describe('getTvMazeDetails', () => {
    it('usa póster cacheado cuando cachePoster falla', async () => {
      globalThis.fetch = vi
        .fn()
        .mockResolvedValueOnce(
          okResponse(200, {
            id: 456,
            name: 'Película',
            type: 'Scripted',
            language: 'Spanish',
            genres: ['Drama'],
            status: 'Ended',
            runtime: 60,
            premiered: '2023-01-01',
            officialSite: null,
            schedule: { time: null, days: [] },
            rating: { average: 7.0 },
            weight: 0,
            network: null,
            webChannel: null,
            externals: { tvrage: null, thetvdb: null, imdb: null },
            image: { medium: null, original: '/poster.jpg' },
            summary: '<p>Sinopsis</p>',
            updated: 1234567890,
            _links: { self: { href: 'https://api.tvmaze.com/shows/456' } },
          }),
        )
        .mockRejectedValueOnce(new TypeError('fetch failed'));

      const service = new MetadataService(tvmazeConfig);
      const result = await service.getTvMazeDetails(456, 'tv');

      expect(result).toMatchObject({
        provider: 'tvmaze',
        externalId: 456,
        title: 'Película',
        localPosterPath: null,
      });
    });

    it('descarga y guarda el póster de TVMaze', async () => {
      // Buffer JPEG válido: magic bytes 0xff 0xd8 0xff + relleno para superar los 100 bytes mínimos
      const posterBuffer = Buffer.concat([
        Buffer.from([0xff, 0xd8, 0xff, 0xe0]),
        Buffer.alloc(100, 0x00),
      ]);

      globalThis.fetch = vi.fn((input: string | URL | Request) => {
        const url =
          typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
        if (url.includes('api.tvmaze.com/shows/')) {
          return Promise.resolve(
            okResponse(200, {
              id: 789,
              name: 'Película 2',
              type: 'Scripted',
              language: 'English',
              genres: ['Drama'],
              status: 'Ended',
              runtime: 60,
              premiered: '2024-01-01',
              officialSite: null,
              schedule: { time: null, days: [] },
              rating: { average: 8.0 },
              weight: 0,
              network: null,
              webChannel: null,
              externals: { tvrage: null, thetvdb: null, imdb: null },
              image: { medium: '/poster.jpg', original: '/nuevo-poster.jpg' },
              summary: '<p>Sinopsis 2</p>',
              updated: 1234567890,
              _links: { self: { href: 'https://api.tvmaze.com/shows/789' } },
            }),
          );
        }
        // Descarga del póster: respuesta con buffer de imagen
        return Promise.resolve(
          new Response(posterBuffer, {
            status: 200,
            headers: { 'Content-Type': 'image/jpeg' },
          }),
        );
      });

      const service = new MetadataService(tvmazeConfig);
      const result = await service.getTvMazeDetails(789, 'tv');

      expect(result.localPosterPath).not.toBeNull();
    });
  });

  describe('searchJikan', () => {
    it('lanza MetadataError cuando Jikan responde 429', async () => {
      globalThis.fetch = vi.fn().mockResolvedValueOnce(errorResponse(429));
      const service = new MetadataService(jikanConfig);
      await expect(service.searchJikan('sever')).rejects.toBeInstanceOf(MetadataError);
    });

    it('propaga error de red', async () => {
      globalThis.fetch = vi.fn().mockRejectedValueOnce(new TypeError('ECONNREFUSED'));
      const service = new MetadataService(jikanConfig);
      await expect(service.searchJikan('sever')).rejects.toThrow('ECONNREFUSED');
    });

    it('devuelve resultados mapeados con respuesta exitosa', async () => {
      globalThis.fetch = vi.fn().mockResolvedValueOnce(
        okResponse(200, {
          data: [
            {
              mal_id: 789,
              title: 'Serie TV',
              title_english: 'Serie TV',
              title_japanese: null,
              title_synonyms: [],
              type: 'TV',
              source: 'Original',
              episodes: 12,
              status: 'Finished Airing',
              airing: false,
              aired: { from: '2021-01-01', to: null, string: 'Ene 2021' },
              duration: '24 min',
              rating: 'PG-13',
              score: 8.5,
              ranked: null,
              popularity: null,
              members: null,
              favorites: null,
              synopsis: 'Una serie de TV',
              background: null,
              season: 'winter',
              year: 2021,
              broadcast: null,
              producers: [],
              licensors: [],
              studios: [],
              genres: [{ mal_id: 1, name: 'Action' }],
              explicit_genres: [],
              themes: [],
              demographic: null,
              images: {
                jpg: {
                  image_url: '/poster.jpg',
                  small_image_url: '/poster.jpg',
                  large_image_url: '/poster.jpg',
                },
                webp: {
                  image_url: '/poster.jpg',
                  small_image_url: '/poster.jpg',
                  large_image_url: '/poster.jpg',
                },
              },
              trailer: null,
            },
          ],
          pagination: { last_visible_page: 1 },
        }),
      );
      const service = new MetadataService(jikanConfig);
      const results = await service.searchJikan('serie');

      expect(results).toHaveLength(1);
      expect(results[0]).toMatchObject({
        provider: 'jikan',
        externalId: 789,
        title: 'Serie TV',
        posterPath: '/poster.jpg',
      });
    });
  });

  describe('search con proveedores públicos', () => {
    it('search funciona sin API keys', async () => {
      globalThis.fetch = vi
        .fn()
        .mockResolvedValueOnce(
          new Response(JSON.stringify({ error: 'Not Found' }), { status: 404 }),
        )
        .mockResolvedValueOnce(
          okResponse(200, {
            data: [
              {
                mal_id: 456,
                title: 'Naruto',
                title_english: 'Naruto',
                title_japanese: null,
                title_synonyms: [],
                type: 'TV',
                source: 'Manga',
                episodes: 220,
                status: 'Finished Airing',
                airing: false,
                aired: { from: '2002-10-03', to: null, string: 'Oct 3, 2002' },
                duration: '23 min',
                rating: 'PG-13',
                score: 8.2,
                ranked: null,
                popularity: null,
                members: null,
                favorites: null,
                synopsis: 'Un ninja busca ser Hokage.',
                background: null,
                season: 'fall',
                year: 2002,
                broadcast: null,
                producers: [],
                licensors: [],
                studios: [],
                genres: [],
                explicit_genres: [],
                themes: [],
                demographic: null,
                images: {
                  jpg: { image_url: null, small_image_url: null, large_image_url: null },
                  webp: { image_url: null, small_image_url: null, large_image_url: null },
                },
                trailer: null,
              },
            ],
            pagination: { last_visible_page: 1 },
          }),
        );

      const service = new MetadataService(noKeyConfig);
      const results = await service.search('Naruto');

      expect(results.length).toBe(1);
      expect(results[0]!.provider).toBe('jikan');
    });
  });
});
