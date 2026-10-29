import { createLibraryService, type LibraryRepositories, type LibraryService } from './service.js';
import { closeDatabase, openDatabase } from '../db/connection.js';
import { createDatabaseMaintenance, type DatabaseMaintenance } from '../db/maintenance.js';
import { createFileRepository } from '../db/repositories/fileRepo.js';
import { createIndexRunRepository } from '../db/repositories/indexRunRepo.js';
import { createMediaRepository } from '../db/repositories/mediaRepo.js';
import {
  createMetadataRepository,
  type MetadataRepository,
} from '../db/repositories/metadataRepo.js';
import {
  createPlaybackRepository,
  type PlaybackRepository,
} from '../db/repositories/playbackRepo.js';
import {
  createPreferenceRepository,
  type PreferenceRepository,
} from '../db/repositories/preferencesRepo.js';
import { createTagRepository, type TagRepository } from '../db/repositories/tagRepo.js';
import { MetadataService } from '../metadata/service.js';

import type { Logger } from '../../shared/logger.js';
import type { AppConfig, MetadataConfig } from '../config/schema.js';

/** Repositorios extendidos que incluyen reproducción, preferencias, etiquetas y metadatos. */
export type ExtendedLibraryRepositories = LibraryRepositories & {
  playbackRepo: PlaybackRepository;
  preferenceRepo: PreferenceRepository;
  tagRepo: TagRepository;
  metadataRepo: MetadataRepository;
  metadataService: MetadataService;
  metadataConfig: MetadataConfig;
};

/** Biblioteca abierta y lista para consultar, con su ciclo de vida. */
export type OpenedLibrary = {
  repositories: ExtendedLibraryRepositories;
  library: LibraryService;
  maintenance: DatabaseMaintenance;
  playbackRepo: PlaybackRepository;
  preferenceRepo: PreferenceRepository;
  tagRepo: TagRepository;
  metadataRepo: MetadataRepository;
  metadataService: MetadataService;
  metadataConfig: MetadataConfig;
  close: () => void;
};

/**
 * Abre la base de datos y construye la capa de consulta.
 *
 * Lo usan tanto la CLI como la TUI para no duplicar el cableado de repositorios.
 * El llamante es responsable de invocar `close()` al terminar.
 *
 * Si se proporciona `config` y `logger`, se crea un `MetadataService` con las
 * claves API de `config.metadata` para permitir la sincronización automática
 * de metadatos externos.
 */
export function openLibrary(
  databasePath: string,
  config?: AppConfig,
  logger?: Logger,
): OpenedLibrary {
  const db = openDatabase(databasePath);
  const fileRepo = createFileRepository(db);
  const indexRunRepo = createIndexRunRepository(db);
  const playbackRepo = createPlaybackRepository(db);
  const preferenceRepo = createPreferenceRepository(db);
  const tagRepo = createTagRepository(db);
  const metadataRepo = createMetadataRepository(db);
  const mediaRepo = createMediaRepository(db, {
    preferenceRepo,
    playbackRepo,
    tagRepo,
  });

  // Crear MetadataService con proveedores públicos gratuitos (sin API keys).
  let metadataService: MetadataService;
  let metadataConfig: MetadataConfig = { cacheTtlHours: 24, autoFetch: false };

  if (config && logger) {
    metadataService = new MetadataService({
      logger,
    });
    metadataConfig = config.metadata ?? { cacheTtlHours: 24 };
  } else {
    metadataService = new MetadataService({
      logger: {
        debug: () => {
          void 0;
        },
        warn: () => {
          void 0;
        },
        error: () => {
          void 0;
        },
      },
    });
  }

  const repositories: ExtendedLibraryRepositories = {
    mediaRepo,
    fileRepo,
    indexRunRepo,
    playbackRepo,
    preferenceRepo,
    tagRepo,
    metadataRepo,
    metadataService,
    metadataConfig,
    ...(logger ? { logger } : {}),
  };

  return {
    repositories,
    library: createLibraryService(repositories),
    maintenance: createDatabaseMaintenance(db),
    playbackRepo,
    preferenceRepo,
    tagRepo,
    metadataRepo,
    metadataService,
    metadataConfig,
    close: () => closeDatabase(db),
  };
}
