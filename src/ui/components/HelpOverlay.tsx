import { Box, Text } from 'ink';
import React from 'react';

import { theme } from '../theme.js';

import type { Keybindings } from '../hooks/useConfig.js';

export type HelpOverlayProps = {
  bindings: Keybindings;
};

const ACTIONS: readonly { key: string; label: string }[] = [
  { key: 'quit', label: 'salir de la aplicación' },
  { key: 'nextSection', label: 'siguiente sección' },
  { key: 'search', label: 'buscar títulos' },
  { key: 'settings', label: 'ver la configuración' },
  { key: 'up', label: 'título anterior' },
  { key: 'down', label: 'título siguiente' },
  { key: 'play', label: 'reproducir (M6)' },
  { key: 'refreshMetadata', label: 'refrescar metadatos' },
  { key: 'refreshAll', label: 'refrescar todos los metadatos' },
  { key: 'gotoTaggedMedia', label: 'siguiente con metadatos' },
  { key: 'escape', label: 'cerrar o volver atrás' },
];

/** Ayuda contextual con los atajos efectivos de la configuración. */
export function HelpOverlay({ bindings }: HelpOverlayProps): React.JSX.Element {
  return (
    <Box flexDirection="column" borderStyle="double" borderColor="cyan" paddingX={1} marginTop={1}>
      <Text>{theme.highlight('Atajos de teclado')}</Text>
      {ACTIONS.map(({ key, label }) => {
        const binding = bindings[key];
        if (binding === undefined || binding.length === 0) return null;
        return (
          <Text key={key}>
            {'  '}
            {theme.primary(binding.padEnd(8))} {theme.muted(label)}
          </Text>
        );
      })}
      <Text>{theme.muted('  Pulsa ? o Esc para cerrar la ayuda.')}</Text>
    </Box>
  );
}
