import { describe, expect, it, vi } from 'vitest';

import { MetadataService } from '../../../../src/core/metadata/service.js';

const logger = {
  debug: vi.fn(),
  warn: vi.fn(),
  error: vi.fn(),
};

function okResponse(body: unknown = {}, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

function fetchMockCalls(): unknown[][] {
  return (globalThis.fetch as unknown as { mock: { calls: unknown[][] } }).mock.calls;
}

describe('MetadataService (TVMaze - proveedor público gratuito)', () => {
  it('hasProviderConfigured es siempre true', () => {
    const service = new MetadataService({ logger });
    expect(service.hasProviderConfigured()).toBe(true);
  });

  it('isProviderConfigured devuelve true para todos los proveedores públicos', () => {
    const service = new MetadataService({ logger });
    expect(service.isProviderConfigured('tvmaze')).toBe(true);
    expect(service.isProviderConfigured('jikan')).toBe(true);
    expect(service.isProviderConfigured('default')).toBe(true);
  });

  it('searchTvMaze mapea resultados de TVMaze', async () => {
    globalThis.fetch = vi.fn().mockResolvedValueOnce(
      okResponse({
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
        image: null,
        summary: '<p>Una oficina donde...</p>',
        updated: 1234567890,
        _links: { self: { href: 'https://api.tvmaze.com/shows/123' } },
      }),
    );

    const service = new MetadataService({ logger });
    const results = await service.searchTvMaze('Severance');

    expect(fetchMockCalls()).toHaveLength(1);
    expect(results.length).toBe(1);
    expect(results[0]!.provider).toBe('tvmaze');
    expect(results[0]!.externalId).toBe(123);
    expect(results[0]!.title).toBe('Severance');
    expect(results[0]!.overview).toBe('Una oficina donde...');
    expect(results[0]!.voteAverage).toBe(8.5);
    expect(results[0]!.releaseDate).toBe('2022-02-18');
    expect(results[0]!.originalLanguage).toBe('English');
    expect(results[0]!.posterPath).toBeNull();
  });

  it('searchTvMaze devuelve array vacío cuando TVMaze responde 404', async () => {
    globalThis.fetch = vi
      .fn()
      .mockResolvedValueOnce(new Response(JSON.stringify({ error: 'Not Found' }), { status: 404 }));
    const service = new MetadataService({ logger });
    const results = await service.searchTvMaze('NoExiste');
    expect(results).toEqual([]);
  });

  it('getTvMazeDetails mapea detalles de TVMaze', async () => {
    globalThis.fetch = vi.fn().mockResolvedValueOnce(
      okResponse({
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
        image: {
          medium: null,
          original: 'https://static.tvmaze.com/uploads/images/original/123.jpg',
        },
        summary: '<p>Una oficina donde...</p>',
        updated: 1234567890,
        _links: { self: { href: 'https://api.tvmaze.com/shows/123' } },
      }),
    );

    const service = new MetadataService({ logger });
    const details = await service.getTvMazeDetails(123);

    expect(details.provider).toBe('tvmaze');
    expect(details.externalId).toBe(123);
    expect(details.title).toBe('Severance');
    expect(details.overview).toBe('Una oficina donde...');
    expect(details.voteAverage).toBe(8.5);
    expect(details.releaseDate).toBe('2022-02-18');
    expect(details.originalLanguage).toBe('English');
    expect(details.posterPath).toBe('https://static.tvmaze.com/uploads/images/original/123.jpg');
  });

  it('getTvMazeDetails trata imagen nula como null', async () => {
    globalThis.fetch = vi.fn().mockResolvedValueOnce(
      okResponse({
        id: 999,
        name: 'Sin póster',
        type: 'Scripted',
        language: null,
        genres: [],
        status: 'Ended',
        runtime: null,
        premiered: null,
        officialSite: null,
        schedule: { time: null, days: [] },
        rating: { average: null },
        weight: 0,
        network: null,
        webChannel: null,
        externals: { tvrage: null, thetvdb: null, imdb: null },
        image: null,
        summary: null,
        updated: 1234567890,
        _links: { self: { href: 'https://api.tvmaze.com/shows/999' } },
      }),
    );

    const service = new MetadataService({ logger });
    const details = await service.getTvMazeDetails(999);

    expect(details.posterPath).toBeNull();
    expect(details.voteAverage).toBeNull();
    expect(details.releaseDate).toBeNull();
    expect(details.originalLanguage).toBeNull();
    expect(details.overview).toBeNull();
  });

  it('getTvMazeDetails lanza error cuando TVMaze no encuentra el título', async () => {
    globalThis.fetch = vi
      .fn()
      .mockResolvedValueOnce(new Response(JSON.stringify({ error: 'Not Found' }), { status: 404 }));
    const service = new MetadataService({ logger });
    await expect(service.getTvMazeDetails(999999)).rejects.toThrow('TVMaze API respondió 404');
  });

  it('search usa TVMaze como proveedor principal', async () => {
    globalThis.fetch = vi
      .fn()
      .mockResolvedValueOnce(
        okResponse({
          id: 123,
          name: 'Severance',
          type: 'Scripted',
          language: 'English',
          genres: ['Drama'],
          status: 'Ended',
          runtime: 60,
          premiered: '2022-02-18',
          officialSite: null,
          schedule: { time: '21:00', days: ['Friday'] },
          rating: { average: 8.5 },
          weight: 90,
          network: null,
          webChannel: null,
          externals: { tvrage: null, thetvdb: null, imdb: null },
          image: { medium: null, original: null },
          summary: null,
          updated: 1234567890,
          _links: { self: { href: 'https://api.tvmaze.com/shows/123' } },
        }),
      )
      .mockResolvedValueOnce(new Response(JSON.stringify({ error: 'Not Found' }), { status: 404 }));

    const service = new MetadataService({ logger });
    const results = await service.search('Severance');

    expect(fetchMockCalls()).toHaveLength(2);
    expect(results.length).toBe(1);
    expect(results[0]!.provider).toBe('tvmaze');
  });

  it('getDetails con provider tvmaze llama a getTvMazeDetails', async () => {
    globalThis.fetch = vi.fn().mockResolvedValueOnce(
      okResponse({
        id: 123,
        name: 'Severance',
        type: 'Scripted',
        language: 'English',
        genres: ['Drama'],
        status: 'Ended',
        runtime: 60,
        premiered: '2022-02-18',
        officialSite: null,
        schedule: { time: '21:00', days: ['Friday'] },
        rating: { average: 8.5 },
        weight: 90,
        network: null,
        webChannel: null,
        externals: { tvrage: null, thetvdb: null, imdb: null },
        image: { medium: null, original: null },
        summary: null,
        updated: 1234567890,
        _links: { self: { href: 'https://api.tvmaze.com/shows/123' } },
      }),
    );

    const service = new MetadataService({ logger });
    const details = await service.getDetails(123, 'tvmaze', 'tv');

    expect(details.provider).toBe('tvmaze');
    expect(details.title).toBe('Severance');
  });

  it('getDetails con provider default devuelve resultado vacío', async () => {
    const service = new MetadataService({ logger });
    const details = await service.getDetails(999, 'default', 'tv');

    expect(details.provider).toBe('default');
    expect(details.externalId).toBe(999);
    expect(details.title).toBe('');
    expect(details.overview).toBeNull();
    expect(details.posterPath).toBeNull();
  });
});
