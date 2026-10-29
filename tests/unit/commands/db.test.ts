/* eslint-disable @typescript-eslint/unbound-method,@typescript-eslint/consistent-type-imports */

import { copyFileSync, chmodSync, existsSync } from 'node:fs';

import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';

import { withCommandContext, type CommandContext } from '../../../src/commands/context.js';
import {
  runDbStatsCommand,
  runDbOptimizeCommand,
  runDbBackupCommand,
  runDbRestoreCommand,
} from '../../../src/commands/db.js';
import { loadConfig } from '../../../src/core/config/loader.js';
import { backupDatabase } from '../../../src/core/db/backup.js';
import { openDatabase, closeDatabase } from '../../../src/core/db/connection.js';
import { databaseSizeBytes } from '../../../src/core/db/size.js';
import { DbError } from '../../../src/shared/errors.js';

import type { AppConfig } from '../../../src/core/config/schema.js';
import type { DatabaseMaintenance } from '../../../src/core/db/maintenance.js';
import type { PlaybackRepository } from '../../../src/core/db/repositories/playbackRepo.js';
import type { PreferenceRepository } from '../../../src/core/db/repositories/preferencesRepo.js';
import type { LibraryService } from '../../../src/core/library/service.js';
import type { Logger } from '../../../src/shared/logger.js';
import type { Database } from 'better-sqlite3';

vi.mock('../../../src/commands/context.js', () => ({
  withCommandContext: vi.fn(),
}));

vi.mock('../../../src/core/db/connection.js', () => ({
  openDatabase: vi.fn(),
  closeDatabase: vi.fn(),
}));

vi.mock('../../../src/core/db/backup.js', () => ({
  backupDatabase: vi.fn(),
}));

vi.mock('../../../src/core/db/size.js', () => ({
  databaseSizeBytes: vi.fn(),
}));

vi.mock('node:fs', async (importOriginal) => {
  const actual = await importOriginal<typeof import('node:fs')>();
  return {
    ...actual,
    existsSync: vi.fn(),
    copyFileSync: vi.fn(),
    chmodSync: vi.fn(),
  };
});

vi.mock('../../../src/core/config/loader.js', () => ({
  loadConfig: vi.fn(),
}));

type MockCommandContext = {
  config: { database: { path: string } };
  library: LibraryService;
  maintenance: DatabaseMaintenance;
  playbackRepo: PlaybackRepository;
  preferenceRepo: PreferenceRepository;
  playbackConfig: unknown;
  logger: Logger;
};

function contextWith(overrides: Partial<MockCommandContext> = {}): MockCommandContext {
  return {
    config: { database: { path: '/test/library.db' } },
    library: {
      list: vi.fn(),
      count: vi.fn(),
      find: vi.fn(),
      stats: vi.fn(),
    } as unknown as LibraryService,
    maintenance: { optimize: vi.fn(), vacuum: vi.fn() },
    playbackRepo: {} as unknown as PlaybackRepository,
    preferenceRepo: {} as unknown as PreferenceRepository,
    playbackConfig: {},
    logger: { debug: vi.fn(), warn: vi.fn() } as unknown as Logger,
    ...overrides,
  };
}

let capturedContext: MockCommandContext | null = null;

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(databaseSizeBytes).mockReturnValue(1024);
  vi.mocked(loadConfig).mockResolvedValue({
    database: { path: '/test/library.db' },
  } as unknown as AppConfig);
  capturedContext = null;
});

afterEach(() => {
  vi.restoreAllMocks();
});

function mockCommandContext(overrides: Partial<MockCommandContext> = {}): MockCommandContext {
  capturedContext = contextWith(overrides);
  vi.mocked(withCommandContext).mockImplementation(async (_opts, run) => {
    return await run(capturedContext as unknown as CommandContext);
  });
  return capturedContext;
}

