import path from 'node:path';

import { classifyFile, type ClassifiedFile } from './classifier.js';
import { scanLibrary, type ScanLibraryOptions } from './scanner.js';
import { IndexError, formatError } from '../../shared/errors.js';
import { resolvePath } from '../../shared/paths.js';

import type { Logger } from '../../shared/logger.js';
import type { LibraryRoot, ScanOptions } from '../config/schema.js';
import type { FileRepository } from '../db/repositories/fileRepo.js';
import type { IndexRunRepository } from '../db/repositories/indexRunRepo.js';
import type { MediaRepository } from '../db/repositories/mediaRepo.js';
import type { MediaFileInput } from '../db/types.js';

/** Configuración de biblioteca que consume el indexador. */
export type IndexLibraryConfig = {
  roots: readonly LibraryRoot[];
  /** Puede ser un valor estático o un getter para recargar sin reiniciar (B3). */
  exclude: readonly string[] | (() => readonly string[]);
  scan: ScanOptions;
};

/** Repositorios que necesita la indexación, inyectados por el llamante (DIP). */
export type IndexingRepositories = {
  mediaRepo: MediaRepository;
  fileRepo: FileRepository;
  indexRunRepo: IndexRunRepository;
};

export type IndexLibraryOptions = {
  /** Si es `true`, calcula el resultado sin escribir en la base de datos. */
  dryRun?: boolean;
  /** Limita la indexación a la raíz configurada con esta ruta. */
  rootPath?: string;
  /** Limita la indexación a una subcarpeta concreta dentro de la(s) raíz(es). */
  folderPath?: string;
  /** Logger inyectado; los eventos detallados se emiten a nivel debug. */
  logger?: Logger;
};

/** Resultado de una ejecución de indexación. */
export type IndexStats = {
  dryRun: boolean;
  runId: number | null;
  rootsScanned: number;
  filesSeen: number;
  /** Ficheros nuevos o modificados (se insertan o actualizan). */
  filesAdded: number;
  filesUnchanged: number;
  filesRemoved: number;
  mediaAdded: number;
  mediaRemoved: number;
  /** Ficheros de vídeo que no se han podido clasificar. */
  skipped: number;
  warnings: string[];
  durationMs: number;
};

type FolderGroup = {
  sample: ClassifiedFile;
  changed: ClassifiedFile[];
};

/**
 * Indexa una biblioteca de forma incremental.
 *
 * Un fichero solo se escribe si es nuevo o si han cambiado su tamaño o su fecha
 * de modificación. Al indexar todas las raíces se eliminan los ficheros que ya no
 * existen en disco y los títulos que se quedan sin ficheros. Si se filtra por
 * `rootPath` nunca se borra nada: las demás raíces no se han recorrido.
 */
export async function indexLibrary(
  repositories: IndexingRepositories,
  library: IndexLibraryConfig,
  options: IndexLibraryOptions = {},
): Promise<IndexStats> {
  const startedAt = Date.now();
  const dryRun = options.dryRun ?? false;
  const roots = selectRoots(library.roots, options.rootPath);
  options.logger?.debug('index.start', {
    dryRun,
    roots: roots.length,
    rootPath: options.rootPath ?? null,
  });

  if (roots.length === 0) {
    throw new IndexError(
      'No hay ninguna raíz configurada. Añade al menos una entrada en library.roots.',
    );
  }

  const stats: IndexStats = {
    dryRun,
    runId: null,
    rootsScanned: 0,
    filesSeen: 0,
    filesAdded: 0,
    filesUnchanged: 0,
    filesRemoved: 0,
    mediaAdded: 0,
    mediaRemoved: 0,
    skipped: 0,
    warnings: [],
    durationMs: 0,
  };

  if (!dryRun) {
    stats.runId = repositories.indexRunRepo.start();
  }

  try {
    let rootsToScan = roots;
    let subfolder: string | undefined;
    if (options.folderPath !== undefined) {
      const target = path.resolve(options.folderPath);
      const containingRoot = roots.find((r) => {
        const resolved = path.resolve(r.path);
        return target === resolved || target.startsWith(resolved + path.sep);
      });
      if (containingRoot) {
        rootsToScan = [containingRoot];
        subfolder = options.folderPath;
      } else {
        throw new IndexError(
          `La carpeta "${options.folderPath}" no está dentro de ninguna raíz de biblioteca configurada.`,
        );
      }
    }

    const classified = await scanAndClassify(
      rootsToScan,
      library,
      stats,
      options.logger,
      subfolder,
    );

    const knownPaths = new Set(classified.keys());
    const folders = groupByFolder(repositories, classified, stats);

    if (dryRun) {
      stats.mediaAdded = [...folders.keys()].filter(
        (folderPath) => repositories.mediaRepo.findByFolderPath(folderPath) === undefined,
      ).length;
      if (options.rootPath === undefined && subfolder === undefined) {
        countWouldBeRemoved(repositories, folders, knownPaths, stats);
      }
    } else {
      writeGroups(repositories, folders, stats);
      if (options.rootPath === undefined && subfolder === undefined) {
        prune(repositories, knownPaths, stats);
      }
    }

    stats.durationMs = Date.now() - startedAt;

    if (stats.runId !== null) {
      repositories.indexRunRepo.complete(stats.runId, {
        filesSeen: stats.filesSeen,
        filesAdded: stats.filesAdded,
        filesRemoved: stats.filesRemoved,
      });
    }

    options.logger?.debug('index.completed', { ...stats });
    return stats;
  } catch (error) {
    stats.durationMs = Date.now() - startedAt;
    options.logger?.debug('index.failed', {
      durationMs: stats.durationMs,
      filesSeen: stats.filesSeen,
      error,
    });
    if (stats.runId !== null) {
      repositories.indexRunRepo.fail(stats.runId, {
        filesSeen: stats.filesSeen,
        filesAdded: stats.filesAdded,
        filesRemoved: stats.filesRemoved,
      });
    }
    if (error instanceof IndexError) throw error;
    throw new IndexError(`Falló la indexación: ${formatError(error)}`, { cause: error });
  }
}

