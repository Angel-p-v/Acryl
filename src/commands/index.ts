import pc from 'picocolors';

import { withCommandContext, type CommandOptions } from './context.js';
import { indexLibrary, type IndexStats } from '../core/indexer/service.js';

export type IndexCommandOptions = CommandOptions & {
  dryRun?: boolean;
  root?: string;
  folder?: string;
  json?: boolean;
};

/** Implementa `mm index`: indexa la biblioteca configurada. */
export async function runIndexCommand(options: IndexCommandOptions): Promise<void> {
  await withCommandContext(options, async ({ config, excludeRef, repositories, logger }) => {
    const library = {
      ...config.library,
      exclude: () => excludeRef.exclude,
    };
    const stats = await indexLibrary(repositories, library, {
      ...(options.dryRun === undefined ? {} : { dryRun: options.dryRun }),
      ...(options.root === undefined ? {} : { rootPath: options.root }),
      ...(options.folder === undefined ? {} : { folderPath: options.folder }),
      logger,
    });

    for (const warning of stats.warnings) logger.warn(warning);

    if (options.json) {
      console.log(JSON.stringify(stats, null, 2));
      return;
    }

    printSummary(stats);
  });
}

/** Resume la ejecución en lenguaje natural. */
function printSummary(stats: IndexStats): void {
  const icon = stats.dryRun ? pc.yellow('·') : pc.green('✔');
  const title = stats.dryRun ? 'Indexación simulada' : 'Indexación completada';

  console.log(`${icon} ${title} en ${formatDuration(stats.durationMs)}`);
  if (stats.dryRun) {
    console.log(pc.dim('  No se ha escrito nada en la base de datos.'));
  }
  console.log(
    `  ${stats.filesSeen} ficheros vistos · ${stats.filesAdded} añadidos o actualizados · ` +
      `${stats.filesUnchanged} sin cambios · ${stats.filesRemoved} eliminados`,
  );
  console.log(
    `  ${stats.mediaAdded} títulos nuevos · ${stats.mediaRemoved} eliminados · ` +
      `${stats.skipped} omitidos`,
  );
}

function formatDuration(durationMs: number): string {
  if (durationMs < 1000) return `${durationMs} ms`;
  return `${(durationMs / 1000).toFixed(2)} s`;
}
