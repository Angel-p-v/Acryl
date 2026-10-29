import { Box, Text } from 'ink';
import React from 'react';

import { theme } from '../theme.js';

import type { ViewName } from '../../ui/types.js';
import type { Keybindings } from '../hooks/useConfig.js';

export type StatusBarProps = {
  bindings: Keybindings;
  /** Vista actual de la TUI (para mostrar los atajos relevantes de cada una). */
  view?: ViewName;
  message?: string | null;
};

/** Atajos globales que siempre están disponibles (salvo en vistas modales). */
const GLOBAL_HINTS: [string, string][] = [
  ['quit', 'salir'],
  ['help', 'ayuda'],
];

/** Atajos principales de la biblioteca. */
const LIBRARY_HINTS: [string, string][] = [
  ['nextSection', 'sección'],
  ['search', 'buscar'],
  ['settings', 'ajustes'],
  ['play', 'reproducir'],
  ['enter', 'detalles'],
];

/** Atajos de la vista de búsqueda. */
const SEARCH_HINTS: [string, string][] = [['escape', 'volver']];

/** Atajos de la vista de ajustes. */
const SETTINGS_HINTS: [string, string][] = [
  ['escape', 'volver'],
  ['tab', 'biblioteca'],
];

/** Atajos de la vista de detalles de reproducción. */
const PLAY_DETAILS_HINTS: [string, string][] = [
  ['p', 'reproducir'],
  ['escape', 'volver'],
  ['←→', 'audio'],
  ['Mayús+←→', 'subtítulos'],
];

/** Resuelve la etiqueta legible de una tecla (incluye combinaciones con Mayús). */
function keyLabel(key: string | undefined): string {
  if (!key) return '—';
  return key
    .replace('return', 'Enter')
    .replace('escape', 'Esc')
    .replace('tab', 'Tab')
    .replace('up', '↑')
    .replace('down', '↓')
    .replace('left', '←')
    .replace('right', '→');
}

/** Barra inferior con los atajos principales o el último mensaje de estado. */
export function StatusBar({ bindings, view, message }: StatusBarProps): React.JSX.Element {
  if (message !== undefined && message !== null && message.length > 0) {
    return (
      <Box paddingX={1}>
        <Text>{theme.warning(message)}</Text>
      </Box>
    );
  }

  // Elige los atajos según la vista actual.
  let hints: [string, string][];
  switch (view) {
    case 'search':
      hints = [...GLOBAL_HINTS, ...SEARCH_HINTS];
      break;
    case 'settings':
      hints = SETTINGS_HINTS;
      break;
    case 'play-details':
      hints = PLAY_DETAILS_HINTS;
      break;
    case 'library':
    default:
      hints = [...GLOBAL_HINTS, ...LIBRARY_HINTS];
      break;
  }

  return (
    <Box paddingX={1} justifyContent="space-between">
      <Text>
        {hints.map(([action, label], index) => {
          let key = bindings[action];
          // Sobrescribir teclas fijas que no están en los keybindings (flechas, Mayús, etc.).
          if (action === '←→') key = '←→';
          if (action === 'Mayús+←→') key = 'Mayús+←→';
          if (key === undefined || key.length === 0) return null;
          return (
            <Text key={action}>
              {index > 0 ? theme.muted(' · ') : ''}
              {theme.primary(`[${keyLabel(key)}]`)} {theme.muted(label)}
            </Text>
          );
        })}
      </Text>
    </Box>
  );
}
