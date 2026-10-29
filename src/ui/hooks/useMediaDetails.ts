import { useMemo } from 'react';

import type { LibraryService, MediaDetails } from '../../core/library/service.js';

/** Carga el detalle del título seleccionado, si hay uno. */
export function useMediaDetails(
  service: LibraryService,
  mediaId: number | undefined,
): MediaDetails | null {
  return useMemo(() => {
    if (mediaId === undefined) return null;

    try {
      return service.find(String(mediaId));
    } catch {
      // El panel de detalle es accesorio: si el título desaparece entre consultas,
      // la lista sigue siendo usable.
      return null;
    }
  }, [service, mediaId]);
}
