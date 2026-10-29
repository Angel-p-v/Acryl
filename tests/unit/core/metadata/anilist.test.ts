import { describe, expect, it, vi } from 'vitest';

import { MetadataService } from '../../../../src/core/metadata/service.js';
import { MetadataError } from '../../../../src/shared/errors.js';

const logger = {
  debug: vi.fn(),
  warn: vi.fn(),
  error: vi.fn(),
};

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

describe('MetadataService (Jikan)', () => {
  it('isProviderConfigured devuelve true para jikan', () => {
    const service = new MetadataService(jikanConfig);
    expect(service.isProviderConfigured('jikan')).toBe(true);
    expect(service.isProviderConfigured('tvmaze')).toBe(true);
    expect(service.isProviderConfigured('default')).toBe(true);
  });

  it('hasProviderConfigured es true con proveedor público', () => {
    const service = new MetadataService(jikanConfig);
    expect(service.hasProviderConfigured()).toBe(true);
  });

  it('searchJikan funciona sin API key', async () => {
    globalThis.fetch = vi.fn().mockResolvedValueOnce(
      new Response(JSON.stringify({ data: [], pagination: { last_visible_page: 1 } }), {
        status: 200,
      }),
    );
    const service = new MetadataService(noKeyConfig);
    const result = await service.searchJikan('one piece');
    expect(result).toEqual([]);
  });

  it('searchJikan mapea resultados de Jikan', async () => {
    globalThis.fetch = vi.fn().mockResolvedValueOnce(
      okResponse(200, {
        data: [
          {
            mal_id: 123,
            title: 'One Piece',
            title_english: 'One Piece',
            title_japanese: 'ワンピース',
            title_synonyms: [],
            type: 'TV',
            source: 'Manga',
            episodes: 1000,
            status: 'Finished Airing',
            airing: false,
            aired: { from: '1999-10-20', to: null, string: 'Oct 20, 1999' },
            duration: '24 min',
            rating: 'PG-13',
            score: 8.7,
            ranked: null,
            popularity: null,
            members: null,
            favorites: null,
            synopsis: 'Un pirata busca el tesoro.',
            background: null,
            season: 'fall',
            year: 1999,
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
    const results = await service.searchJikan('one piece');

    expect(results.length).toBe(1);
    expect(results[0]!.provider).toBe('jikan');
    expect(results[0]!.externalId).toBe(123);
    expect(results[0]!.title).toBe('One Piece');
    expect(results[0]!.overview).toBe('Un pirata busca el tesoro.');
    expect(results[0]!.voteAverage).toBe(8.7);
    expect(results[0]!.releaseDate).toBe('1999-10-20');
    expect(results[0]!.originalLanguage).toBe('ja');
    expect(results[0]!.posterPath).toBe('/poster.jpg');
  });

  it('searchJikan devuelve array vacío cuando no hay resultados', async () => {
    globalThis.fetch = vi.fn().mockResolvedValueOnce(
      new Response(JSON.stringify({ data: [], pagination: { last_visible_page: 1 } }), {
        status: 200,
      }),
    );

    const service = new MetadataService(jikanConfig);
    const results = await service.searchJikan('inexistente');
    expect(results).toEqual([]);
  });

  it('getJikanDetails mapea detalles de Jikan', async () => {
    // Jikan v4 devuelve el objeto anime directamente (sin envolver en { data: ... }).
    globalThis.fetch = vi.fn().mockResolvedValueOnce(
      okResponse(200, {
        mal_id: 456,
        title: 'Naruto',
        title_english: 'Naruto',
        title_japanese: 'NARUTO',
        title_synonyms: [],
        type: 'TV',
        source: 'Manga',
        episodes: 220,
        status: 'Finished Airing',
        airing: false,
        aired: { from: '2002-10-03', to: '2007-02-08', string: 'Oct 3, 2002' },
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
      }),
    );

    const service = new MetadataService(jikanConfig);
    const details = await service.getJikanDetails(456);

    expect(details.provider).toBe('jikan');
    expect(details.externalId).toBe(456);
    expect(details.title).toBe('Naruto');
    expect(details.overview).toBe('Un ninja busca ser Hokage.');
    expect(details.voteAverage).toBe(8.2);
    expect(details.releaseDate).toBe('2002-10-03');
    expect(details.originalLanguage).toBe('ja');
    expect(details.posterPath).toBe('/poster.jpg');
  });

  it('getJikanDetails lanza MetadataError cuando no encuentra el anime', async () => {
    globalThis.fetch = vi.fn().mockResolvedValueOnce(errorResponse(404));
    const service = new MetadataService(jikanConfig);
    await expect(service.getJikanDetails(999999)).rejects.toBeInstanceOf(MetadataError);
  });

  it('getJikanDetails funciona sin API key', async () => {
    globalThis.fetch = vi.fn().mockResolvedValueOnce(
      okResponse(200, {
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
      }),
    );
    const service = new MetadataService({ logger });
    const details = await service.getJikanDetails(456);
    expect(details.provider).toBe('jikan');
    expect(details.externalId).toBe(456);
  });

  it('search usa Jikan como fallback para anime', async () => {
    globalThis.fetch = vi
      .fn()
      .mockResolvedValueOnce(new Response(JSON.stringify({ error: 'Not Found' }), { status: 404 }))
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
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
          { status: 200, headers: { 'Content-Type': 'application/json' } },
        ),
      );

    const service = new MetadataService({ logger });
    const results = await service.search('Naruto');

    expect(results.length).toBe(1);
    expect(results[0]!.provider).toBe('jikan');
  });

  it('getDetails con provider jikan llama a getJikanDetails', async () => {
    globalThis.fetch = vi.fn().mockResolvedValueOnce(
      okResponse(200, {
        mal_id: 123,
        title: 'Severance',
        title_english: 'Severance',
        title_japanese: null,
        title_synonyms: [],
        type: 'TV',
        source: 'Original',
        episodes: 9,
        status: 'Released',
        airing: false,
        aired: { from: '2022-02-18', to: null, string: 'Feb 18, 2022' },
        duration: '60 min',
        rating: 'TV-MA',
        score: 8.7,
        ranked: null,
        popularity: null,
        members: null,
        favorites: null,
        synopsis: 'Una oficina donde...',
        background: null,
        season: 'winter',
        year: 2022,
        broadcast: null,
        producers: [],
        licensors: [],
        studios: [],
        genres: [{ mal_id: 1, name: 'Drama' }],
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
      }),
    );

    const service = new MetadataService({ logger });
    const details = await service.getDetails(123, 'jikan', 'tv');

    expect(details.provider).toBe('jikan');
    expect(details.title).toBe('Severance');
  });
});
