import { mkdtemp, rm } from 'node:fs/promises';
import { createServer, type Socket } from 'node:net';
import { tmpdir } from 'node:os';
import path from 'node:path';
import process from 'node:process';

import { PlayerError, formatError } from '../../shared/errors.js';

import type { Logger } from '../../shared/logger.js';
import type { PlaybackConfig } from '../config/schema.js';
import type { PlaybackRepository } from '../db/repositories/playbackRepo.js';

/** Eventos que emite mpv a través del socket IPC. */
type MpvEvent = {
  event: string;
  [key: string]: unknown;
};

/** Opciones para crear el tracker. */
export type MpvTrackerOptions = {
  mediaFileId: number;
  playbackRepo: PlaybackRepository;
  playbackConfig: PlaybackConfig;
  logger: Logger;
  socketPath: string;
};

/**
 * Tracker de mpv vía socket IPC.
 *
 * Se conecta al socket Unix de mpv, observa `time-pos`, `duration`, `pause` y `eof-reached`,
 * y guarda el progreso en la base de datos cada 5 segundos como máximo.
 */
export class MpvTracker {
  private readonly options: MpvTrackerOptions;
  private server: ReturnType<typeof createServer> | null = null;
  private client: Socket | null = null;
  private saveTimer: ReturnType<typeof setInterval> | null = null;
  private pendingPosition: number | null = null;
  private pendingDuration: number | null = null;
  private isPaused = false;
  private hasEnded = false;
  private destroyed = false;
  private readonly socketDir: string;

  constructor(options: MpvTrackerOptions) {
    this.options = options;
    this.socketDir = path.dirname(options.socketPath);
  }

  /** Inicia el servidor de socket y espera la conexión de mpv. */
  async start(): Promise<void> {
    if (this.destroyed) throw new PlayerError('Tracker ya destruido');

    this.server = createServer((socket) => this.handleConnection(socket));

    const server = this.server;
    if (!server) throw new PlayerError('Servidor de socket no inicializado');

    await new Promise<void>((resolve, reject) => {
      server.once('error', reject);
      server.listen(this.options.socketPath, () => {
        server.off('error', reject);
        this.options.logger.debug('mpv-tracker: servidor escuchando', {
          path: this.options.socketPath,
        });
        resolve();
      });
    });
  }

  /** Detiene el tracker y limpia recursos. */
  async stop(): Promise<void> {
    // Flush final si hay posición pendiente, antes de marcar como destruido
    // (flushPosition verifica this.destroyed y retornaría temprano si lo hacemos después)
    if (this.pendingPosition !== null) {
      await this.flushPosition();
    }

    this.destroyed = true;

    if (this.saveTimer) {
      clearInterval(this.saveTimer);
      this.saveTimer = null;
    }

    if (this.client) {
      this.client.destroy();
      this.client = null;
    }

    if (this.server) {
      const server = this.server;
      await new Promise<void>((resolve) => {
        server.close(() => resolve());
      });
      this.server = null;
    }

    // Flush final si hay posición pendiente
    if (this.pendingPosition !== null) {
      await this.flushPosition();
    }

    // Limpieza del socket / pipe
    if (process.platform !== 'win32') {
      try {
        await rm(this.socketDir, { recursive: true, force: true });
      } catch {
        // Ignorado
      }
    }

    this.options.logger.debug('mpv-tracker: detenido');
  }

  private handleConnection(socket: Socket): void {
    if (this.client) {
      this.options.logger.warn('mpv-tracker: conexión duplicada, cerrando anterior');
      this.client.destroy();
    }

    this.client = socket;
    let buffer = '';

    socket.on('data', (chunk) => {
      buffer += chunk.toString();
      const lines = buffer.split('\n');
      buffer = lines.pop() ?? '';
      for (const line of lines) {
        if (line.trim()) this.handleLine(line);
      }
    });

    socket.on('close', () => {
      this.options.logger.debug('mpv-tracker: mpv cerró la conexión');
      this.client = null;
      if (!this.hasEnded) this.handleEnd();
    });

    socket.on('error', (error) => {
      this.options.logger.warn('mpv-tracker: error en socket', { error: formatError(error) });
    });

    // Observar propiedades
    this.sendCommand(['observe_property', 1, 'time-pos']);
    this.sendCommand(['observe_property', 2, 'duration']);
    this.sendCommand(['observe_property', 3, 'pause']);
    this.sendCommand(['observe_property', 4, 'eof-reached']);
  }

