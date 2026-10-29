import { describe, it, expect, vi, beforeEach } from 'vitest';
import { listMedia } from './tauri';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

beforeEach(() => {
  mockInvoke.mockReset();
});

describe('listMedia', () => {
  it('invoca list_media_cmd con db_path y query', async () => {
    const medios = [
      { id: 1, type: 'movie', title: 'X', sort_title: 'x', year: 2000, folder_path: '/a' },
    ];
    mockInvoke.mockResolvedValue(medios);

    const resultado = await listMedia('/ruta/db', 'bebop');

    expect(mockInvoke).toHaveBeenCalledWith('list_media_cmd', {
      db_path: '/ruta/db',
      query: 'bebop',
    });
    expect(resultado).toEqual(medios);
  });

  it('usa la ruta por defecto y query undefined cuando no se proveen', async () => {
    mockInvoke.mockResolvedValue([]);

    await listMedia();

    expect(mockInvoke).toHaveBeenCalledWith('list_media_cmd', {
      db_path: 'media-manager.db',
      query: undefined,
    });
  });
});
