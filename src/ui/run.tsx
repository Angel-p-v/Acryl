import { homedir } from 'node:os';
import path from 'node:path';

import { render } from 'ink';
import React from 'react';

import { App } from './App.js';
import { enterAlternateScreen, leaveAlternateScreen } from './screen.js';
import { indexLibrary } from '../core/indexer/service.js';
import { openLibrary } from '../core/library/open.js';
import { createLogger } from '../shared/logger.js';

import type { AppConfig } from '../core/config/schema.js';

/** Directorio de datos de la aplicación (p. ej. `~/.local/share/media-manager`). */
function appDataDir(): string {
  return path.join(homedir(), '.local', 'share', 'media-manager');
}

/** Ruta por defecto del fichero de log cuando se activa `--verbose`. */
export function defaultLogFile(): string {
  return path.join(appDataDir(), 'app.log');
}

/** Arranca la interfaz de terminal y espera a que termine. */
export async function runTui(
  config: AppConfig,
  configPath?: string,
  options: { verbose?: boolean; logFile?: string } = {},
): Promise<void> {
  const { verbose, logFile } = options;
  const logger = createLogger({
    level: verbose === true ? 'debug' : config.logging.level,
    format: config.logging.format,
    ...(logFile !== undefined ? { logFile } : {}),
  });
  const opened = openLibrary(config.database.path, config, logger);

  /** Reindexa la biblioteca usando las raíces y filtros de la configuración
   *  pasada por App (siempre la config más actualizada tras guardar ajustes). */
  const indexLibraryNow = async (newConfig: AppConfig): Promise<void> => {
    await indexLibrary(
      opened.repositories,
      {
        roots: newConfig.library.roots,
        exclude: newConfig.library.exclude,
        scan: newConfig.library.scan,
      },
      { logger },
    );
  };

  enterAlternateScreen();
  try {
    const instance = render(
      <App
        config={config}
        library={opened.library}
        {...(configPath ? { configPath } : {})}
        onIndex={indexLibraryNow}
      />,
      { exitOnCtrlC: false },
    );
    await instance.waitUntilExit();
  } finally {
    leaveAlternateScreen();
    opened.close();
  }
}
