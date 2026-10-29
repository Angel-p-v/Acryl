import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';

import { withCommandContext, type CommandContext } from '../../../src/commands/context.js';
import { runListCommand, parseListOptions } from '../../../src/commands/list.js';
import { runShowCommand } from '../../../src/commands/show.js';

vi.mock('../../../src/commands/context.js', () => ({
  withCommandContext: vi.fn(),
}));

type LibraryMock = {
  list: ReturnType<typeof vi.fn>;
  count: ReturnType<typeof vi.fn>;
  find: ReturnType<typeof vi.fn>;
  stats: ReturnType<typeof vi.fn>;
};

function contextWith(): { library: LibraryMock } {
  return {
    library: {
      list: vi.fn(),
      count: vi.fn(),
      find: vi.fn(),
      stats: vi.fn(),
    },
  };
}

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  vi.restoreAllMocks();
});

function mockCommandContext(): { library: LibraryMock } {
  const ctx = contextWith();
  vi.mocked(withCommandContext).mockImplementation(async (_opts, run) => {
    return await run(ctx as unknown as CommandContext);
  });
  return ctx;
}

describe('runListCommand', () => {
  it('lanza error cuando export es un formato invalido', async () => {
    mockCommandContext();
    await expect(runListCommand({ export: 'xml' as unknown as 'json' | 'csv' })).rejects.toThrow(
      'Formato de exportación no válido',
    );
  });

  it('imprime JSON cuando json es true', async () => {
    const logSpy = vi.fn();
    vi.spyOn(console, 'log').mockImplementation(logSpy);
    const ctx = mockCommandContext();
    ctx.library.list.mockReturnValue([]);
    ctx.library.count.mockReturnValue(0);

    await runListCommand({ json: true });

    expect(withCommandContext).toHaveBeenCalledTimes(1);
    expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('"total"'));
    expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('"count"'));
  });

  it('imprime JSON cuando export es json', async () => {
    const logSpy = vi.fn();
    vi.spyOn(console, 'log').mockImplementation(logSpy);
    const ctx = mockCommandContext();
    ctx.library.list.mockReturnValue([]);
    ctx.library.count.mockReturnValue(0);

    await runListCommand({ export: 'json' });

    expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('"total"'));
  });

  it('imprime CSV cuando export es csv', async () => {
    const logSpy = vi.fn();
    vi.spyOn(console, 'log').mockImplementation(logSpy);
    const ctx = mockCommandContext();
    ctx.library.list.mockReturnValue([]);
    ctx.library.count.mockReturnValue(0);

    await runListCommand({ export: 'csv' });

    expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('id,type,title'));
  });

  it('imprime "No hay titulos indexados" cuando total es 0', async () => {
    const logSpy = vi.fn();
    vi.spyOn(console, 'log').mockImplementation(logSpy);
    const ctx = mockCommandContext();
    ctx.library.list.mockReturnValue([]);
    ctx.library.count.mockReturnValue(0);

    await runListCommand({});

    expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('No hay títulos indexados'));
  });

  it('imprime "No hay resultados en esta pagina" cuando items vacio pero total > 0', async () => {
    const logSpy = vi.fn();
    vi.spyOn(console, 'log').mockImplementation(logSpy);
    const ctx = mockCommandContext();
    ctx.library.list.mockReturnValue([]);
    ctx.library.count.mockReturnValue(10);

    await runListCommand({ offset: 50 });

    expect(logSpy).toHaveBeenCalledWith(
      expect.stringContaining('No hay resultados en esta página'),
    );
  });

  it('imprime listado con filtros aplicados', async () => {
    const logSpy = vi.fn();
    vi.spyOn(console, 'log').mockImplementation(logSpy);
    const ctx = mockCommandContext();
    const items = [{ id: 1, type: 'movie', title: 'Test', year: 2020, folderPath: '/x/m.mkv' }];
    ctx.library.list.mockReturnValue(items);
    ctx.library.count.mockReturnValue(1);

    await runListCommand({
      type: 'movie',
      favorites: true,
      unwatched: true,
      tags: [1, 2],
      limit: 10,
      offset: 0,
      sort: 'recent',
    });

    const output = logSpy.mock.calls.map((args) => String(args[0])).join('\n');
    expect(output).toContain('tipo: movie');
    expect(output).toContain('favoritos');
    expect(output).toContain('no vistos');
    expect(output).toContain('tag: 1,2');
    expect(output).toContain('orden: recent');
  });

  it('usa valores por defecto cuando no se pasan opciones', async () => {
    const logSpy = vi.fn();
    vi.spyOn(console, 'log').mockImplementation(logSpy);
    const ctx = mockCommandContext();
    ctx.library.list.mockReturnValue([]);
    ctx.library.count.mockReturnValue(0);

    await runListCommand({});

    expect(ctx.library.list).toHaveBeenCalledWith({
      limit: 50,
      offset: 0,
      orderBy: 'title',
    });
    expect(ctx.library.count).toHaveBeenCalledWith(undefined);
  });
});

