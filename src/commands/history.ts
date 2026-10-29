import { withCommandContext, type CommandOptions } from './context.js';
import { toMediaFileJson, toMediaJson } from './output.js';
import { calculatePlayback } from '../core/playback/index.js';

import type { PlaybackConfig } from '../core/config/schema.js';
import type { PlaybackEntryRow } from '../core/db/repositories/playbackRepo.js';

export type HistoryCommandOptions = CommandOptions & {
  limit?: number;
  json?: boolean;
};

/** Implementa `mm history`: muestra historial reciente. */
export async function runHistoryCommand(options: HistoryCommandOptions): Promise<void> {
  await withCommandContext(options, ({ playbackRepo, playbackConfig }) => {
    const entries = playbackRepo.listRecent(options.limit ?? 50);

    if (options.json === true) {
      console.log(
        JSON.stringify(
          {
            items: entries.map((entry) => toHistoryJson(entry, playbackConfig)),
          },
          null,
          2,
        ),
      );
      return;
    }

    if (entries.length === 0) {
      console.log('Historial vacío.');
      return;
    }

    console.log('Historial:');
    for (const entry of entries) {
      const calc = calculatePlayback(entry.state, playbackConfig, new Date().toISOString());
      const year = entry.media.year === null ? '' : ` (${entry.media.year})`;
      const status = calc.isCompleted ? '✓' : '●';
      const progress =
        calc.progressPercent !== null ? ` [${calc.progressPercent.toFixed(0)}%]` : '';
      console.log(
        `  ${status} #${entry.media.id} ${entry.media.title}${year} · ${entry.file.filePath}${progress}`,
      );
    }
  });
}

function toHistoryJson(entry: PlaybackEntryRow, config: PlaybackConfig) {
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
