import { describe, it, expect, vi, beforeEach } from 'vitest';

import { listMedia, searchMedia } from './tauri';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]): unknown => mockInvoke(...args) as unknown,
}));

beforeEach(() => {
  mockInvoke.mockReset();
});

describe('listMedia', () => {
  it('invoca list_media_cmd con filtros de tipo y paginación', async () => {
    mockInvoke.mockResolvedValue({ media: [], total: 0 });

    await listMedia('/ruta/db', { type: 'movie', limit: 10, offset: 0, orderBy: 'title' });

    expect(mockInvoke).toHaveBeenCalledWith('list_media_cmd', {
      db_path: '/ruta/db',
      type: 'movie',
      limit: 10,
      offset: 0,
      order_by: 'title',
    });
  });

  it('usa valores por defecto cuando no se proveen opciones', async () => {
    mockInvoke.mockResolvedValue({ media: [], total: 0 });

    await listMedia();

    expect(mockInvoke).toHaveBeenCalledWith('list_media_cmd', {
      db_path: 'acryl.db',
      type: undefined,
      limit: undefined,
      offset: undefined,
      order_by: undefined,
    });
  });
});

describe('searchMedia', () => {
  it('invoca search_media_cmd con query y filtros', async () => {
    mockInvoke.mockResolvedValue([]);

    await searchMedia('/ruta/db', 'bebop', { type: 'anime', limit: 5 });

    expect(mockInvoke).toHaveBeenCalledWith('search_media_cmd', {
      db_path: '/ruta/db',
      query: 'bebop',
      type: 'anime',
      limit: 5,
    });
  });

  it('usa la ruta por defecto cuando no se provee', async () => {
    mockInvoke.mockResolvedValue([]);

    await searchMedia('db', 'query');

    expect(mockInvoke).toHaveBeenCalledWith('search_media_cmd', {
      db_path: 'db',
      query: 'query',
      type: undefined,
      limit: undefined,
    });
  });
});