describe('runShowCommand', () => {
  it('imprime JSON cuando json es true', async () => {
    const logSpy = vi.fn();
    vi.spyOn(console, 'log').mockImplementation(logSpy);
    const ctx = mockCommandContext();
    ctx.library.find.mockReturnValue({
      media: {
        id: 1,
        type: 'movie',
        title: 'Test',
        sortTitle: 'test',
        year: 2020,
        folderPath: '/x/m.mkv',
        createdAt: '2026-01-01',
        updatedAt: '2026-01-01',
      },
      files: [],
    });

    await runShowCommand('test', { json: true });

    expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('"title"'));
    expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('"fileCount"'));
  });

  it('muestra ano vacio cuando media.year es null', async () => {
    const logSpy = vi.fn();
    vi.spyOn(console, 'log').mockImplementation(logSpy);
    const ctx = mockCommandContext();
    ctx.library.find.mockReturnValue({
      media: {
        id: 1,
        type: 'movie',
        title: 'Test',
        sortTitle: 'test',
        year: null,
        folderPath: '/x/m.mkv',
        createdAt: '2026-01-01',
        updatedAt: '2026-01-01',
      },
      files: [],
    });

    await runShowCommand('test', {});

    const output = logSpy.mock.calls.map((args) => String(args[0])).join('\n');
    expect(output).toContain('Test');
    expect(output).not.toContain('(null)');
  });

  it('muestra "(sin ficheros asociados)" cuando files esta vacio', async () => {
    const logSpy = vi.fn();
    vi.spyOn(console, 'log').mockImplementation(logSpy);
    const ctx = mockCommandContext();
    ctx.library.find.mockReturnValue({
      media: {
        id: 1,
        type: 'movie',
        title: 'Test',
        sortTitle: 'test',
        year: 2020,
        folderPath: '/x/m.mkv',
        createdAt: '2026-01-01',
        updatedAt: '2026-01-01',
      },
      files: [],
    });

    await runShowCommand('test', {});

    const output = logSpy.mock.calls.map((args) => String(args[0])).join('\n');
    expect(output).toContain('sin ficheros asociados');
  });

  it('lista ficheros cuando los hay', async () => {
    const logSpy = vi.fn();
    vi.spyOn(console, 'log').mockImplementation(logSpy);
    const ctx = mockCommandContext();
    ctx.library.find.mockReturnValue({
      media: {
        id: 1,
        type: 'episode',
        title: 'Test',
        sortTitle: 'test',
        year: 2020,
        folderPath: '/x/',
        createdAt: '2026-01-01',
        updatedAt: '2026-01-01',
      },
      files: [
        {
          id: 1,
          mediaId: 1,
          season: 1,
          episode: 1,
          title: null,
          filePath: '/x/e1.mkv',
          sizeBytes: 100,
          mtimeMs: 1,
          indexedAt: '2026-01-01',
        },
      ],
    });

    await runShowCommand('test', {});

    const output = logSpy.mock.calls.map((args) => String(args[0])).join('\n');
    expect(output).toContain('1 ficheros');
    expect(output).toContain('/x/e1.mkv');
  });
});

describe('parseListOptions', () => {
  it('devuelve objeto vacio cuando no hay opciones', () => {
    expect(parseListOptions({})).toEqual({});
  });

  it('mapea type y sort', () => {
    expect(parseListOptions({ type: 'series', sort: 'recent' })).toEqual({
      type: 'series',
      sort: 'recent',
    });
  });

  it('mapea limit y offset', () => {
    expect(parseListOptions({ limit: 25, offset: 10 })).toEqual({
      limit: 25,
      offset: 10,
    });
  });

  it('mapea booleanos favorites, inProgress, unwatched, watched', () => {
    expect(
      parseListOptions({
        favorites: true,
        inProgress: true,
        unwatched: false,
        watched: true,
      }),
    ).toEqual({
      favorites: true,
      inProgress: true,
      unwatched: false,
      watched: true,
    });
  });

  it('mapea tags convirtiendo a numeros y filtrando NaN', () => {
    expect(parseListOptions({ tags: ['1', '2', 'abc', '3'] })).toEqual({
      tags: [1, 2, 3],
    });
  });

  it('omite tags cuando es undefined', () => {
    expect(parseListOptions({})).not.toHaveProperty('tags');
  });

  it('mapea export', () => {
    expect(parseListOptions({ export: 'csv' })).toEqual({
      export: 'csv',
    });
  });
});
