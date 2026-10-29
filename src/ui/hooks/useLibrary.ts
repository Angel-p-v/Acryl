import { useMemo } from 'react';

import { formatError } from '../../shared/errors.js';

import type { MediaType } from '../../core/config/schema.js';
import type { MediaRow } from '../../core/db/types.js';
import type { LibraryService } from '../../core/library/service.js';

export type UseLibraryOptions = {
  type?: MediaType;
  query?: string;
  limit?: number;
  /** Sección especial: 'in-progress' para títulos en curso, 'favorites' para favoritos. */
  section?: 'in-progress' | 'favorites' | undefined;
  /** Versión de datos: al incrementarla se fuerza una re-lectura de la BD (útil tras indexar). */
  dataVersion?: number;
};

export type UseLibraryResult = {
  items: MediaRow[];
  total: number;
  error: string | null;
};

const DEFAULT_LIMIT = 200;

/**
 * Consulta la biblioteca para la vista actual.
 *
 * SQLite es síncrono, así que no hay estados de carga: el resultado se recalcula
 * en el mismo render cuando cambian la sección o la búsqueda. El límite evita
 * renderizar miles de filas de golpe.
 */
export function useLibrary(
  service: LibraryService,
  options: UseLibraryOptions = {},
): UseLibraryResult {
  const { type, query, limit = DEFAULT_LIMIT, section, dataVersion = 0 } = options;

  return useMemo(() => {
    try {
      const trimmed = query?.trim() ?? '';
      if (query !== undefined) {
        const items =
          trimmed.length === 0
            ? []
            : service.search(trimmed, {
                limit,
                ...(type === undefined ? {} : { type }),
              });
        return { items, total: items.length, error: null };
      }

      if (section === 'in-progress') {
        const items = service.listInProgress(limit);
        return { items, total: items.length, error: null };
      }

      if (section === 'favorites') {
        const items = service.list({
          limit,
          favorites: true,
        });
        return { items, total: items.length, error: null };
      }

      const items = service.list({
        limit,
        ...(type === undefined ? {} : { type }),
      });
      return { items, total: service.count(type), error: null };
    } catch (error) {
      return { items: [], total: 0, error: formatError(error) };
    }
  }, [service, type, query, limit, section, dataVersion]);
}