  private handleLine(line: string): void {
    let event: MpvEvent;
    try {
      event = JSON.parse(line) as MpvEvent;
    } catch {
      return;
    }

    if (event.event === 'property-change') {
      const name = event.name as string;
      const data = event.data as number | boolean | null | undefined;

      switch (name) {
        case 'time-pos':
          if (typeof data === 'number' && !Number.isNaN(data)) {
            this.pendingPosition = data;
            this.scheduleSave();
          }
          break;
        case 'duration':
          if (typeof data === 'number' && !Number.isNaN(data) && data > 0) {
            this.pendingDuration = data;
          }
          break;
        case 'pause':
          this.isPaused = data === true;
          break;
        case 'eof-reached':
          if (data === true) this.handleEnd();
          break;
      }
    }
  }

  private scheduleSave(): void {
    if (this.saveTimer) return;

    // Guardar como máximo cada 5 segundos
    this.saveTimer = setInterval(() => {
      void this.flushPosition().catch((error) => {
        this.options.logger.warn('mpv-tracker: error guardando posición', { error: String(error) });
      });
    }, 5000);
  }

  private async flushPosition(): Promise<void> {
    // Satisfy require-await: the function is async because callers use .catch()
    await Promise.resolve();
    if (this.pendingPosition === null || this.destroyed) return;

    const position = this.pendingPosition;
    const duration = this.pendingDuration;

    this.pendingPosition = null;
    if (this.saveTimer) {
      clearInterval(this.saveTimer);
      this.saveTimer = null;
    }

    try {
      const now = new Date().toISOString();
      this.options.playbackRepo.save({
        mediaFileId: this.options.mediaFileId,
        positionSeconds: Math.max(0, position),
        durationSeconds: duration ?? null,
        completedAt: null,
        lastPlayedAt: now,
        playCount: 0, // No incrementamos aquí; se hace al lanzar
        updatedAt: now,
      });
      this.options.logger.debug('mpv-tracker: posición guardada', { position, duration });
    } catch (error) {
      this.options.logger.warn('mpv-tracker: error en save', { error: String(error) });
      throw error;
    }
  }

  private handleEnd(): void {
    if (this.hasEnded) return;
    this.hasEnded = true;
    this.options.logger.debug('mpv-tracker: fin de reproducción detectado');
    void this.flushPosition().catch(() => {
      // Ignorado: flush final best-effort
    });
  }

  private sendCommand(args: unknown[]): void {
    if (!this.client?.writable) return;
    this.client.write(`${JSON.stringify({ command: args })}\n`);
  }
}

/**
 * Crea una ruta de IPC para mpv.
 *
 * En POSIX (Linux/macOS) devuelve la ruta de un socket Unix en el filesystem.
 * En Windows devuelve el nombre de un named pipe (`\\.\pipe\mm-mpv-<aleatorio>`),
 * que mpv en Windows usa vía `--input-ipc-server`.
 */
export async function createSocketPath(): Promise<string> {
  if (process.platform === 'win32') {
    const pipeName = `mm-mpv-${crypto.randomUUID().replaceAll('-', '')}`;
    return `\\\\.\\pipe\\${pipeName}`;
  }
  const base = process.env.XDG_RUNTIME_DIR ?? (await mkdtemp(path.join(tmpdir(), 'mm-ipc-')));
  const socketDir = await mkdtemp(path.join(base, 'mm-mpv-'));
  return path.join(socketDir, 'socket');
}

/**
 * Lanza mpv con el socket IPC configurado y arranca el tracker.
 * Devuelve una función de limpieza que detiene el tracker.
 */
export async function launchWithTracker(
  mpvCommand: string,
  mpvArgs: string[],
  filePath: string,
  options: MpvTrackerOptions,
): Promise<() => Promise<void>> {
  const socketPath = await createSocketPath();
  const tracker = new MpvTracker({ ...options, socketPath });
  await tracker.start();

  // Añadir --input-ipc-server antes del fichero
  const ipcArg = `--input-ipc-server=${socketPath}`;
  const finalArgs = [...mpvArgs];
  const fileIndex = finalArgs.indexOf(filePath);
  if (fileIndex >= 0) {
    finalArgs.splice(fileIndex, 0, ipcArg);
  } else {
    finalArgs.push(ipcArg, filePath);
  }

  const { spawn } = await import('node:child_process');
  const child = spawn(mpvCommand, finalArgs, {
    shell: false,
    stdio: 'ignore',
    detached: true,
  });
  child.unref();

  return async () => {
    try {
      await tracker.stop();
    } finally {
      if (!child.killed) child.kill('SIGTERM');
    }
  };
}
