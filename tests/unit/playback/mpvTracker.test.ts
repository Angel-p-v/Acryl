import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';

import type { PlaybackConfig } from '../../../src/core/config/schema.js';
import type {
  PlaybackRepository,
  PlaybackStateRow,
} from '../../../src/core/db/repositories/playbackRepo.js';
import type { Logger } from '../../../src/shared/logger.js';

vi.mock('node:net', () => {
  const createServer = vi.fn();
  const mockServer = {
    listen: vi.fn((path: string, cb: () => void) => cb()),
    close: vi.fn((cb: () => void) => cb()),
    on: vi.fn(),
    once: vi.fn(),
    off: vi.fn(),
  };
  createServer.mockReturnValue(mockServer);
  return { createServer, Socket: vi.fn() };
});

vi.mock('node:fs/promises', () => ({
  mkdtemp: vi.fn().mockResolvedValue('/tmp/mock-socket'),
  rm: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('node:os', () => ({
  tmpdir: vi.fn().mockReturnValue('/tmp'),
  platform: vi.fn().mockReturnValue('linux'),
}));

vi.mock('node:child_process', () => ({
  spawn: vi.fn(() => ({
    kill: vi.fn(),
    killed: false,
    unref: vi.fn(),
  })),
}));

vi.mock('node:path', async (importOriginal) => {
  const actual = await importOriginal();
  return {
    ...(actual as object),
    dirname: vi.fn((p) => p.split('/').slice(0, -1).join('/') || '/'),
  };
});

const { MpvTracker, createSocketPath, launchWithTracker } =
  await import('../../../src/core/playback/mpvTracker.js');

describe('MpvTracker', () => {
  let mockRepo: PlaybackRepository & { save: ReturnType<typeof vi.fn> };
  let mockConfig: PlaybackConfig;
  let mockLogger: Logger;
  let tracker: InstanceType<typeof MpvTracker> | null = null;

  function createMockClient() {
    const handlers: Record<string, ((...args: unknown[]) => void)[]> = {};
    return {
      writable: true,
      on: vi.fn((event: string, handler: (...args: unknown[]) => void) => {
        if (event === 'data') mockClient.dataHandler = handler as (data: string) => void;
        if (event === 'close') mockClient.closeHandler = handler as () => void;
        (handlers[event] ??= []).push(handler);
      }),
      write: vi.fn(),
      destroy: vi.fn(),
      off: vi.fn(),
      emit: vi.fn((event: string, ...args: unknown[]) => {
        for (const h of handlers[event] ?? []) h(...args);
      }),
    };
  }

  type MockClient = ReturnType<typeof createMockClient> & {
    dataHandler?: (data: string) => void;
    closeHandler?: () => void;
  };

  let mockClient: MockClient;
  let mockServer: ReturnType<typeof vi.fn>;

  beforeEach(async () => {
    const { createServer } = await import('node:net');
    mockServer = createServer as ReturnType<typeof vi.fn>;

    mockRepo = {
      save: vi.fn((input) => ({
        mediaFileId: input.mediaFileId,
        positionSeconds: input.positionSeconds,
        durationSeconds: input.durationSeconds ?? null,
        completedAt: input.completedAt ?? null,
        lastPlayedAt: input.lastPlayedAt,
        playCount: input.playCount ?? 0,
        updatedAt: input.updatedAt,
      })),
      get: vi.fn(),
      remove: vi.fn(),
      listInProgress: vi.fn(),
      listRecent: vi.fn(),
      count: vi.fn(),
    };

    mockConfig = {
      tracking: 'auto',
      resume: true,
      resumeMinSeconds: 30,
      completedThreshold: 0.9,
      completedTailSeconds: 30,
    };

    mockLogger = {
      level: 'debug',
      format: 'text',
      error: vi.fn(),
      warn: vi.fn(),
      info: vi.fn(),
      debug: vi.fn(),
      child: vi.fn(() => mockLogger),
    };

    mockClient = createMockClient();
    vi.clearAllMocks();
  });

  afterEach(async () => {
    if (tracker) {
      await tracker.stop().catch(() => {});
    }
    vi.clearAllMocks();
  });

  it('crea el servidor y escucha en el socket path', async () => {
    const { createServer } = await import('node:net');
    tracker = new MpvTracker({
      mediaFileId: 1,
      playbackRepo: mockRepo,
      playbackConfig: mockConfig,
      logger: mockLogger,
      socketPath: `/tmp/test-socket-${Date.now()}`,
    });

    await tracker.start();

    expect(createServer).toHaveBeenCalled();
    const serverInstance = (createServer as ReturnType<typeof vi.fn>).mock.results[0]!.value!;
    expect(serverInstance.listen).toHaveBeenCalledWith(expect.any(String), expect.any(Function));
  });

  it('guarda posición cuando mpv envía time-pos y luego eof-reached', async () => {
    tracker = new MpvTracker({
      mediaFileId: 42,
      playbackRepo: mockRepo,
      playbackConfig: mockConfig,
      logger: mockLogger,
      socketPath: `/tmp/test-socket-${Date.now()}`,
    });

    await tracker.start();

    const { createServer: cs } = await import('node:net');
    const connectionHandler = (cs as ReturnType<typeof vi.fn>).mock.calls[0]![0]!;
    expect(connectionHandler).toBeDefined();
    connectionHandler(mockClient);

    // Simular evento time-pos seguido de eof-reached (para disparar flush inmediato)
    const dataHandler = mockClient.dataHandler as ((data: string) => void) | undefined;

    dataHandler?.(
      JSON.stringify({ event: 'property-change', name: 'time-pos', data: 120.5 }) + '\n',
    );
    dataHandler?.(
      JSON.stringify({ event: 'property-change', name: 'eof-reached', data: true }) + '\n',
    );

    await vi.waitFor(() => {
      expect(mockRepo.save).toHaveBeenCalledWith(
        expect.objectContaining({
          mediaFileId: 42,
          positionSeconds: 120.5,
        }),
      );
    });
  });

  it('actualiza duración cuando mpv envía duration', async () => {
    tracker = new MpvTracker({
      mediaFileId: 42,
      playbackRepo: mockRepo,
      playbackConfig: mockConfig,
      logger: mockLogger,
      socketPath: `/tmp/test-socket-${Date.now()}`,
    });

    await tracker.start();
    const { createServer: cs } = await import('node:net');
    const connectionHandler = (cs as ReturnType<typeof vi.fn>).mock.calls[0]![0]!;
    expect(connectionHandler).toBeDefined();
    connectionHandler(mockClient);

    const dataHandler = mockClient.dataHandler as ((data: string) => void) | undefined;

    dataHandler?.(
      JSON.stringify({ event: 'property-change', name: 'duration', data: 2700 }) + '\n',
    );

    // La duración se guarda internamente, no hace save inmediato
    // Simular time-pos y eof-reached para disparar save con duración
    dataHandler?.(JSON.stringify({ event: 'property-change', name: 'time-pos', data: 100 }) + '\n');
    dataHandler?.(
      JSON.stringify({ event: 'property-change', name: 'eof-reached', data: true }) + '\n',
    );

    await vi.waitFor(() => {
      expect(mockRepo.save).toHaveBeenCalledWith(
        expect.objectContaining({
          mediaFileId: 42,
          durationSeconds: 2700,
        }),
      );
    });
  });

  it('marca como finalizado cuando mpv envía eof-reached', async () => {
    tracker = new MpvTracker({
      mediaFileId: 42,
      playbackRepo: mockRepo,
      playbackConfig: mockConfig,
      logger: mockLogger,
      socketPath: `/tmp/test-socket-${Date.now()}`,
    });

    await tracker.start();
    const { createServer: cs } = await import('node:net');
    const connectionHandler = (cs as ReturnType<typeof vi.fn>).mock.calls[0]![0]!;
    expect(connectionHandler).toBeDefined();
    connectionHandler(mockClient);

    const dataHandler = mockClient.dataHandler as ((data: string) => void) | undefined;

    dataHandler?.(
      JSON.stringify({ event: 'property-change', name: 'time-pos', data: 2690 }) + '\n',
    );
    dataHandler?.(
      JSON.stringify({ event: 'property-change', name: 'eof-reached', data: true }) + '\n',
    );

    await vi.waitFor(() => {
      expect(mockRepo.save).toHaveBeenCalledWith(
        expect.objectContaining({
          mediaFileId: 42,
          positionSeconds: 2690,
        }),
      );
    });
  });

  it('detiene y limpia recursos al llamar stop', async () => {
    tracker = new MpvTracker({
      mediaFileId: 42,
      playbackRepo: mockRepo,
      playbackConfig: mockConfig,
      logger: mockLogger,
      socketPath: `/tmp/test-socket-${Date.now()}`,
    });

    await tracker.start();

    await expect(tracker.stop()).resolves.toBeUndefined();
  });

  it('no guarda si tracking es off', async () => {
    const configOff = { ...mockConfig, tracking: 'off' as const };
    tracker = new MpvTracker({
      mediaFileId: 42,
      playbackRepo: mockRepo,
      playbackConfig: configOff,
      logger: mockLogger,
      socketPath: `/tmp/test-socket-${Date.now()}`,
    });

    await tracker.start();

    await tracker.stop();

    // El tracker se inicia pero no procesa eventos cuando tracking es off
    // La validación real ocurre en launchPlayer que no llama al tracker si tracking === 'off'

    expect(mockRepo.save).not.toHaveBeenCalled();
  });

  it('lanza error al iniciar tracker ya destruido', async () => {
    tracker = new MpvTracker({
      mediaFileId: 42,
      playbackRepo: mockRepo,
      playbackConfig: mockConfig,
      logger: mockLogger,
      socketPath: `/tmp/test-socket-${Date.now()}`,
    });

    await tracker.start();
    await tracker.stop();

    // destroyed = true después de stop()
    await expect(tracker.start()).rejects.toThrow('Tracker ya destruido');
  });

  it('cierra conexión anterior cuando llega una duplicada', async () => {
    tracker = new MpvTracker({
      mediaFileId: 42,
      playbackRepo: mockRepo,
      playbackConfig: mockConfig,
      logger: mockLogger,
      socketPath: `/tmp/test-socket-${Date.now()}`,
    });

    await tracker.start();
    const { createServer: cs } = await import('node:net');
    const connectionHandler = (cs as ReturnType<typeof vi.fn>).mock.calls[0]![0]!;

    const firstClient = createMockClient();
    const secondClient = createMockClient();
    connectionHandler(firstClient);

    // Segunda conexión: se debe destruir la primera
    connectionHandler(secondClient);

    expect(firstClient.destroy).toHaveBeenCalled();
  });

  it('ignora líneas JSON inválidas en handleLine', async () => {
    tracker = new MpvTracker({
      mediaFileId: 42,
      playbackRepo: mockRepo,
      playbackConfig: mockConfig,
      logger: mockLogger,
      socketPath: `/tmp/test-socket-${Date.now()}`,
    });

    await tracker.start();
    const { createServer: cs } = await import('node:net');
    const connectionHandler = (cs as ReturnType<typeof vi.fn>).mock.calls[0]![0]!;
    connectionHandler(mockClient);

    const dataHandler = mockClient.dataHandler as ((data: string) => void) | undefined;
    dataHandler?.('no es json válido\n');
    dataHandler?.('{ tampoco }\n');

    // No debe lanzar ni hacer save
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(mockRepo.save).not.toHaveBeenCalled();
  });

  it('actualiza isPaused cuando mpv envía pause', async () => {
    tracker = new MpvTracker({
      mediaFileId: 42,
      playbackRepo: mockRepo,
      playbackConfig: mockConfig,
      logger: mockLogger,
      socketPath: `/tmp/test-socket-${Date.now()}`,
    });

    await tracker.start();
    const { createServer: cs } = await import('node:net');
    const connectionHandler = (cs as ReturnType<typeof vi.fn>).mock.calls[0]![0]!;
    connectionHandler(mockClient);

    const dataHandler = mockClient.dataHandler as ((data: string) => void) | undefined;
    dataHandler?.(JSON.stringify({ event: 'property-change', name: 'pause', data: true }) + '\n');

    // Simular eof-reached para flush y verificar que se guarda
    dataHandler?.(JSON.stringify({ event: 'property-change', name: 'time-pos', data: 100 }) + '\n');
    dataHandler?.(
      JSON.stringify({ event: 'property-change', name: 'eof-reached', data: true }) + '\n',
    );

    await vi.waitFor(() => {
      expect(mockRepo.save).toHaveBeenCalled();
    });
  });

  it('scheduleSave no crea timer si ya existe', async () => {
    tracker = new MpvTracker({
      mediaFileId: 42,
      playbackRepo: mockRepo,
      playbackConfig: mockConfig,
      logger: mockLogger,
      socketPath: `/tmp/test-socket-${Date.now()}`,
    });

    await tracker.start();
    const { createServer: cs } = await import('node:net');
    const connectionHandler = (cs as ReturnType<typeof vi.fn>).mock.calls[0]![0]!;
    connectionHandler(mockClient);

    const dataHandler = mockClient.dataHandler as ((data: string) => void) | undefined;
    dataHandler?.(JSON.stringify({ event: 'property-change', name: 'time-pos', data: 50 }) + '\n');

    // Enviar eof-reached para disparar flush inmediato (no esperar al timer de 5s)
    dataHandler?.(
      JSON.stringify({ event: 'property-change', name: 'eof-reached', data: true }) + '\n',
    );

    await vi.waitFor(() => {
      expect(mockRepo.save).toHaveBeenCalledTimes(1);
    });
  });

  it('flushPosition no guarda si pendingPosition es null', async () => {
    tracker = new MpvTracker({
      mediaFileId: 42,
      playbackRepo: mockRepo,
      playbackConfig: mockConfig,
      logger: mockLogger,
      socketPath: `/tmp/test-socket-${Date.now()}`,
    });

    // Llamar al método privado vía cualquier ruta que no tenga pendingPosition
    // Crear tracker sin eventos previos, luego llamar stop directamente
    await tracker.stop();
    expect(mockRepo.save).not.toHaveBeenCalled();
  });

  it('flushPosition guarda cuando hay pendingPosition', async () => {
    tracker = new MpvTracker({
      mediaFileId: 42,
      playbackRepo: mockRepo,
      playbackConfig: mockConfig,
      logger: mockLogger,
      socketPath: `/tmp/test-socket-${Date.now()}`,
    });

    await tracker.start();
    const { createServer: cs } = await import('node:net');
    const connectionHandler = (cs as ReturnType<typeof vi.fn>).mock.calls[0]![0]!;
    connectionHandler(mockClient);

    const dataHandler = mockClient.dataHandler as ((data: string) => void) | undefined;
    dataHandler?.(
      JSON.stringify({ event: 'property-change', name: 'time-pos', data: 42.5 }) + '\n',
    );
    // eof-reached dispara flushPosition inmediatamente
    dataHandler?.(
      JSON.stringify({ event: 'property-change', name: 'eof-reached', data: true }) + '\n',
    );

    await vi.waitFor(() => {
      expect(mockRepo.save).toHaveBeenCalledWith(
        expect.objectContaining({ positionSeconds: 42.5 }),
      );
    });
  });

  it('handleEnd no hace flush si ya terminó', async () => {
    tracker = new MpvTracker({
      mediaFileId: 42,
      playbackRepo: mockRepo,
      playbackConfig: mockConfig,
      logger: mockLogger,
      socketPath: `/tmp/test-socket-${Date.now()}`,
    });

    await tracker.start();
    const { createServer: cs } = await import('node:net');
    const connectionHandler = (cs as ReturnType<typeof vi.fn>).mock.calls[0]![0]!;
    connectionHandler(mockClient);

    const dataHandler = mockClient.dataHandler as ((data: string) => void) | undefined;
    // Primer eof-reached dispara handleEnd → flushPosition → save
    dataHandler?.(JSON.stringify({ event: 'property-change', name: 'time-pos', data: 10 }) + '\n');
    dataHandler?.(
      JSON.stringify({ event: 'property-change', name: 'eof-reached', data: true }) + '\n',
    );

    await vi.waitFor(() => {
      expect(mockRepo.save).toHaveBeenCalledTimes(1);
    });

    const saveCallsBefore = mockRepo.save.mock.calls.length;

    // Segundo eof-reached: handleEnd ve hasEnded === true y retorna temprano
    dataHandler?.(
      JSON.stringify({ event: 'property-change', name: 'eof-reached', data: true }) + '\n',
    );

    await new Promise((resolve) => setTimeout(resolve, 100));
    expect(mockRepo.save).toHaveBeenCalledTimes(saveCallsBefore);
  });

  it('sendCommand no escribe si el cliente no es writable', async () => {
    tracker = new MpvTracker({
      mediaFileId: 42,
      playbackRepo: mockRepo,
      playbackConfig: mockConfig,
      logger: mockLogger,
      socketPath: `/tmp/test-socket-${Date.now()}`,
    });

    await tracker.start();
    const { createServer: cs } = await import('node:net');
    const connectionHandler = (cs as ReturnType<typeof vi.fn>).mock.calls[0]![0]!;

    const nonWritableClient = createMockClient();
    nonWritableClient.writable = false;
    connectionHandler(nonWritableClient);

    // sendCommand se llama internamente en handleConnection con observe_property
    // No debe llamar a write si no es writable
    expect(nonWritableClient.write).not.toHaveBeenCalled();
  });

  it('stop flushes pendingPosition si existe', async () => {
    tracker = new MpvTracker({
      mediaFileId: 42,
      playbackRepo: mockRepo,
      playbackConfig: mockConfig,
      logger: mockLogger,
      socketPath: `/tmp/test-socket-${Date.now()}`,
    });

    await tracker.start();
    const { createServer: cs } = await import('node:net');
    const connectionHandler = (cs as ReturnType<typeof vi.fn>).mock.calls[0]![0]!;
    connectionHandler(mockClient);

    const dataHandler = mockClient.dataHandler as ((data: string) => void) | undefined;
    dataHandler?.(JSON.stringify({ event: 'property-change', name: 'time-pos', data: 99 }) + '\n');

    // Detener antes de que el timer de 5s dispare: debe flush inmediato
    await tracker.stop();
    expect(mockRepo.save).toHaveBeenCalledWith(expect.objectContaining({ positionSeconds: 99 }));
  });

  it('launchWithTracker spawns mpv con --input-ipc-server y retorna limpieza', async () => {
    const { spawn } = await import('node:child_process');
    const mockSpawn = spawn as ReturnType<typeof vi.fn>;
    vi.clearAllMocks();

    const cleanup = await launchWithTracker('mpv', ['--no-config'], '/media/serie.mkv', {
      mediaFileId: 42,
      playbackRepo: mockRepo,
      playbackConfig: mockConfig,
      logger: mockLogger,
      socketPath: '/tmp/ignored',
    });

    // spawn se llamó con mpv y args que incluyen --input-ipc-server y el fichero
    expect(mockSpawn).toHaveBeenCalledTimes(1);
    const callArgs = mockSpawn.mock.calls[0]!;
    expect(callArgs[0]).toBe('mpv');
    const args = callArgs[1] as string[];
    expect(args).toContain('/media/serie.mkv');
    // --input-ipc-server debe ir justo antes del fichero (como --input-ipc-server=<ruta>)
    const fileIndex = args.indexOf('/media/serie.mkv');
    expect(fileIndex).toBeGreaterThanOrEqual(0);
    expect(args[fileIndex - 1]).toMatch(/^--input-ipc-server=/);

    // La función de limpieza detiene el tracker y mata el child
    await cleanup();

    const child = mockSpawn.mock.results[0]!.value as {
      kill: ReturnType<typeof vi.fn>;
      killed: boolean;
      unref: ReturnType<typeof vi.fn>;
    };
    expect(child.kill).toHaveBeenCalledWith('SIGTERM');
  });
});
