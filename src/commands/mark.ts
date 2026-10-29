import pc from 'picocolors';

import { withCommandContext, type CommandOptions } from './context.js';
import { toMediaFileJson, toMediaJson } from './output.js';
import { calculatePlayback, nowIso } from '../core/playback/index.js';
import { selectMediaFile } from '../core/player/selection.js';
import { ValidationError } from '../shared/errors.js';

import type { PlaybackStateRow } from '../core/db/types.js';

export type MarkCommandOptions = CommandOptions & {
  watched?: boolean;
  unwatched?: boolean;
  file?: number;
  position?: number;
  json?: boolean;
  dryRun?: boolean;
};

/** Implementa `mm mark`: marca estado manualmente. */
export async function runMarkCommand(query: string, options: MarkCommandOptions): Promise<void> {
  await withCommandContext(options, ({ library, playbackRepo, playbackConfig }) => {
    const { media, files } = library.find(query);

    // Validar que exactamente una acción
    const actions = [options.watched, options.unwatched, options.position !== undefined].filter(
      Boolean,
    ).length;
    if (actions === 0) {
      throw new ValidationError('Debe indicar una acción: --watched, --unwatched o --position.');
    }
    if (actions > 1) {
      throw new ValidationError(
        'Las opciones --watched, --unwatched y --position son mutuamente excluyentes.',
      );
    }

    // Resolver el fichero (usa el primero si hay varios, igual que play)
    const file = selectMediaFile(files, options.file);

    const state = playbackRepo.get(file.id);
    const dryRun = options.dryRun === true;
    const now = nowIso();

    const buildState = (overrides: Partial<PlaybackStateRow>): PlaybackStateRow => ({
      mediaFileId: file.id,
      positionSeconds: 0,
      durationSeconds: state?.durationSeconds ?? null,
      completedAt: null,
      lastPlayedAt: now,
      playCount: state?.playCount ?? 0,
      updatedAt: now,
      ...overrides,
    });

    let updatedState: PlaybackStateRow | null = null;

    if (options.watched) {
      // Marcar como visto: posición = duración (o 0 si no se conoce), completedAt = now
      const duration = state?.durationSeconds ?? 0;
      const input = {
        mediaFileId: file.id,
        positionSeconds: duration,
        durationSeconds: state?.durationSeconds ?? null,
        completedAt: now,
        lastPlayedAt: now,
        playCount: (state?.playCount ?? 0) + 1,
        updatedAt: now,
      };
      if (dryRun) {
        updatedState = buildState(input);
      } else {
        updatedState = playbackRepo.save(input);
      }
    } else if (options.unwatched) {
      // Marcar como no visto: posición = 0, limpiar completedAt
      const input = {
        mediaFileId: file.id,
        positionSeconds: 0,
        durationSeconds: state?.durationSeconds ?? null,
        completedAt: null,
        lastPlayedAt: now,
        playCount: state?.playCount ?? 0,
        updatedAt: now,
      };
      if (dryRun) {
        updatedState = buildState(input);
      } else {
        updatedState = playbackRepo.save(input);
      }
    } else if (options.position !== undefined) {
      // Posición manual
      const positionValue = options.position;
      if (positionValue < 0) {
        throw new ValidationError('La posición no puede ser negativa.');
      }
      const duration = state?.durationSeconds ?? null;
      const position = duration === null ? positionValue : Math.min(positionValue, duration);
      const input = {
        mediaFileId: file.id,
        positionSeconds: position,
        durationSeconds: state?.durationSeconds ?? null,
        completedAt: null, // posición manual no completa
        lastPlayedAt: now,
        playCount: state?.playCount ?? 0,
        updatedAt: now,
      };
      if (dryRun) {
        updatedState = buildState(input);
      } else {
        updatedState = playbackRepo.save(input);
      }
    }

    if (!updatedState) {
      throw new ValidationError('No se pudo determinar el estado de reproducción');
    }

    const dryRunLabel = dryRun ? ' [dry-run]' : '';

    if (options.json === true) {
      const calc = calculatePlayback(updatedState, playbackConfig, new Date().toISOString());
      console.log(
        JSON.stringify(
          {
            dryRun,
            media: toMediaJson(media),
            file: toMediaFileJson(file),
            position: calc.positionSeconds,
            duration: calc.durationSeconds,
            completed: calc.isCompleted,
            lastPlayedAt: updatedState.lastPlayedAt,
            playCount: updatedState.playCount,
          },
          null,
          2,
        ),
      );
      return;
    }

    const year = media.year === null ? '' : ` (${media.year})`;
    const calc = calculatePlayback(updatedState, playbackConfig, new Date().toISOString());
    const status = calc.isCompleted ? '✓ visto' : '● en curso';
    const progress = calc.progressPercent !== null ? ` [${calc.progressPercent.toFixed(0)}%]` : '';
    console.log(
      `${pc.green(status)} #${media.id} ${media.title}${year} · ${file.filePath}${progress}${dryRunLabel}`,
    );
  });
}