describe('runDbStatsCommand', () => {
  it('imprime JSON cuando options.json es true', async () => {
    const logSpy = vi.fn();
    vi.spyOn(console, 'log').mockImplementation(logSpy);
    const ctx = mockCommandContext();
    const stats = {
      media: { total: 3, series: 1, anime: 1, movie: 1, unknown: 0 },
      files: 10,
      inProgress: 0,
      lastRun: null,
    };
    vi.mocked(ctx.library.stats).mockReturnValue(stats);

    await runDbStatsCommand({ json: true });

    expect(withCommandContext).toHaveBeenCalledTimes(1);
    expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('"database"'));
    expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('/test/library.db'));
  });

  it('imprime texto plano cuando no se ha indexado nada (lastRun null)', async () => {
    const logSpy = vi.fn();
    vi.spyOn(console, 'log').mockImplementation(logSpy);
    const ctx = mockCommandContext();
    const stats = {
      media: { total: 0, series: 0, anime: 0, movie: 0, unknown: 0 },
      files: 0,
      inProgress: 0,
      lastRun: null,
    };
    vi.mocked(ctx.library.stats).mockReturnValue(stats);

    await runDbStatsCommand({});

    expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('todavía no se ha indexado nada'));
  });

  it('imprime datos de la última indexación cuando lastRun no es null', async () => {
    const logSpy = vi.fn();
    vi.spyOn(console, 'log').mockImplementation(logSpy);
    const ctx = mockCommandContext();
    const stats = {
      media: { total: 5, series: 2, anime: 1, movie: 2, unknown: 0 },
      files: 20,
      inProgress: 0,
      lastRun: {
        id: 42,
        status: 'completed' as const,
        startedAt: '2026-01-01T00:00:00Z',
        finishedAt: '2026-01-01T00:01:00Z',
        filesSeen: 30,
        filesAdded: 20,
        filesRemoved: 5,
      },
    };
    vi.mocked(ctx.library.stats).mockReturnValue(stats);

    await runDbStatsCommand({});

    expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('#42'));
    expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('completed'));
  });
});

describe('runDbOptimizeCommand', () => {
  it('ejecuta solo PRAGMA optimize cuando vacuum no es true', async () => {
    const logSpy = vi.fn();
    vi.spyOn(console, 'log').mockImplementation(logSpy);
    mockCommandContext();

    await runDbOptimizeCommand({});

    expect(databaseSizeBytes).toHaveBeenCalledTimes(2);
    expect(capturedContext!.maintenance.optimize).toHaveBeenCalledTimes(1);
    expect(capturedContext!.maintenance.vacuum).not.toHaveBeenCalled();
    expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('PRAGMA optimize'));
  });

  it('ejecuta PRAGMA optimize + VACUUM cuando vacuum es true', async () => {
    const logSpy = vi.fn();
    vi.spyOn(console, 'log').mockImplementation(logSpy);
    mockCommandContext();

    await runDbOptimizeCommand({ vacuum: true });

    expect(capturedContext!.maintenance.optimize).toHaveBeenCalledTimes(1);
    expect(capturedContext!.maintenance.vacuum).toHaveBeenCalledTimes(1);
    expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('PRAGMA optimize + VACUUM'));
  });

  it('imprime JSON cuando options.json es true', async () => {
    const logSpy = vi.fn();
    vi.spyOn(console, 'log').mockImplementation(logSpy);
    mockCommandContext();

    await runDbOptimizeCommand({ json: true });

    expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('"optimized"'));
    expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('"vacuum"'));
  });
});

