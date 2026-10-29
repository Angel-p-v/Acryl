import pc from 'picocolors';

import { withCommandContext, type CommandOptions } from './context.js';
import { indexLibrary, type IndexStats } from '../core/indexer/service.js';
import { watchLibrary as startWatching } from '../core/indexer/watcher.js';
import { formatError, IndexError } from '../shared/errors.js';

import type { LibraryWatcher, WatchLibraryOptions } from '../core/indexer/watcher.js';

export type WatchCommandOptions = CommandOptions;

/** Dependencias inyectables para probar el comando sin dejar un watcher real vivo. */
export type WatchCommandDependencies = {
  watchLibrary: (options: WatchLibraryOptions) => LibraryWatcher;
  waitForSignal: () => Promise<NodeJS.Signals>;
};

const defaultDependencies: WatchCommandDependencies = {
  watchLibrary: startWatching,
  waitForSignal,
};

/**
 * Implementa `mm watch`: indexa al arrancar y vuelve a indexar cuando cambian
 * ficheros de vídeo. La indexación es incremental y las ráfagas se agrupan.
 */
export async function runWatchCommand(
  options: WatchCommandOptions,
  dependencies: WatchCommandDependencies = defaultDependencies,
): Promise<void> {
  await withCommandContext(options, async ({ config, excludeRef, repositories, logger }) => {
    if (config.library.roots.length === 0) {
      throw new IndexError(
        'No hay ninguna raíz configurada. Añade al menos una entrada en library.roots.',
      );
    }

    const library = {
      ...config.library,
      exclude: () => excludeRef.exclude,
    };

    const initial = await indexLibrary(repositories, library);
    console.log(pc.green('✔') + ' Índice inicial actualizado.');
    printStats(initial);

    const watcher = dependencies.watchLibrary({
      repositories,
      library,
      logger,
      onIndexed: (stats) => {
        console.log(pc.green('✔') + ' Cambios detectados; índice actualizado.');
        printStats(stats);
      },
      onError: (error) => {
        console.error(pc.red(formatError(error)));
      },
    });

    try {
      await watcher.ready;
    } catch (error) {
      await watcher.close();
      throw error;
    }

    console.log(
      pc.dim(
        `Vigilando ${config.library.roots.length} ${
          config.library.roots.length === 1 ? 'raíz' : 'raíces'
        } · Ctrl+C para salir.`,
      ),
    );

    await dependencies.waitForSignal();
    await watcher.close();
    console.log(pc.dim('Vigilancia detenida.'));
  });
}

function printStats(stats: IndexStats): void {
  console.log(
    pc.dim(
      `  ${stats.filesSeen} vistos · ${stats.filesAdded} añadidos o actualizados · ` +
        `${stats.filesUnchanged} sin cambios · ${stats.filesRemoved} eliminados · ` +
        `${stats.durationMs} ms`,
    ),
  );
}

/** Resuelve con el primer SIGINT/SIGTERM; retira el otro handler para no dejarlo colgado. */
function waitForSignal(): Promise<NodeJS.Signals> {
  return new Promise((resolve) => {
    const finish = (signal: NodeJS.Signals): void => {
      process.off('SIGINT', onInt);
      process.off('SIGTERM', onTerm);
      resolve(signal);
    };
    const onInt = (): void => {
      finish('SIGINT');
    };
    const onTerm = (): void => {
      finish('SIGTERM');
    };

    process.on('SIGINT', onInt);
    process.on('SIGTERM', onTerm);
  });
}
