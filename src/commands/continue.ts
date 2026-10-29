import { withCommandContext, type CommandOptions } from './context.js';
import { toMediaFileJson, toMediaJson } from './output.js';
import { calculatePlayback } from '../core/playback/index.js';

import type { PlaybackConfig } from '../core/config/schema.js';
import type { PlaybackEntryRow } from '../core/db/repositories/playbackRepo.js';

export type ContinueCommandOptions = CommandOptions & {
  limit?: number;
  json?: boolean;
};

/** Implementa `mm continue`: lista títulos en curso. */
export async function runContinueCommand(options: ContinueCommandOptions): Promise<void> {
  await withCommandContext(options, ({ playbackRepo, playbackConfig }) => {
    const entries = playbackRepo.listInProgress(options.limit ?? 50);

    if (options.json === true) {
      console.log(
        JSON.stringify(
          {
            items: entries.map((entry) => toContinueJson(entry, playbackConfig)),
          },
          null,
          2,
        ),
      );
      return;
    }

    if (entries.length === 0) {
      console.log('No hay nada en curso.');
      return;
    }

    console.log('En curso:');
    for (const entry of entries) {
      const calc = calculatePlayback(entry.state, playbackConfig, new Date().toISOString());
      const year = entry.media.year === null ? '' : ` (${entry.media.year})`;
      const progress =
        calc.progressPercent !== null ? ` [${calc.progressPercent.toFixed(0)}%]` : '';
      console.log(
        `  #${entry.media.id} ${entry.media.title}${year} · ${entry.file.filePath}${progress}`,
      );
    }
  });
}

function toContinueJson(entry: PlaybackEntryRow, config: PlaybackConfig) {
  const calc = calculatePlayback(entry.state, config, new Date().toISOString());
  return {
    media: toMediaJson(entry.media),
    file: toMediaFileJson(entry.file),
    position: calc.positionSeconds,
    duration: calc.durationSeconds,
    completed: calc.isCompleted,
    lastPlayedAt: entry.state.lastPlayedAt,
    playCount: entry.state.playCount,
  };
}
