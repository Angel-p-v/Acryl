import { describe, expect, it } from 'vitest';

import { createSocketPath } from '../../../../src/core/playback/mpvTracker.js';

describe('createSocketPath', () => {
  it('devuelve ruta de socket Unix en POSIX', async () => {
    const socketPath: string = await createSocketPath();
    expect(socketPath.includes('/mm-mpv-')).toBe(true);
    expect(socketPath.endsWith('/socket')).toBe(true);
    expect(socketPath.includes('\\\\.\\pipe\\')).toBe(false);
  });

  it('en Windows devuelve named pipe (solo se ejecuta en win32)', async () => {
    if (process.platform !== 'win32') return;
    const pipePath: string = await createSocketPath();
    expect(pipePath.startsWith('\\\\.\\pipe\\mm-mpv-')).toBe(true);
    const pipeName = pipePath.split('\\').pop()!;
    expect(pipeName.startsWith('mm-mpv-')).toBe(true);
    expect(pipeName.length).toBe('mm-mpv-'.length + 32); // UUID sin guiones = 32 hex chars
    expect(pipeName).not.toContain('-');
  });
});
