import pc from 'picocolors';

import { withCommandContext, type CommandOptions } from './context.js';
import { clearPosterCache, DEFAULT_CACHE_DIR } from '../core/metadata/posterCache.js';
import { MetadataService } from '../core/metadata/service.js';
import { formatError } from '../shared/errors.js';

export type MetadataCommandOptions = CommandOptions & {
  action?: 'fetch' | 'search' | 'clear' | 'cache';
  mediaId?: number;
  query?: string;
  provider?: 'tvmaze' | 'jikan';
  externalId?: number;
  json?: boolean;
  cacheClear?: boolean;
  maxAgeDays?: number;
  dryRun?: boolean;
};

/** Implementa `mm metadata`: gestiona metadatos externos (proveedores públicos gratuitos). */
export async function runMetadataCommand(options: MetadataCommandOptions): Promise<void> {
  await withCommandContext(options, async ({ library, logger }) => {
    const service = new MetadataService({ logger });

    switch (options.action) {
      case 'search': {
        if (!options.query) {
          console.error(
            pc.red('Uso: mm metadata search <consulta> [--provider tvmaze|jikan] [--json]'),
          );
          process.exitCode = 2;
          return;
        }

        try {
          const results = await service.search(options.query);
          if (options.json) {
            console.log(JSON.stringify({ query: options.query, results }, null, 2));
          } else {
            if (results.length === 0) {
              console.log(pc.dim('No se encontraron resultados.'));
            } else {
              console.log(pc.bold(`Resultados para "${options.query}":`));
              for (const r of results) {
                console.log(
                  `  #${r.externalId}  ${r.title}  (${r.provider}, ${r.releaseDate ?? 'sin fecha'})`,
                );
              }
            }
          }
        } catch (error) {
          console.error(pc.red(`Error buscando: ${formatError(error)}`));
          process.exitCode = 1;
        }
        return;
      }
      case 'fetch': {
        if (!options.mediaId || !options.provider || !options.externalId) {
          console.error(
            pc.red(
              'Uso: mm metadata fetch <id> --provider tvmaze|jikan --external-id <id> [--json]',
            ),
          );
          process.exitCode = 2;
          return;
        }

        try {
          const details = await service.getDetails(
            options.externalId,
            options.provider,
            options.provider === 'jikan' ? 'tv' : 'tv',
          );

          const saved = library.setMetadata({
            mediaId: options.mediaId,
            provider: options.provider,
            externalId: options.externalId,
            title: details.title,
            overview: details.overview,
            posterPath: details.posterPath,
            backdropPath: details.backdropPath,
            voteAverage: details.voteAverage,
            releaseDate: details.releaseDate,
            originalLanguage: details.originalLanguage,
          });

          if (options.json) {
            console.log(JSON.stringify(saved, null, 2));
          } else {
            console.log(pc.green(`Metadatos guardados para el título #${options.mediaId}.`));
            console.log(`  Título: ${saved.title}`);
            console.log(`  Voto: ${saved.voteAverage ?? 'N/A'}`);
            console.log(`  Fecha: ${saved.releaseDate ?? 'N/A'}`);
          }
        } catch (error) {
          console.error(pc.red(`Error obteniendo metadatos: ${formatError(error)}`));
          process.exitCode = 1;
        }
        return;
      }
      case 'clear': {
        if (!options.mediaId) {
          console.error(pc.red('Uso: mm metadata clear <id> [--json] [--dry-run]'));
          process.exitCode = 2;
          return;
        }

        const dryRun = options.dryRun === true;
        const existing = library.getMetadata(options.mediaId);
        const wouldRemove = existing !== undefined;

        if (dryRun) {
          if (options.json) {
            console.log(
              JSON.stringify(
                {
                  action: 'clear',
                  dryRun: true,
                  mediaId: options.mediaId,
                  wouldRemove,
                },
                null,
                2,
              ),
            );
          } else {
            if (wouldRemove) {
              console.log(
                pc.yellow(`[dry-run] Se eliminarían los metadatos del título #${options.mediaId}.`),
              );
            } else {
              console.log(
                pc.dim(`[dry-run] No había metadatos para el título #${options.mediaId}.`),
              );
            }
          }
          return;
        }

        const removed = library.removeMetadata(options.mediaId);
        if (options.json) {
          console.log(
            JSON.stringify({ action: 'clear', mediaId: options.mediaId, removed }, null, 2),
          );
        } else {
          if (removed) {
            console.log(pc.green(`Metadatos eliminados del título #${options.mediaId}.`));
          } else {
            console.log(pc.yellow(`No había metadatos para el título #${options.mediaId}.`));
          }
        }
        return;
      }
      case 'cache': {
        if (!options.cacheClear) {
          console.error(pc.red('Uso: mm metadata cache --clear [--max-age-dias <n>] [--json]'));
          process.exitCode = 2;
          return;
        }

        const maxAgeDays = options.maxAgeDays ?? 30;

        try {
          const result = await clearPosterCache(DEFAULT_CACHE_DIR, maxAgeDays);

          if (options.json) {
            console.log(
              JSON.stringify(
                {
                  action: 'cache-clear',
                  deleted: result.deleted,
                  errors: result.errors,
                },
                null,
                2,
              ),
            );
          } else {
            console.log(
              pc.green(
                `Caché de pósters limpiado: ${result.deleted} archivo(s) eliminado(s) (más de ${maxAgeDays} días).`,
              ),
            );
            if (result.errors.length > 0) {
              console.error(
                pc.yellow(`Se produjeron ${result.errors.length} error(es) al limpiar el caché.`),
              );
            }
          }
        } catch (error) {
          console.error(pc.red(`Error limpiando caché: ${formatError(error)}`));
          process.exitCode = 1;
        }
        return;
      }
      default: {
        console.error(pc.red('Uso: mm metadata <fetch|search|clear|cache> [opciones] [--json]'));
        process.exitCode = 2;
        return;
      }
    }
  });
}
