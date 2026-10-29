import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi, type MockInstance } from 'vitest';

import { runWatchCommand } from '../../../src/commands/watch.js';
import { IndexError } from '../../../src/shared/errors.js';

import type { LibraryWatcher, WatchLibraryOptions } from '../../../src/core/indexer/watcher.js';

describe('comando mm watch (integración)', () => {
  let workDir: string;
  let configPath: string;
  let output: string[];
  let spy: MockInstance<typeof console.log>;

  beforeEach(async () => {
    workDir = await mkdtemp(path.join(tmpdir(), 'mm-watch-cmd-'));
    const root = path.join(workDir, 'series');
    await mkdir(root, { recursive: true });
    configPath = path.join(workDir, 'config.yaml');
    await writeFile(
      configPath,
      [
        'library:',
        '  roots:',
        `    - path: '${root}'`,
        '      type: series',
        'database:',
        `  path: '${path.join(workDir, 'library.db')}'`,
        '',
      ].join('\n'),
      'utf8',
    );

    output = [];
    spy = vi.spyOn(console, 'log').mockImplementation((...args: unknown[]) => {
      output.push(args.map((arg) => String(arg)).join(' '));
    });
  });

  afterEach(async () => {
    spy.mockRestore();
    await rm(workDir, { recursive: true, force: true });
  });

  it('indexa, instala el watcher y lo cierra al recibir una señal', async () => {
    const close = vi.fn(() => Promise.resolve());
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    let received: WatchLibraryOptions | undefined;

    const watchLibrary = (options: WatchLibraryOptions): LibraryWatcher => {
      received = options;
      return { ready: Promise.resolve(), close };
    };

    await runWatchCommand(
      { config: configPath },
      { watchLibrary, waitForSignal: () => Promise.resolve('SIGINT') },
    );

    received?.onIndexed?.({
      dryRun: false,
      runId: 1,
      rootsScanned: 1,
      filesSeen: 1,
      filesAdded: 1,
      filesUnchanged: 0,
      filesRemoved: 0,
      mediaAdded: 1,
      mediaRemoved: 0,
      skipped: 0,
      warnings: [],
      durationMs: 5,
    });
    received?.onError?.(new Error('falló el watcher'));

    expect(received?.library.roots).toHaveLength(1);
    expect(close).toHaveBeenCalledOnce();
    expect(errorSpy).toHaveBeenCalled();
    expect(output.join('\n')).toContain('Índice inicial actualizado');
    expect(output.join('\n')).toContain('Cambios detectados');
    expect(output.join('\n')).toContain('Vigilando 1 raíz');
    expect(output.join('\n')).toContain('Vigilancia detenida');
    errorSpy.mockRestore();
  });

  it('explica que faltan raíces antes de crear el watcher', async () => {
    await writeFile(
      configPath,
      ['database:', `  path: '${path.join(workDir, 'empty.db')}'`, ''].join('\n'),
      'utf8',
    );
    const watchLibrary = vi.fn();

    await expect(
      runWatchCommand(
        { config: configPath },
        {
          watchLibrary,
          waitForSignal: () => Promise.resolve('SIGINT'),
        },
      ),
    ).rejects.toThrow(IndexError);
    expect(watchLibrary).not.toHaveBeenCalled();
  });
});
