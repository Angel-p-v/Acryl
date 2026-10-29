import pc from 'picocolors';

import { withCommandContext, type CommandOptions } from './context.js';
import { formatEpisode, toMediaFileJson, toMediaJson } from './output.js';
import {
  calculatePlayback,
  findNextEpisode,
  isEpisodic,
  nowIso,
  resolveResumeDecision,
} from '../core/playback/index.js';
import {
  createLaunchPlan,
  launchPlayer,
  type LaunchMode,
  type LaunchPlan,
  type TrackerDeps,
} from '../core/player/launcher.js';
import { selectMediaFile } from '../core/player/selection.js';
import { ValidationError } from '../shared/errors.js';
import { createLogger } from '../shared/logger.js';

import type { MediaFileRow, MediaRow } from '../core/db/types.js';

export type PlayCommandOptions = CommandOptions & {
  file?: number;
  player?: string;
  foreground?: boolean;
  dryRun?: boolean;
  json?: boolean;
  resume?: boolean;
  restart?: boolean;
  next?: boolean;
  /** Ruta al archivo de subtítulo (.srt, .sub, .vtt, …) a cargar. */
  subtitle?: string;
};

type PlayJson = {
  player: string;
  command: string;
  args: string[];
  mode: LaunchMode;
  media: ReturnType<typeof toMediaJson>;
  file: ReturnType<typeof toMediaFileJson>;
  pid: number | null;
  /** Ruta al archivo de subtítulo cargado, si se indicó. */
  subtitle: string | null;
  playback: {
    resumedFrom: number | null;
    position: number;
    duration: number | null;
    completed: boolean;
  };
};

/** Implementa `mm play`: resuelve un título y lo abre en el reproductor configurado. */
export async function runPlayCommand(query: string, options: PlayCommandOptions): Promise<void> {
  await withCommandContext(options, async ({ config, library, playbackRepo, playbackConfig }) => {
    // --next busca el siguiente episodio no completado
    const { media: mediaFromFind, files } = library.find(query);
    let media = mediaFromFind;
    let file: MediaFileRow;
    let startSeconds: number | null = null;
    let resumedFrom: number | null = null;

    if (options.next) {
      if (!isEpisodic(media.type)) {
        throw new ValidationError('La opción --next solo está disponible para series y anime.');
      }
      const current = selectMediaFile(files, options.file);
      const next = findNextEpisode(current, files, playbackRepo, playbackConfig);
      if (!next) {
        throw new ValidationError('No hay siguiente episodio no completado.');
      }
      media = next.media;
      file = next.file;
    } else {
      file = selectMediaFile(files, options.file);
    }

    // Calcular estado de reproducción y decidir si reanudar
    const state = playbackRepo.get(file.id);
    const calculated = calculatePlayback(state, playbackConfig, nowIso());

    const decision = resolveResumeDecision({
      resume: options.resume === true,
      restart: options.restart === true,
      hasSavedState: state !== undefined,
      canResume: calculated.canResume,
      positionSeconds: calculated.positionSeconds,
    });

    if (decision === 'resume') {
      startSeconds = calculated.positionSeconds;
      resumedFrom = calculated.positionSeconds;
    }

    // Incrementar contador de reproducción si se lanza (no en dry-run)
    if (options.dryRun !== true) {
      playbackRepo.save({
        mediaFileId: file.id,
        positionSeconds: state?.positionSeconds ?? 0,
        durationSeconds: state?.durationSeconds ?? null,
        completedAt: state?.completedAt ?? null,
        lastPlayedAt: nowIso(),
        playCount: (state?.playCount ?? 0) + 1,
        updatedAt: nowIso(),
      });
    }

    const mode: LaunchMode = options.foreground === true ? 'foreground' : config.player.onLaunch;

    const plan = createLaunchPlan({
      playerConfig: config.player,
      filePath: file.filePath,
      ...(options.player === undefined ? {} : { playerName: options.player }),
      mode,
      startSeconds,
      ...(options.subtitle === undefined ? {} : { subtitlePath: options.subtitle }),
    });

    if (options.dryRun === true) {
      if (options.json === true) {
        console.log(
          JSON.stringify(toPlayJson(plan, media, file, null, resumedFrom, calculated), null, 2),
        );
        return;
      }

      console.log(pc.bold('Comando de reproducción') + pc.dim(` (${plan.mode})`));
      console.log(`  ${formatCommand(plan.command, plan.args)}`);
      console.log(pc.dim(`  ${media.title} · ${formatEpisode(file)} · ${plan.filePath}`));
      if (plan.subtitlePath) {
        console.log(pc.dim(`  Subtítulo: ${plan.subtitlePath}`));
      }
      if (resumedFrom !== null) {
        console.log(pc.dim(`  Reanudando desde ${formatSeconds(resumedFrom)}`));
      }
      console.log(pc.dim('  No se ha lanzado ningún proceso.'));
      return;
    }

    const trackerDeps: TrackerDeps = {
      playbackRepo,
      playbackConfig,
      logger: options.verbose
        ? createLogger({ level: 'debug', format: 'text' })
        : createLogger({ level: 'warn', format: 'text' }),
      mediaFileId: file.id,
    };
    const handle = await launchPlayer(plan, trackerDeps);

    if (options.json === true) {
      if (plan.mode === 'foreground') await handle.wait();
      console.log(
        JSON.stringify(
          toPlayJson(plan, media, file, handle.pid ?? null, resumedFrom, calculated),
          null,
          2,
        ),
      );
      return;
    }

    const year = media.year === null ? '' : ` (${media.year})`;
    console.log(
      `▶ ${pc.bold(`${media.title}${year}`)} · ${pc.cyan(formatEpisode(file))} · ${plan.player}`,
    );
    console.log(pc.dim(`  ${plan.filePath}`));
    if (plan.subtitlePath) {
      console.log(pc.dim(`  Subtítulo: ${plan.subtitlePath}`));
    }
    if (resumedFrom !== null) {
      console.log(pc.dim(`  Reanudando desde ${formatSeconds(resumedFrom)}`));
    }

    if (plan.mode === 'foreground') {
      await handle.wait();
      console.log(pc.dim('El reproductor se ha cerrado.'));
    }

    // Limpiar tracker si existe
    if (handle.cleanupTracker) {
      await handle.cleanupTracker();
    }
  });
}

function toPlayJson(
  plan: LaunchPlan,
  media: MediaRow,
  file: MediaFileRow,
  pid: number | null,
  resumedFrom: number | null,
  calculated: ReturnType<typeof calculatePlayback>,
): PlayJson {
  return {
    player: plan.player,
    command: plan.command,
    args: plan.args,
    mode: plan.mode,
    media: toMediaJson(media),
    file: toMediaFileJson(file),
    pid,
    subtitle: plan.subtitlePath ?? null,
    playback: {
      resumedFrom,
      position: calculated.positionSeconds,
      duration: calculated.durationSeconds,
      completed: calculated.isCompleted,
    },
  };
}

function formatSeconds(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  return h > 0
    ? `${h}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`
    : `${m}:${s.toString().padStart(2, '0')}`;
}

/** Representa el comando para humanos; no se usa para ejecutarlo. */
function formatCommand(command: string, args: readonly string[]): string {
  return [command, ...args].map(quoteArgument).join(' ');
}

function quoteArgument(value: string): string {
  if (/^[A-Za-z0-9_./:=+,@%-]+$/.test(value)) return value;
  return `'${value.replaceAll("'", `'\\''`)}'`;
}
