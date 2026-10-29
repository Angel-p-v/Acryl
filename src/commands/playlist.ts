import pc from 'picocolors';

import { withCommandContext, type CommandOptions } from './context.js';
import { formatEpisode, toMediaFileJson, toMediaJson } from './output.js';
import { createLaunchPlan, launchPlayer, type LaunchPlayerFn } from '../core/player/launcher.js';
import { selectMediaFile } from '../core/player/selection.js';

import type { MediaFileRow, MediaRow } from '../core/db/types.js';

export type PlaylistCommandOptions = CommandOptions & {
  dryRun?: boolean;
  json?: boolean;
};

/** Resultado planeado de un título en la playlist. */
type PlannedItem = {
  media: ReturnType<typeof toMediaJson>;
  file: ReturnType<typeof toMediaFileJson>;
  filePath: string;
};

/** Resultado de ejecución de un ítem de la playlist. */
type ExecutedItem = PlannedItem & {
  pid: number | null;
  error: string | null;
};

/** Resultado completo de la playlist. */
export type PlaylistResult = {
  total: number;
  executed: ExecutedItem[];
  completed: number;
  failed: number;
};

/**
 * Resuelve un título o id a su primer fichero reproducible.
 * Lanza ValidationError si no se encuentra o no tiene ficheros.
 */
function resolvePlayable(
  library: { find(query: string): { media: MediaRow; files: MediaFileRow[] } },
  query: string,
): { media: MediaRow; file: MediaFileRow } {
  const { media, files } = library.find(query);
  const file = selectMediaFile(files);
  return { media, file };
}

/**
 * Ejecuta una lista de reproducción: lanza cada título en modo foreground,
 * esperando a que termine antes de pasar al siguiente.
 */
export async function runPlaylistCommand(
  queries: string[],
  options: PlaylistCommandOptions,
  launchPlayerFn: LaunchPlayerFn = launchPlayer,
): Promise<void> {
  if (queries.length === 0) {
    console.error(pc.red('Uso: mm playlist <titulo-o-id> [titulo-o-id ...] [--json] [--dry-run]'));
    process.exitCode = 2;
    return;
  }

  await withCommandContext(options, async ({ config, library }) => {
    if (options.dryRun === true) {
      // Resolver todos solo para mostrar qué se reproduciría (no lanza nada)
      const planned: PlannedItem[] = [];
      for (const query of queries) {
        const { media, file } = resolvePlayable(library, query);
        planned.push({
          media: toMediaJson(media),
          file: toMediaFileJson(file),
          filePath: file.filePath,
        });
      }

      if (options.json === true) {
        console.log(JSON.stringify({ total: planned.length, items: planned }, null, 2));
        return;
      }
      console.log(pc.bold('Playlist (simulación)'));
      for (const item of planned) {
        console.log(`  · ${item.media.title} · ${formatEpisode(item.file)}`);
      }
      console.log(pc.dim('  No se ha lanzado ningún proceso.'));
      return;
    }

    const executed: ExecutedItem[] = [];
    let completed = 0;
    let failed = 0;

    for (const query of queries) {
      // Resolver título justo antes de reproducirlo; si falla, se marca y continúa
      let media: MediaRow;
      let file: MediaFileRow;
      try {
        ({ media, file } = resolvePlayable(library, query));
      } catch (error) {
        const now = new Date().toISOString();
        const failedMedia: MediaRow = {
          id: 0,
          type: 'movie',
          title: query,
          sortTitle: query.toLocaleLowerCase(),
          year: null,
          folderPath: query,
          createdAt: now,
          updatedAt: now,
        };
        const failedFile: MediaFileRow = {
          id: 0,
          mediaId: 0,
          filePath: query,
          season: null,
          episode: null,
          title: null,
          sizeBytes: null,
          mtimeMs: 0,
          indexedAt: now,
        };
        executed.push({
          media: toMediaJson(failedMedia),
          file: toMediaFileJson(failedFile),
          filePath: query,
          pid: null,
          error: (error as Error).message,
        });
        failed += 1;
        if (options.json !== true) {
          console.log(pc.red(`✖ ${query}`) + pc.dim(` · ${(error as Error).message}`));
        }
        continue;
      }

      const plan = createLaunchPlan({
        playerConfig: config.player,
        filePath: file.filePath,
        mode: 'foreground',
      });

      try {
        const handle = await launchPlayerFn(plan);
        await handle.wait();
        executed.push({
          ...{ media: toMediaJson(media), file: toMediaFileJson(file), filePath: file.filePath },
          pid: handle.pid ?? null,
          error: null,
        });
        completed += 1;
        if (options.json !== true) {
          console.log(pc.green(`✔ ${media.title}`) + pc.dim(` · ${formatEpisode(file)}`));
        }
      } catch (error) {
        executed.push({
          ...{ media: toMediaJson(media), file: toMediaFileJson(file), filePath: file.filePath },
          pid: null,
          error: (error as Error).message,
        });
        failed += 1;
        if (options.json !== true) {
          console.log(
            pc.red(`✖ ${media.title}`) +
              pc.dim(` · ${formatEpisode(file)} · ${(error as Error).message}`),
          );
        }
      }
    }

    if (options.json === true) {
      console.log(JSON.stringify({ total: queries.length, executed, completed, failed }, null, 2));
      return;
    }

    console.log(
      pc.bold(
        `Playlist completada: ${completed} exitosos, ${failed} fallidos de ${queries.length}`,
      ),
    );
  });
}
