import { PlayerError } from '../../shared/errors.js';

import type { MediaType } from '../config/schema.js';
import type { MediaFileRow, MediaRow, PlaybackStateRow } from '../db/types.js';

/**
 * Configuración de reproducción validada. Se inyecta desde el archivo YAML para
 * que el dominio sea testeable sin depender de la configuración global.
 */
export type PlaybackConfig = {
  tracking: 'auto' | 'manual' | 'off';
  resume: boolean;
  resumeMinSeconds: number;
  completedThreshold: number;
  completedTailSeconds: number;
};

/** Estado de reproducción calculado para un fichero. */
export type PlaybackCalculated = {
  positionSeconds: number;
  durationSeconds: number | null;
  completedAt: string | null;
  lastPlayedAt: string | null;
  playCount: number;
  isCompleted: boolean;
  canResume: boolean;
  progressPercent: number | null;
};

/** Episodio siguiente con su fichero. */
export type NextEpisode = {
  media: MediaRow;
  file: MediaFileRow;
};

/**
 * Calcula el estado de reproducción a partir de la fila almacenada y la
 * configuración. El dominio no muta nada; solo deriva valores.
 */
function calculatePlaybackImpl(
  state: PlaybackStateRow | undefined,
  config: PlaybackConfig,
  nowIso: string,
): PlaybackCalculated {
  if (!state) {
    return {
      positionSeconds: 0,
      durationSeconds: null,
      completedAt: null,
      lastPlayedAt: nowIso,
      playCount: 0,
      isCompleted: false,
      canResume: false,
      progressPercent: null,
    };
  }

  const duration = state.durationSeconds;
  const position = Math.max(0, state.positionSeconds);
  const completedAt = state.completedAt;

  let isCompleted = false;
  if (completedAt !== null) {
    isCompleted = true;
  } else if (duration !== null && duration > 0) {
    const byThreshold = position >= duration * config.completedThreshold;
    const byTail = duration - position <= config.completedTailSeconds;
    isCompleted = byThreshold || byTail;
  }

  const canResume =
    config.resume &&
    !isCompleted &&
    position >= config.resumeMinSeconds &&
    (duration === null || position < duration);

  let progressPercent: number | null = null;
  if (duration !== null && duration > 0) {
    progressPercent = Math.min(100, Math.max(0, (position / duration) * 100));
  }

  return {
    positionSeconds: position,
    durationSeconds: duration,
    completedAt,
    lastPlayedAt: state.lastPlayedAt,
    playCount: state.playCount,
    isCompleted,
    canResume,
    progressPercent,
  };
}

/**
 * Decide si reanudar, reiniciar o ignorar el estado guardado según los
 * argumentos de la CLI y la configuración.
 */
export type ResumeDecision = 'resume' | 'restart' | 'none';

export type ResumeOptions = {
  resume: boolean;
  restart: boolean;
  hasSavedState: boolean;
  canResume: boolean;
  positionSeconds: number;
};

/**
 * Resuelve la decisión de reanudación.
 *
 * - `--restart` siempre gana y fuerza el inicio desde cero.
 * - `--resume` fuerza reanudar; error si no hay estado reanudable.
 * - Sin flags: reanuda si `config.resume` y hay estado reanudable.
 */
function resolveResumeDecisionImpl(options: ResumeOptions): ResumeDecision {
  if (options.restart) return 'restart';
  if (options.resume) {
    if (!options.hasSavedState || !options.canResume) {
      throw new PlayerError('No hay posición guardada reanudable para --resume');
    }
    return 'resume';
  }
  // Por defecto (sin flags): reanuda si la configuración lo permite.
  if (options.hasSavedState && options.canResume) return 'resume';
  return 'none';
}

/**
 * Encuentra el siguiente episodio no completado de una serie/anime.
 * Los especiales (season = 0) no se mezclan con la temporada regular.
 * Devuelve `null` si no hay siguiente (último episodio, película, o todos vistos).
 */
function findNextEpisodeImpl(
  current: MediaFileRow,
  allFiles: MediaFileRow[],
  playbackRepo: { get(id: number): PlaybackStateRow | undefined },
  config: PlaybackConfig,
): NextEpisode | null {
  if (current.season === null || current.episode === null) return null;
  if (current.mediaId === undefined) return null;

  const currentSeason = current.season;
  const currentEpisode = current.episode;

  // Buscar en la misma temporada
  const candidates = allFiles
    .filter(
      (f) =>
        f.season === currentSeason &&
        f.episode !== null &&
        f.episode > currentEpisode &&
        f.mediaId === current.mediaId,
    )
    .sort((a, b) => (a.episode ?? 0) - (b.episode ?? 0));

  if (candidates.length > 0) {
    for (const file of candidates) {
      const state = playbackRepo.get(file.id);
      const calc = calculatePlaybackImpl(state, config, new Date().toISOString());
      if (!calc.isCompleted) return { media: { id: current.mediaId } as MediaRow, file };
    }
    return null; // hay candidatos pero todos completados
  }

  // Buscar primera temporada siguiente
  const futureSeasons = allFiles
    .filter(
      (f) =>
        f.season !== null &&
        f.season > currentSeason &&
        f.episode !== null &&
        f.mediaId === current.mediaId,
    )
    .sort((a, b) => (a.season ?? 0) - (b.season ?? 0) || (a.episode ?? 0) - (b.episode ?? 0));

  for (const file of futureSeasons) {
    const state = playbackRepo.get(file.id);
    const calc = calculatePlaybackImpl(state, config, new Date().toISOString());
    if (!calc.isCompleted) return { media: { id: current.mediaId } as MediaRow, file };
  }

  return null;
}

/**
 * Normaliza el tipo de contenido para comparaciones. `series` y `anime`
 * se tratan igual para buscar el siguiente episodio.
 */
function isEpisodicImpl(type: MediaType): boolean {
  return type === 'series' || type === 'anime';
}

/**
 * Genera una marca de tiempo ISO-8601 en UTC para el dominio.
 * Se separa del reloj del sistema para que los tests puedan inyectar `now`.
 */
function nowIsoImpl(now: Date = new Date()): string {
  return now.toISOString();
}

/**
 * Calcula los argumentos de lanzamiento con `{start}` sustituido por los
 * segundos enteros de la posición, o elimina el argumento si no hay
 * reanudación.
 */
function applyStartPlaceholderImpl(args: string[], startSeconds: number | null): string[] {
  if (startSeconds === null) {
    return args.filter((arg) => !arg.includes('{start}'));
  }
  const value = Math.floor(startSeconds).toString();
  return args.map((arg) => arg.replaceAll('{start}', value));
}

/** Exportaciones nombradas para compatibilidad con ESM y verbatimModuleSyntax. */
export {
  calculatePlaybackImpl as calculatePlayback,
  resolveResumeDecisionImpl as resolveResumeDecision,
  findNextEpisodeImpl as findNextEpisode,
  isEpisodicImpl as isEpisodic,
  nowIsoImpl as nowIso,
  applyStartPlaceholderImpl as applyStartPlaceholder,
};
