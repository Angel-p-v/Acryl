import { spawn } from 'node:child_process';
import { constants } from 'node:fs';
import { access } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';

import { PlayerError, formatError } from '../../shared/errors.js';
import { launchWithTracker } from '../playback/mpvTracker.js';

import type { Logger } from '../../shared/logger.js';
import type { PlayerConfig, PlaybackConfig } from '../config/schema.js';
import type { PlaybackRepository } from '../db/repositories/playbackRepo.js';
import type { ChildProcess } from 'node:child_process';

/** Forma de lanzar el reproductor: en segundo plano o esperando a que cierre. */
export type LaunchMode = 'detach' | 'foreground';

/** Comando resuelto y listo para ejecutarse, sin pasar por una shell. */
export type LaunchPlan = {
  player: string;
  command: string;
  args: string[];
  mode: LaunchMode;
  filePath: string;
  /** Ruta opcional al archivo de subtítulo (p. ej. .srt) a cargar. */
  subtitlePath?: string | null;
  env?: Record<string, string>;
};

export type CreateLaunchPlanOptions = {
  playerConfig: PlayerConfig;
  filePath: string;
  /** Reproductor alternativo al configurado por defecto. */
  playerName?: string;
  /** Fuerza el modo; si se omite se usa `player.onLaunch`. */
  mode?: LaunchMode;
  /** Segundos desde los que reanudar; si se omite se elimina el argumento con {start}. */
  startSeconds?: number | null;
  /** Ruta opcional al archivo de subtítulo (p. ej. .srt) a cargar. */
  subtitlePath?: string | null;
  /** Índice 0-based de la pista de audio seleccionada (según ffprobe). */
  audioTrackIndex?: number | null;
  /** Índice 0-based de la pista de subtítulos seleccionada (según ffprobe). */
  subtitleTrackIndex?: number | null;
};

/** Opciones para el tracker IPC de mpv. */
export type TrackerDeps = {
  playbackRepo: PlaybackRepository;
  playbackConfig: PlaybackConfig;
  logger: Logger;
  mediaFileId: number;
};

/** Proceso de reproducción ya arrancado. */
export type PlayerHandle = {
  pid: number | undefined;
  /** Espera al cierre y falla si el reproductor termina con error. */
  wait: () => Promise<void>;
  /** Limpia el tracker si existe. */
  cleanupTracker?: () => Promise<void> | undefined;
};

/** Firma que consume la TUI, inyectable en los tests. */
export type LaunchPlayerFn = (plan: LaunchPlan, trackerDeps?: TrackerDeps) => Promise<PlayerHandle>;

/**
 * Construye el comando del reproductor a partir de la definición configurada.
 *
 * La plantilla debe contener `{file}`: sin ese marcador el reproductor se abriría
 * sin saber qué vídeo tiene que cargar. Cada argumento se sustituye por separado,
 * así que una ruta con espacios sigue siendo un único argumento.
 */
export function createLaunchPlan(options: CreateLaunchPlanOptions): LaunchPlan {
  const { playerConfig, filePath } = options;

  if (filePath.trim().length === 0) {
    throw new PlayerError('No hay ningún fichero que reproducir.');
  }

  const playerName = options.playerName ?? playerConfig.default;
  const definition = playerConfig.players[playerName];

  if (definition === undefined) {
    const available = Object.keys(playerConfig.players).sort().join(', ') || '(ninguno)';
    throw new PlayerError(
      `El reproductor «${playerName}» no está configurado. Disponibles: ${available}.`,
    );
  }

  if (!definition.args.some((arg) => arg.includes('{file}'))) {
    throw new PlayerError(
      `La plantilla del reproductor «${playerName}» debe incluir el marcador {file}.`,
    );
  }

  const hasStartPlaceholder = definition.args.some((arg) => arg.includes('{start}'));
  const startSeconds = options.startSeconds ?? null;
  let finalArgs = definition.args;
  if (hasStartPlaceholder) {
    finalArgs = finalArgs.map((arg) =>
      arg.replaceAll('{start}', startSeconds !== null ? Math.floor(startSeconds).toString() : ''),
    );
    finalArgs = finalArgs.filter((arg) => arg.length > 0);
  }

  let finalArgsResolved = finalArgs.map((arg) => arg.replaceAll('{file}', filePath));

  // Añadir el archivo de subtítulo como argumento adicional del reproductor.
  // mpv y vlc aceptan --sub-file=<ruta>; se pasa como argumento independiente
  // para que rutas con espacios se conserven como un solo argumento.
  if (options.subtitlePath) {
    finalArgsResolved = [...finalArgsResolved, `--sub-file=${options.subtitlePath}`];
  }

  // Seleccionar la pista de audio por índice (ffprobe es 0-based, reproductores son 1-based).
  // mpv usa --aid, vlc usa --audio-track.
  if (options.audioTrackIndex !== null && options.audioTrackIndex !== undefined) {
    const aid = options.audioTrackIndex + 1;
    if (playerName === 'mpv') {
      finalArgsResolved = [...finalArgsResolved, `--aid=${aid}`];
    } else if (playerName === 'vlc') {
      finalArgsResolved = [...finalArgsResolved, `--audio-track=${aid}`];
    }
  }

  // Seleccionar la pista de subtítulos por índice (ffprobe es 0-based, reproductores son 1-based).
  // mpv usa --sid, vlc usa --sub-track.
  if (options.subtitleTrackIndex !== null && options.subtitleTrackIndex !== undefined) {
    const sid = options.subtitleTrackIndex + 1;
    if (playerName === 'mpv') {
      finalArgsResolved = [...finalArgsResolved, `--sid=${sid}`];
    } else if (playerName === 'vlc') {
      finalArgsResolved = [...finalArgsResolved, `--sub-track=${sid}`];
    }
  }

  return {
    player: playerName,
    command: definition.command,
    args: finalArgsResolved,
    mode: options.mode ?? playerConfig.onLaunch,
    filePath,
    subtitlePath: options.subtitlePath ?? null,
    ...(definition.env === undefined ? {} : { env: { ...definition.env } }),
  };
}

