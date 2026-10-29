import pc from 'picocolors';

import { withCommandContext, type CommandOptions } from './context.js';
import { parseMediaType, parseSortOrder } from './options.js';
import { mediaCsvHeader, printMediaList, toMediaCsv, toMediaJson } from './output.js';
import { ValidationError } from '../shared/errors.js';

import type { MediaType } from '../core/config/schema.js';
import type { ListMediaOptions } from '../core/db/repositories/mediaRepo.js';

export type ListCommandOptions = CommandOptions & {
  type?: MediaType;
  limit?: number;
  offset?: number;
  sort?: NonNullable<ListMediaOptions['orderBy']>;
  json?: boolean;
  export?: 'json' | 'csv';
  favorites?: boolean;
  inProgress?: boolean;
  unwatched?: boolean;
  watched?: boolean;
  tags?: number[];
};

const DEFAULT_LIMIT = 50;

/** Implementa `mm list`: enumera la biblioteca con filtros y paginación. */
export async function runListCommand(options: ListCommandOptions): Promise<void> {
  const limit = options.limit ?? DEFAULT_LIMIT;
  const offset = options.offset ?? 0;
  const sort = options.sort ?? 'title';

  if (options.export !== undefined && options.export !== 'json' && options.export !== 'csv') {
    throw new ValidationError(
      `Formato de exportación no válido: ${String(options.export)}. Use 'json' o 'csv'.`,
    );
  }

  await withCommandContext(options, ({ library }) => {
    const items = library.list({
      ...(options.type === undefined ? {} : { type: options.type }),
      limit,
      offset,
      orderBy: sort,
      ...(options.favorites === true ? { favorites: true } : {}),
      ...(options.inProgress === true ? { inProgress: true } : {}),
      ...(options.unwatched === true ? { unwatched: true } : {}),
      ...(options.watched === true ? { watched: true } : {}),
      ...(options.tags && options.tags.length > 0 ? { tags: options.tags } : {}),
    });
    const total = library.count(options.type);

    if (options.json || options.export === 'json') {
      console.log(
        JSON.stringify(
          {
            total,
            count: items.length,
            offset,
            limit,
            sort,
            items: items.map(toMediaJson),
          },
          null,
          2,
        ),
      );
      return;
    }

    if (options.export === 'csv') {
      const lines = [mediaCsvHeader, ...items.map(toMediaCsv)];
      console.log(lines.join('\n'));
      return;
    }

    if (total === 0) {
      console.log(
        pc.dim('No hay títulos indexados. Ejecuta «mm index» para escanear tus carpetas.'),
      );
      return;
    }

    if (items.length === 0) {
      console.log(
        pc.dim(`No hay resultados en esta página (${total} títulos en total, offset ${offset}).`),
      );
      return;
    }

    const first = offset + 1;
    const last = offset + items.length;
    const filterParts = [];
    if (options.type) filterParts.push(`tipo: ${options.type}`);
    if (options.favorites) filterParts.push('favoritos');
    if (options.inProgress) filterParts.push('en curso');
    if (options.unwatched) filterParts.push('no vistos');
    if (options.watched) filterParts.push('vistos');
    if (options.tags && options.tags.length > 0) filterParts.push(`tag: ${options.tags.join(',')}`);
    const filterStr = filterParts.length > 0 ? pc.dim(` · ${filterParts.join(', ')}`) : '';
    console.log(
      pc.bold(`${total} títulos`) +
        pc.dim(` · mostrando ${first}-${last} · orden: ${sort}`) +
        filterStr,
    );
    printMediaList(items);
  });
}

/** Adapta los valores de Commander a los tipos del comando. */
export function parseListOptions(raw: {
  type?: string;
  sort?: string;
  limit?: number;
  offset?: number;
  favorites?: boolean;
  inProgress?: boolean;
  unwatched?: boolean;
  watched?: boolean;
  tags?: string[];
  export?: string;
}): Pick<
  ListCommandOptions,
  | 'type'
  | 'sort'
  | 'limit'
  | 'offset'
  | 'favorites'
  | 'inProgress'
  | 'unwatched'
  | 'watched'
  | 'tags'
  | 'export'
> {
  return {
    ...(raw.type === undefined ? {} : { type: parseMediaType(raw.type) }),
    ...(raw.sort === undefined ? {} : { sort: parseSortOrder(raw.sort) }),
    ...(raw.limit === undefined ? {} : { limit: raw.limit }),
    ...(raw.offset === undefined ? {} : { offset: raw.offset }),
    ...(raw.favorites === undefined ? {} : { favorites: raw.favorites }),
    ...(raw.inProgress === undefined ? {} : { inProgress: raw.inProgress }),
    ...(raw.unwatched === undefined ? {} : { unwatched: raw.unwatched }),
    ...(raw.watched === undefined ? {} : { watched: raw.watched }),
    ...(raw.tags === undefined ? {} : { tags: raw.tags.map(Number).filter((n) => !isNaN(n)) }),
    ...(raw.export === undefined ? {} : { export: raw.export as 'json' | 'csv' }),
  };
}