describe('runDbBackupCommand', () => {
  it('lanza DbError cuando la base de datos es en memoria', async () => {
    vi.mocked(loadConfig).mockResolvedValue({
      database: { path: ':memory:' },
    } as unknown as AppConfig);

    await expect(runDbBackupCommand({})).rejects.toThrow(DbError);
    expect(openDatabase).not.toHaveBeenCalled();
  });

  it('crea backup e imprime JSON cuando options.json es true', async () => {
    const logSpy = vi.fn();
    vi.spyOn(console, 'log').mockImplementation(logSpy);
    const db = {} as unknown as Database;
    vi.mocked(openDatabase).mockReturnValue(db);
    vi.mocked(backupDatabase).mockReturnValue({
      path: '/test/backup.db',
      createdAt: '2026-01-01T00:00:00Z',
    });

    await runDbBackupCommand({ json: true });

    expect(openDatabase).toHaveBeenCalledWith('/test/library.db');
    expect(backupDatabase).toHaveBeenCalledWith(db, '/test/library.db', 'manual');
    expect(closeDatabase).toHaveBeenCalledWith(db);
    expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('"backup"'));
  });

  it('crea backup e imprime texto cuando options.json es false', async () => {
    const logSpy = vi.fn();
    vi.spyOn(console, 'log').mockImplementation(logSpy);
    const db = {} as unknown as Database;
    vi.mocked(openDatabase).mockReturnValue(db);
    vi.mocked(backupDatabase).mockReturnValue({
      path: '/test/backup.db',
      createdAt: '2026-01-01T00:00:00Z',
    });

    await runDbBackupCommand({});

    expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('Backup creado:'));
    expect(closeDatabase).toHaveBeenCalledWith(db);
  });

  it('usa la label proporcionada en options.label', async () => {
    const db = {} as unknown as Database;
    vi.mocked(openDatabase).mockReturnValue(db);
    vi.mocked(backupDatabase).mockReturnValue({
      path: '/test/backup.db',
      createdAt: '2026-01-01T00:00:00Z',
    });

    await runDbBackupCommand({ label: 'antes-de-migrar' });

    expect(backupDatabase).toHaveBeenCalledWith(db, '/test/library.db', 'antes-de-migrar');
  });
});

describe('runDbRestoreCommand', () => {
  it('lanza DbError cuando el archivo de backup no existe', async () => {
    vi.mocked(existsSync).mockReturnValue(false);

    await expect(runDbRestoreCommand({ from: '/no-existe.db' })).rejects.toThrow(DbError);
    expect(copyFileSync).not.toHaveBeenCalled();
  });

  it('lanza DbError cuando la base de datos es en memoria', async () => {
    vi.mocked(loadConfig).mockResolvedValue({
      database: { path: ':memory:' },
    } as unknown as AppConfig);
    vi.mocked(existsSync).mockReturnValue(true);

    await expect(runDbRestoreCommand({ from: '/backup.db' })).rejects.toThrow(DbError);
    expect(copyFileSync).not.toHaveBeenCalled();
  });

  it('restaura e imprime JSON cuando options.json es true', async () => {
    const logSpy = vi.fn();
    vi.spyOn(console, 'log').mockImplementation(logSpy);
    const db = {} as unknown as Database;
    vi.mocked(existsSync).mockReturnValue(true);
    vi.mocked(loadConfig).mockResolvedValue({
      database: { path: '/test/library.db' },
    } as unknown as AppConfig);
    vi.mocked(openDatabase).mockReturnValue(db);

    await runDbRestoreCommand({ from: '/backup.db', json: true });

    expect(copyFileSync).toHaveBeenCalledWith('/backup.db', '/test/library.db');
    expect(chmodSync).toHaveBeenCalledWith('/test/library.db', 0o600);
    expect(openDatabase).toHaveBeenCalledWith('/test/library.db');
    expect(closeDatabase).toHaveBeenCalledWith(db);
    expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('"restored"'));
  });

  it('restaura e imprime texto cuando options.json es false', async () => {
    const logSpy = vi.fn();
    vi.spyOn(console, 'log').mockImplementation(logSpy);
    const db = {} as unknown as Database;
    vi.mocked(existsSync).mockReturnValue(true);
    vi.mocked(loadConfig).mockResolvedValue({
      database: { path: '/test/library.db' },
    } as unknown as AppConfig);
    vi.mocked(openDatabase).mockReturnValue(db);

    await runDbRestoreCommand({ from: '/backup.db' });

    expect(logSpy).toHaveBeenCalledWith(expect.stringContaining('Base de datos restaurada desde:'));
    expect(closeDatabase).toHaveBeenCalledWith(db);
  });
});
