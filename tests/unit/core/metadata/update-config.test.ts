import { describe, expect, it, vi } from 'vitest';

import { MetadataService } from '../../../../src/core/metadata/service.js';

const logger = {
  debug: vi.fn(),
  warn: vi.fn(),
  error: vi.fn(),
};

function okResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

function fetchMockCalls(): unknown[][] {
  return (globalThis.fetch as unknown as { mock: { calls: unknown[][] } }).mock.calls;
}

describe('MetadataService.updateConfig (proveedores públicos)', () => {
  it('updateConfig es un no-op: los proveedores públicos no necesitan claves API', () => {
    const service = new MetadataService({ logger });

    expect(service.isProviderConfigured('tvmaze')).toBe(true);
    expect(service.isProviderConfigured('jikan')).toBe(true);
    expect(service.isProviderConfigured('default')).toBe(true);

    service.updateConfig({});
    expect(service.isProviderConfigured('tvmaze')).toBe(true);
    expect(service.isProviderConfigured('jikan')).toBe(true);
  });

  it('searchTvMaze funciona sin configuración previa', async () => {
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
    const results = await service.searchTvMaze('Severance');

    expect(fetchMockCalls()).toHaveLength(1);
    expect(results.length).toBe(1);
    expect(results[0]!.provider).toBe('tvmaze');
    expect(results[0]!.title).toBe('Severance');
  });

  it('searchJikan funciona sin configuración previa', async () => {
    globalThis.fetch = vi.fn().mockResolvedValueOnce(
      okResponse({
        data: [
          {
            mal_id: 123,
            title: 'One Piece',
            title_english: 'One Piece',
            title_japanese: null,
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

    const service = new MetadataService({ logger });
    const results = await service.searchJikan('One Piece');

    expect(fetchMockCalls()).toHaveLength(1);
    expect(results.length).toBe(1);
    expect(results[0]!.provider).toBe('jikan');
    expect(results[0]!.title).toBe('One Piece');
  });
});
