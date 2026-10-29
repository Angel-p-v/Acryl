import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type {
  IndexingRepositories,
  IndexLibraryConfig,
  IndexStats,
} from '../../../../src/core/indexer/service.js';
import type { Logger } from '../../../../src/shared/logger.js';

const mockWatchEvents: { event: string; path: string }[] = [];
type MockWatcher = {
  on: ReturnType<typeof vi.fn>;
  once: ReturnType<typeof vi.fn>;
  off: ReturnType<typeof vi.fn>;
  close: ReturnType<typeof vi.fn>;
  emit: ReturnType<typeof vi.fn>;
  allHandler?: (event: string, path: string) => void;
  errorHandler?: (error: Error) => void;
  readyHandler?: () => void;
  readyErrorHandler?: (error: Error) => void;
};

const mockWatcher: MockWatcher = {
  on: vi.fn(),
  once: vi.fn((event: string, handler: (...args: unknown[]) => void) => {
    if (event === 'ready') handler();
  }),
  off: vi.fn(),
  close: vi.fn().mockResolvedValue(undefined),
  emit: vi.fn(),
};

vi.mock('chokidar', () => ({
  watch: vi.fn(() => mockWatcher),
}));

// Mock indexLibrary at top level
const mockIndexLibrary = vi.fn();

vi.mock('../../../../src/core/indexer/service.js', () => ({
  indexLibrary: mockIndexLibrary,
}));

// Import after mock
const { watchLibrary } = await import('../../../../src/core/indexer/watcher.js');