/**
 * Lanza el reproductor sin shell.
 *
 * En modo `detach` el hijo se desacopla de la terminal y sobrevive al cierre de
 * la TUI; en modo `foreground` hereda stdio y `wait()` espera su salida. Comprobar
 * el binario antes de lanzarlo permite devolver un error útil aunque el proceso
 * se desacople inmediatamente.
 *
 * Si `trackerDeps` se proporciona, el reproductor es `mpv` y `playback.tracking` es
 * `auto`, se lanza el tracker IPC que guarda la posición periódicamente.
 */
export async function launchPlayer(
  plan: LaunchPlan,
  trackerDeps?: TrackerDeps,
): Promise<PlayerHandle> {
  if (!(await isExecutable(plan.command))) {
    throw new PlayerError(
      `No se encontró el reproductor «${plan.player}» (${plan.command}). ` +
        'Instálalo o ajusta player.players en la configuración.',
    );
  }

  const child = spawn(plan.command, plan.args, {
    shell: false,
    detached: plan.mode === 'detach',
    stdio: plan.mode === 'detach' ? 'ignore' : 'inherit',
    ...(plan.env === undefined ? {} : { env: { ...process.env, ...plan.env } }),
  });

  await waitForSpawn(child, plan);

  if (plan.mode === 'detach') child.unref();

  let cleanupTracker: (() => Promise<void>) | undefined;

  // Iniciar tracker IPC para mpv si procede
  if (
    trackerDeps &&
    plan.player === 'mpv' &&
    trackerDeps.playbackConfig.tracking === 'auto' &&
    plan.mode === 'detach'
  ) {
    try {
      cleanupTracker = await launchWithTracker(
        plan.command,
        plan.args.filter((arg) => arg !== plan.filePath), // args ya tiene {file} resuelto
        plan.filePath,
        {
          mediaFileId: trackerDeps.mediaFileId,
          playbackRepo: trackerDeps.playbackRepo,
          playbackConfig: trackerDeps.playbackConfig,
          logger: trackerDeps.logger.child({ component: 'mpv-tracker' }),
          socketPath: '', // se genera dentro
        },
      );
    } catch (error) {
      trackerDeps.logger.warn('No se pudo iniciar tracker mpv, continuando sin tracking', {
        error: formatError(error),
      });
    }
  }

  const closed = waitForClose(child, plan.mode);
  const handle: PlayerHandle = {
    pid: child.pid,
    wait: () => closed.then((outcome) => assertSuccessfulExit(outcome, plan)),
  };
  if (cleanupTracker) handle.cleanupTracker = cleanupTracker;
  return handle;
}

/**
 * Busca un ejecutable en `PATH` (o comprueba la ruta si contiene separadores).
 * Se hace antes del spawn para que el error no se pierda al desacoplar el hijo.
 */
export async function isExecutable(
  command: string,
  env: NodeJS.ProcessEnv = process.env,
): Promise<boolean> {
  if (command.length === 0) return false;

  const hasSeparator =
    command.includes('/') || (process.platform === 'win32' && command.includes('\\'));
  const directories = hasSeparator
    ? ['']
    : (env.PATH ?? '').split(path.delimiter).filter((directory) => directory.length > 0);
  const extensions =
    process.platform === 'win32' ? (env.PATHEXT ?? '.EXE;.CMD;.BAT;.COM').split(';') : [''];

  for (const directory of directories) {
    for (const extension of extensions) {
      const candidate = hasSeparator ? command : path.join(directory, `${command}${extension}`);
      try {
        await access(candidate, constants.X_OK);
        return true;
      } catch {
        // Siguiente candidato del PATH.
      }
    }
  }

  return false;
}

type CloseOutcome = {
  code: number | null;
  signal: NodeJS.Signals | null;
};

function waitForSpawn(child: ChildProcess, plan: LaunchPlan): Promise<void> {
  return new Promise((resolve, reject) => {
    const cleanup = (): void => {
      child.off('spawn', onSpawn);
      child.off('error', onError);
    };
    const onSpawn = (): void => {
      cleanup();
      resolve();
    };
    const onError = (error: Error): void => {
      cleanup();
      reject(
        new PlayerError(`No se pudo lanzar «${plan.player}»: ${formatError(error)}`, {
          cause: error,
        }),
      );
    };

    child.once('spawn', onSpawn);
    child.once('error', onError);
  });
}

function waitForClose(child: ChildProcess, mode: LaunchMode): Promise<CloseOutcome> {
  return new Promise((resolve, reject) => {
    child.once('close', (code, signal) => {
      resolve({ code, signal });
    });
    // En detach nadie consume la espera; un error tardío quedaría como rechazo
    // no manejado, así que solo se propaga cuando el llamante va a esperar.
    if (mode === 'foreground') child.once('error', reject);
  });
}

function assertSuccessfulExit(outcome: CloseOutcome, plan: LaunchPlan): void {
  if (outcome.signal !== null) {
    throw new PlayerError(
      `El reproductor «${plan.player}» terminó por la señal ${outcome.signal}.`,
    );
  }
  if (outcome.code !== 0) {
    throw new PlayerError(
      `El reproductor «${plan.player}» terminó con código ${outcome.code ?? 'desconocido'}.`,
    );
  }
}
