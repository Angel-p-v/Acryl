import { Box, Text } from 'ink';
import React from 'react';

import { theme } from '../theme.js';

import type { Counts, Section } from '../types.js';

export type SidebarProps = {
  section: Section;
  counts: Counts;
};

const SECTIONS: readonly { key: Section; label: string }[] = [
  { key: 'all', label: 'Todo' },
  { key: 'in-progress', label: 'Continuar' },
  { key: 'favorites', label: 'Favoritos' },
  { key: 'series', label: 'Series' },
  { key: 'anime', label: 'Anime' },
  { key: 'movie', label: 'Películas' },
];

/** Navegación por tipo de contenido con el número de títulos de cada sección. */
export function Sidebar({ section, counts }: SidebarProps): React.JSX.Element {
  return (
    <Box
      flexDirection="column"
      width={20}
      borderStyle="round"
      borderColor={theme.border}
      paddingX={1}
    >
      <Text>{theme.label('Biblioteca')}</Text>
      {SECTIONS.map(({ key, label }) => {
        const active = key === section;
        return (
          <Text key={key} {...(active ? { color: 'cyan' as const } : {})} bold={active}>
            {active ? '▸ ' : '  '}
            {label} {theme.muted(`(${counts[key]})`)}
          </Text>
        );
      })}
    </Box>
  );
}
