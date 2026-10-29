import { defaultPlayers, metadataConfigSchema, playbackConfigSchema } from './schema.js';

import type { AppConfig } from './schema.js';

/**
 * Valores por defecto de la aplicación. Se fusionan con el archivo YAML
 * antes de validarlo, de modo que un archivo parcial sigue siendo válido.
 */
export const defaultConfig: AppConfig = {
  version: 1,
  library: {
    roots: [],
    exclude: ['**/*.sample.*', '**/sample/**', '**/.hidden/**', '**/@eaDir/**'],
    scan: {
      followSymlinks: false,
      maxDepth: 4,
      minSizeBytes: 0,
    },
  },
  database: {
    path: '~/.local/share/media-manager/library.db',
  },
  player: {
    default: 'mpv',
    onLaunch: 'detach',
    players: defaultPlayers,
  },
  ui: {
    theme: 'default',
    layout: 'split',
    sort: 'title',
    showDetails: true,
    compact: false,
  },
  playback: playbackConfigSchema.parse({}),
  metadata: metadataConfigSchema.parse({}),
  logging: {
    level: 'warn',
    format: 'text',
  },
  keybindings: {
    quit: 'q',
    help: '?',
    search: '/',
    play: 'p',
    refresh: 'r',
    refreshAll: 'R',
    goToDetails: 'm',
    nextSection: 'tab',
    settings: 's',
    up: 'up',
    down: 'down',
    left: 'left',
    right: 'right',
    enter: 'return',
    escape: 'escape',
    /** Abre la vista de reasignación de atajos de teclado desde la TUI. */
    configureKeybindings: 'k',
  },
};
