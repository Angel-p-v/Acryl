import pc from 'picocolors';

import { withCommandContext, type CommandOptions } from './context.js';
import { printMediaList, toMediaJson } from './output.js';

import type { MediaType } from '../core/config/schema.js';

export type SearchCommandOptions = CommandOptions & {
  type?: MediaType;
  limit?: number;
  json?: boolean;
};

/** Implementa `mm search`: búsqueda de texto completo sobre los títulos indexados. */
export async function runSearchCommand(
  query: string,
  options: SearchCommandOptions,
): Promise<void> {
  await withCommandContext(options, ({ library, logger }) => {
    const startedAt = performance.now();
    const results = library.search(query, {
      ...(options.type === undefined ? {} : { type: options.type }),
      ...(options.limit === undefined ? {} : { limit: options.limit }),
    });
    logger.debug('search.completed', {
      query,
      type: options.type ?? null,
      results: results.length,
      durationMs: Math.round(performance.now() - startedAt),
    });

    if (options.json) {
      console.log(
        JSON.stringify(
          {
            query,
            count: results.length,
            results: results.map(toMediaJson),
          },
          null,
          2,
        ),
      );
      return;
    }

    if (results.length === 0) {
      console.log(pc.dim(`Sin resultados para «${query}».`));
      return;
    }

    console.log(pc.bold(`${results.length} resultados`) + pc.dim(` para «${query}»`));
    printMediaList(results);
  });
}
