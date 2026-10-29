import { Box, Text } from 'ink';
import React from 'react';

import { theme } from '../theme.js';

import type { MediaRow } from '../../core/db/types.js';

export type MediaListProps = {
  items: readonly MediaRow[];
  selected: number;
  emptyMessage: string;
  title?: string;
  /** Modo compacto: más filas por pantalla, menos detalle. */
  compact: boolean | undefined;
  /** IDs de los títulos seleccionados (selección múltiple). */
  selectedIds: ReadonlySet<number> | undefined;
  /** Callback al alternar la selección de un título. */
  _onToggleSelection: ((mediaId: number) => void) | undefined;
};
const WINDOW_SIZE = 12;
const COMPACT_WINDOW_SIZE = 24;

/** Lista de títulos con una ventana alrededor de la selección. */
export function MediaList({
  items,
  selected,
  emptyMessage,
  title = 'Títulos',
  compact = false,
  selectedIds,
  _onToggleSelection,
}: MediaListProps): React.JSX.Element {
  const windowSize: number = compact ? COMPACT_WINDOW_SIZE : WINDOW_SIZE;
  const start = Math.max(
    0,
    Math.min(selected - Math.floor(windowSize / 2), Math.max(0, items.length - windowSize)),
  );
  const visible = items.slice(start, start + windowSize);

  return (
    <Box flexDirection="column" flexGrow={1} paddingX={1}>
      <Text>
        {theme.label(title)} {theme.muted(`(${items.length})`)}
      </Text>
      {items.length === 0 ? (
        <Text>{theme.muted(emptyMessage)}</Text>
      ) : (
        visible.map((media, index) => {
          const position = start + index;
          const active = position === selected;
          const isSelected = selectedIds?.has(media.id) ?? false;
          const year = media.year === null ? '' : ` (${media.year})`;
          const marker = isSelected ? '✓' : active ? '▸' : ' ';
          return (
            <Text
              key={media.id}
              {...(active ? { color: 'cyan' as const } : {})}
              bold={active || isSelected}
            >
              {`${marker} `}
              {media.title}
              {year} {theme.muted(`[${media.type}]`)}
            </Text>
          );
        })
      )}
    </Box>
  );
}
