import { Box } from 'ink';
import React from 'react';

import { MediaDetails } from '../components/MediaDetails.js';
import { MediaList } from '../components/MediaList.js';
import { Sidebar } from '../components/Sidebar.js';

import type { MediaRow } from '../../core/db/types.js';
import type { MediaDetails as Details, LibraryService } from '../../core/library/service.js';
import type { Counts, Section } from '../types.js';

export type LibraryViewProps = {
  items: readonly MediaRow[];
  selected: number;
  section: Section;
  counts: Counts;
  details: Details | null;
  showDetails: boolean;
  library: LibraryService;
  /** Modo compacto: más filas por pantalla. */
  compact?: boolean;
  /** Clave de refresco de metadatos: se incrementa tras obtener metadatos externos. */
  metadataRefreshKey: number;
  /** IDs de títulos seleccionados (selección múltiple). */
  selectedIds?: ReadonlySet<number>;
  /** Callback al alternar selección de un título. */
  _onToggleSelection?: (mediaId: number) => void;
};

/** Vista principal: navegación por secciones, listado y panel de detalle. */
export function LibraryView({
  items,
  selected,
  section,
  counts,
  details,
  showDetails,
  library,
  compact = false,
  metadataRefreshKey,
  selectedIds,
  _onToggleSelection,
}: LibraryViewProps): React.JSX.Element {
  return (
    <Box flexGrow={1} flexDirection="row">
      <Sidebar section={section} counts={counts} />
      <Box flexDirection="column" flexGrow={1}>
        <MediaList
          items={items}
          selected={selected}
          emptyMessage="No hay títulos en esta sección. Ejecuta «mm index» para escanear tus carpetas."
          compact={compact}
          selectedIds={selectedIds}
          _onToggleSelection={_onToggleSelection}
        />
        {showDetails ? (
          <MediaDetails
            details={details}
            library={library}
            metadataRefreshKey={metadataRefreshKey}
          />
        ) : null}
      </Box>
    </Box>
  );
}