describe('watchLibrary', () => {
  let mockRepositories: IndexingRepositories;
  let mockLibrary: IndexLibraryConfig;
  let mockLogger: Logger;
  let mockOnIndexed: (stats: IndexStats) => void;
  let mockOnError: (error: unknown) => void;
  let closeWatcher: () => Promise<void>;

  beforeEach(() => {
    mockWatchEvents.length = 0;
    vi.clearAllMocks();

    mockRepositories = {
      mediaRepo: {
        upsert: vi.fn(),
        findById: vi.fn(),
        findByFolderPath: vi.fn(),
        list: vi.fn(),
        search: vi.fn(),
        remove: vi.fn(),
        removeEmpty: vi.fn(),
        count: vi.fn(),
        listFolders: vi.fn(),
        updateFolderPath: vi.fn(),
      },
      fileRepo: {
        upsertMany: vi.fn(),
        findByPath: vi.fn(),
        listByMedia: vi.fn(),
        listAllPaths: vi.fn(),
        removeByMedia: vi.fn(),
        removeMissing: vi.fn(),
        count: vi.fn(),
        updateFilePath: vi.fn(),
      },
      indexRunRepo: {
        start: vi.fn(),
        complete: vi.fn(),
        fail: vi.fn(),
        list: vi.fn(),
        latest: vi.fn(),
      },
    };

    mockLibrary = {
      roots: [{ path: '/media/series', type: 'series' }],
      exclude: ['**/*.sample.*'],
      scan: { followSymlinks: false, maxDepth: 4, minSizeBytes: 0 },
    };

    mockLogger = {
      debug: vi.fn(),
      warn: vi.fn(),
      error: vi.fn(),
      info: vi.fn(),
      child: vi.fn(),
      level: 'debug',
      format: 'text',
    } as unknown as Logger;

    mockOnIndexed = vi.fn();
    mockOnError = vi.fn();

    mockIndexLibrary.mockReset();
    mockIndexLibrary.mockResolvedValue({
      filesSeen: 0,
      filesAdded: 0,
      filesRemoved: 0,
      filesUnchanged: 0,
      mediaAdded: 0,
      mediaRemoved: 0,
      foldersAdded: 0,
      foldersRemoved: 0,
      durationMs: 100,
    });

    // Configurar mockWatcher.on para capturar los manejadores de evento
    mockWatcher.on.mockImplementation((event: string, handler: (...args: unknown[]) => void) => {
      if (event === 'all') {
        mockWatcher.allHandler = handler;
      }
      if (event === 'error') {
        mockWatcher.errorHandler = handler;
      }
    });
    mockWatcher.once.mockImplementation((event: string, handler: (...args: unknown[]) => void) => {
      if (event === 'ready') {
        mockWatcher.readyHandler = handler;
        handler();
      }
      if (event === 'error') {
        mockWatcher.readyErrorHandler = handler;
      }
    });
  });

  afterEach(async () => {
    if (closeWatcher) {
      await closeWatcher();
    }
    vi.restoreAllMocks();
  });

  it('crea un watcher y se resuelve ready', async () => {
    const watcher = watchLibrary({
      repositories: mockRepositories,
      library: mockLibrary,
      logger: mockLogger,
      onIndexed: mockOnIndexed,
      onError: mockOnError,
    });

    await expect(watcher.ready).resolves.toBeUndefined();
    closeWatcher = watcher.close;
  });

  it('cierra el watcher correctamente', async () => {
    const watcher = watchLibrary({
      repositories: mockRepositories,
      library: mockLibrary,
      logger: mockLogger,
      onIndexed: mockOnIndexed,
      onError: mockOnError,
    });

    await watcher.ready;
    await expect(watcher.close()).resolves.toBeUndefined();
  });

  it('maneja evento unlinkDir programando y retornando', async () => {
    const watcher = watchLibrary({
      repositories: mockRepositories,
      library: mockLibrary,
      logger: mockLogger,
      onIndexed: mockOnIndexed,
      onError: mockOnError,
      debounceMs: 10,
    });

    await watcher.ready;
    const allHandler = mockWatcher.allHandler as (event: string, path: string) => void;
    allHandler('unlinkDir', '/media/series/old');

    // unlinkDir debe programar sin indexar
    expect(mockIndexLibrary).not.toHaveBeenCalled();

    await vi.waitFor(() => {
      expect(mockIndexLibrary).toHaveBeenCalledTimes(1);
    });

    closeWatcher = watcher.close;
  });

  it('ignora eventos que no son de vídeo', async () => {
    const watcher = watchLibrary({
      repositories: mockRepositories,
      library: mockLibrary,
      logger: mockLogger,
      onIndexed: mockOnIndexed,
      onError: mockOnError,
      debounceMs: 10,
    });

    await watcher.ready;
    const allHandler = mockWatcher.allHandler as (event: string, path: string) => void;
    allHandler('add', '/media/series/readme.txt');

    // No es un fichero de vídeo, no debe indexar
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(mockIndexLibrary).not.toHaveBeenCalled();

    closeWatcher = watcher.close;
  });

  it('cierra el watcher con timer pendiente (líneas 118-119)', async () => {
    const watcher = watchLibrary({
      repositories: mockRepositories,
      library: mockLibrary,
      logger: mockLogger,
      onIndexed: mockOnIndexed,
      onError: mockOnError,
      debounceMs: 1000, // Timer largo para que esté pendiente al cerrar
    });

    await watcher.ready;
    const allHandler = mockWatcher.allHandler as (event: string, path: string) => void;
    allHandler('add', '/media/series/video.mkv');

    // Cerrar antes de que el timer de debounce se ejecute
    await watcher.close();
  });

  it('propaga errores del watcher a onError', async () => {
    const watcher = watchLibrary({
      repositories: mockRepositories,
      library: mockLibrary,
      logger: mockLogger,
      onIndexed: mockOnIndexed,
      onError: mockOnError,
    });

    await watcher.ready;
    const errorHandler = mockWatcher.errorHandler as (error: Error) => void;
    const testError = new Error('watcher error');
    errorHandler(testError);

    expect(mockOnError).toHaveBeenCalledWith(testError);

    closeWatcher = watcher.close;
  });

  it('no programa indexación cuando el watcher está cerrado', async () => {
    const watcher = watchLibrary({
      repositories: mockRepositories,
      library: mockLibrary,
      logger: mockLogger,
      onIndexed: mockOnIndexed,
      onError: mockOnError,
      debounceMs: 10,
    });

    await watcher.ready;
    await watcher.close();

    const allHandler = mockWatcher.allHandler as (event: string, path: string) => void;
    allHandler('add', '/media/series/video.mkv');

    // closed=true -> onEvent retorna temprano, no programa nada
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(mockIndexLibrary).not.toHaveBeenCalled();
  });

  it('propaga error de indexLibrary a onError', async () => {
    mockIndexLibrary.mockRejectedValueOnce(new Error('index failed'));

    const watcher = watchLibrary({
      repositories: mockRepositories,
      library: mockLibrary,
      logger: mockLogger,
      onIndexed: mockOnIndexed,
      onError: mockOnError,
      debounceMs: 10,
    });

    await watcher.ready;
    const allHandler = mockWatcher.allHandler as (event: string, path: string) => void;
    allHandler('add', '/media/series/video.mkv');

    await vi.waitFor(() => {
      expect(mockOnError).toHaveBeenCalled();
    });

    closeWatcher = watcher.close;
  });

  it('programa una segunda indexación cuando llega un evento durante la primera (líneas 72-73)', async () => {
    const resolvers: (() => void)[] = [];
    mockIndexLibrary.mockImplementation(() => {
      return new Promise<void>((resolve) => {
        resolvers.push(resolve);
      });
    });

    const watcher = watchLibrary({
      repositories: mockRepositories,
      library: mockLibrary,
      logger: mockLogger,
      onIndexed: mockOnIndexed,
      onError: mockOnError,
      debounceMs: 10,
    });

    await watcher.ready;
    const allHandler = mockWatcher.allHandler as (event: string, path: string) => void;

    // Primer evento: se programa indexación tras el debounce
    allHandler('add', '/media/series/video1.mkv');
    await vi.waitFor(() => {
      expect(mockIndexLibrary).toHaveBeenCalledTimes(1);
    });

    // Segundo evento mientras la primera está en curso: running !== null, pending = true
    allHandler('add', '/media/series/video2.mkv');
    expect(mockIndexLibrary).toHaveBeenCalledTimes(1); // aún no se ejecutó la segunda

    // Resolver la primera: en finally, pending=true → schedule() → run() → segunda indexación
    resolvers[0]?.();
    await vi.waitFor(() => {
      expect(mockIndexLibrary).toHaveBeenCalledTimes(2);
    });

    // Resolver la segunda para que close() no se quede colgado esperando running
    resolvers[1]?.();

    closeWatcher = watcher.close;
  });
});
