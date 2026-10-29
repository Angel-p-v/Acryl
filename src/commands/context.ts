import { statSync } from 'node:fs';

import { loadConfig, resolveConfigPath, watchConfigFile } from '../core/config/loader.js';
import { openLibrary } from '../core/library/open.js';
import { createLogger, type Logger } from '../shared/logger.js';

import type { AppConfig, PlaybackConfig } from '../core/config/schema.js';
import type { DatabaseMaintenance } from '../core/db/maintenance.js';
import type { PlaybackRepository } from '../core/db/repositories/playbackRepo.js';
import type { PreferenceRepository } from '../core/db/repositories/preferencesRepo.js';
import type { LibraryRepositories, LibraryService } from '../core/library/service.js';

/** Opciones comunes a todos los subcomandos que necesitan configuración y base de datos. */
export type CommandOptions = {
  config?: string;
  /** Fuerza `logging.level=debug`; lo inyecta el flag global `--verbose`. */
  verbose?: boolean;
  /** Ruta de archivo para escribir los logs (con rotación automática). */
  logFile?: string;
};

/** Referencia mutable para recargar `library.exclude` sin reiniciar (B3). */
export type ExcludeRef = {
  exclude: string[];
};

/** Dependencias que recibe un subcomando ya conectado a la biblioteca. */
export type CommandContext = {
  config: AppConfig;
  excludeRef: ExcludeRef;
  repositories: LibraryRepositories;
  library: LibraryService;
  maintenance: DatabaseMaintenance;
  playbackRepo: PlaybackRepository;
  preferenceRepo: PreferenceRepository;
  playbackConfig: PlaybackConfig;
  logger: Logger;
};

/**
 * Carga la configuración, abre la base de datos y garantiza que se cierra al terminar.
 *
 * Centralizar este ciclo evita que cada subcomando repita la misma secuencia y que
 * alguno se olvide de cerrar la conexión si una consulta falla. El logger se crea
 * aquí para que todos los subcomandos compartan nivel y formato.
 *
 * Además configura un watcher del archivo de configuración para recargar
 * `library.exclude` sin reiniciar (B3).
 */
export async function withCommandContext<T>(
  options: CommandOptions,
  run: (context: CommandContext) => T | Promise<T>,
): Promise<T> {
  const configPath = resolveConfigPath(
    options.config === undefined ? {} : { configPath: options.config },
  );
  const config = await loadConfig({ configPath });
  const logger = createLogger({
    level: options.verbose === true ? 'debug' : config.logging.level,
    format: config.logging.format,
    ...(options.logFile !== undefined ? { logFile: options.logFile } : {}),
  });
  logger.debug('config.loaded', {
    database: config.database.path,
    roots: config.library.roots.length,
    player: config.player.default,
  });

  // Ref mutable para recargar exclude sin reiniciar (B3)
  const excludeRef: ExcludeRef = { exclude: [...config.library.exclude] };

  // Vigilar cambios en el archivo de config para recargar exclude
  const configWatcher = watchConfigFile(configPath, (newConfig) => {
    excludeRef.exclude = [...newConfig.library.exclude];
    logger.debug('config.exclude.reloaded', {
      exclude: excludeRef.exclude,
    });
  });

  // Validar que las raíces de biblioteca existen; advertir si alguna falta
  for (const root of config.library.roots) {
    try {
      const stat = statSync(root.path);
      if (!stat.isDirectory()) {
        logger.warn('library.root.no-existe', {
          root: root.path,
          type: root.type,
          motivo: 'no es un directorio',
        });
      }
    } catch {
      logger.warn('library.root.no-existe', {
        root: root.path,
        type: root.type,
        motivo: 'no se pudo acceder',
      });
    }
  }

  const opened = openLibrary(config.database.path, config, logger);
  logger.debug('db.opened', { path: config.database.path });

  try {
    return await run({
      config,
      excludeRef,
      repositories: opened.repositories,
      library: opened.library,
      maintenance: opened.maintenance,
      playbackRepo: opened.playbackRepo,
      preferenceRepo: opened.preferenceRepo,
      playbackConfig: config.playback,
      logger,
    });
  } finally {
    configWatcher.close();
    opened.close();
    logger.debug('db.closed');
  }
}