async function scanAndClassify(
  roots: readonly LibraryRoot[],
  library: IndexLibraryConfig,
  stats: IndexStats,
  logger: Logger | undefined,
  subfolder?: string,
): Promise<Map<string, ClassifiedFile>> {
  const classified = new Map<string, ClassifiedFile>();

  for (const root of roots) {
    const rootSubfolder = subfolder !== undefined ? path.relative(root.path, subfolder) : undefined;
    const scanOptions: ScanLibraryOptions = {
      maxDepth: library.scan.maxDepth,
      followSymlinks: library.scan.followSymlinks,
      minSizeBytes: library.scan.minSizeBytes,
      exclude: typeof library.exclude === 'function' ? library.exclude() : library.exclude,
    };
    if (rootSubfolder !== undefined) {
      scanOptions.subfolder = rootSubfolder;
    }
    const files = await scanLibrary(root.path, scanOptions);

    stats.rootsScanned += 1;
    stats.filesSeen += files.length;
    logger?.debug('index.root.scanned', {
      root: root.path,
      type: root.type,
      files: files.length,
    });

    for (const file of files) {
      const entry = classifyFile({ root, ...file });
      if (!entry) {
        stats.skipped += 1;
        continue;
      }

      if (classified.has(entry.filePath)) {
        stats.warnings.push(`${entry.filePath} aparece en más de una raíz; se conserva la primera`);
        continue;
      }

      classified.set(entry.filePath, entry);
    }
  }

  logger?.debug('index.classified', {
    files: classified.size,
    skipped: stats.skipped,
    warnings: stats.warnings.length,
  });
  return classified;
}

function groupByFolder(
  repositories: IndexingRepositories,
  classified: Map<string, ClassifiedFile>,
  stats: IndexStats,
): Map<string, FolderGroup> {
  const folders = new Map<string, FolderGroup>();

  for (const entry of classified.values()) {
    const existing = repositories.fileRepo.findByPath(entry.filePath);
    const changed = existing?.sizeBytes !== entry.sizeBytes || existing?.mtimeMs !== entry.mtimeMs;

    if (changed) {
      stats.filesAdded += 1;
    } else {
      stats.filesUnchanged += 1;
    }

    const group = folders.get(entry.folderPath) ?? { sample: entry, changed: [] };
    if (changed) group.changed.push(entry);
    folders.set(entry.folderPath, group);
  }

  return folders;
}

function writeGroups(
  repositories: IndexingRepositories,
  folders: Map<string, FolderGroup>,
  stats: IndexStats,
): void {
  for (const [folderPath, group] of folders) {
    if (group.changed.length === 0) continue;

    const isNew = repositories.mediaRepo.findByFolderPath(folderPath) === undefined;
    const media = repositories.mediaRepo.upsert({
      type: group.sample.type,
      title: group.sample.title,
      year: group.sample.year,
      folderPath,
    });

    if (isNew) stats.mediaAdded += 1;

    const files: MediaFileInput[] = group.changed.map((entry) => ({
      mediaId: media.id,
      season: entry.season,
      episode: entry.episode,
      filePath: entry.filePath,
      sizeBytes: entry.sizeBytes,
      mtimeMs: entry.mtimeMs,
    }));
    repositories.fileRepo.upsertMany(files);
  }
}

function prune(
  repositories: IndexingRepositories,
  knownPaths: ReadonlySet<string>,
  stats: IndexStats,
): void {
  stats.filesRemoved = repositories.fileRepo.removeMissing(knownPaths);
  // `removeEmpty` borra también títulos que ya estuvieran vacíos de antes; es
  // idempotente y mantiene la biblioteca coherente.
  stats.mediaRemoved = repositories.mediaRepo.removeEmpty();
}

function countWouldBeRemoved(
  repositories: IndexingRepositories,
  folders: Map<string, FolderGroup>,
  knownPaths: ReadonlySet<string>,
  stats: IndexStats,
): void {
  stats.filesRemoved = repositories.fileRepo
    .listAllPaths()
    .filter((filePath) => !knownPaths.has(filePath)).length;

  const knownFolders = new Set(folders.keys());
  stats.mediaRemoved = repositories.mediaRepo
    .listFolders()
    .filter((folderPath) => !knownFolders.has(folderPath)).length;
}

function selectRoots(roots: readonly LibraryRoot[], rootPath: string | undefined): LibraryRoot[] {
  if (rootPath === undefined) return [...roots];

  const target = resolvePath(rootPath);
  const selected = roots.filter((root) => {
    // Se comparan rutas normalizadas para aceptar `~`, rutas relativas y barras finales.
    return path.normalize(resolvePath(root.path)) === path.normalize(target);
  });

  if (selected.length === 0) {
    throw new IndexError(`La raíz ${target} no está configurada en library.roots.`);
  }

  return selected;
}
