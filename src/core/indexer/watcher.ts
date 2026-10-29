import path from 'node:path';

import { watch } from 'chokidar';

import { indexLibrary } from './service.js';
import { isVideoFile } from './video.js';

import type { IndexLibraryConfig, IndexingRepositories, IndexStats } from './service.js';
import type { Logger } from '../../shared/logger.js';

/** Repositorios extendidos con métodos de actualización para rename/move. */
export type WatchLibraryRepositories = IndexingRepositories & {
  mediaRepo: IndexingRepositories['mediaRepo'] & {
    updateFolderPath(oldPath: string, newPath: string): boolean;
  };
  fileRepo: IndexingRepositories['fileRepo'] & {
    updateFilePath(oldPath: string, newPath: string): boolean;
  };
};

export type WatchLibraryOptions = {
  repositories: WatchLibraryRepositories;
  library: IndexLibraryConfig;
  /** Tiempo de espera para agrupar ráfagas de eventos (copias, renombrados...). */
  debounceMs?: number;
  logger?: Logger;
  onIndexed?: (stats: IndexStats) => void;
  onError?: (error: unknown) => void;
};

/** Vigilancia activa de las raíces de la biblioteca. */
export type LibraryWatcher = {
  /** Se resuelve cuando chokidar ha terminado el escaneo inicial. */
  ready: Promise<void>;
  close: () => Promise<void>;
};

const DEFAULT_DEBOUNCE_MS = 400;

/**
 * Reindexa la biblioteca cuando cambia un fichero de vídeo.
 *
 * Los eventos se agrupan con un debounce y se ejecuta `indexLibrary`, que ya es
 * incremental: los ficheros sin cambios no se escriben. La reindexación es
 * completa para que las eliminaciones también se detecten; filtrar por raíz
 * desactivaría el borrado de huérfanos en el indexador.
 *
 * Detecta renombrados/movimientos (pares unlink+add en la misma carpeta o
 * distinta) y actualiza las rutas en la BD en lugar de reindexar todo.
 */
export function watchLibrary(options: WatchLibraryOptions): LibraryWatcher {
  const debounceMs = options.debounceMs ?? DEFAULT_DEBOUNCE_MS;
  const watcher = watch(
    options.library.roots.map((root) => root.path),
    {
      ignoreInitial: true,
      persistent: true,
      awaitWriteFinish: { stabilityThreshold: 300, pollInterval: 50 },
    },
  );

  let closed = false;
  let timer: NodeJS.Timeout | undefined;
  let running: Promise<void> | null = null;
  let pending = false;
  /** Rutas eliminadas pendientes de ser emparejadas con un 'add' (posibles renames). */
  const pendingUnlinks = new Map<string, string>(); // oldPath -> directory

  const run = (): void => {
    if (closed || running !== null) {
      pending = !closed;
      return;
    }

    running = (async (): Promise<void> => {
      try {
        options.logger?.debug('watch.index.start');
        const stats = await indexLibrary(options.repositories, options.library, {
          ...(options.logger === undefined ? {} : { logger: options.logger }),
        });
        options.logger?.debug('watch.index.completed', { ...stats });
        options.onIndexed?.(stats);
      } catch (error) {
        options.logger?.debug('watch.index.failed', { error });
        options.onError?.(error);
      } finally {
        running = null;
        if (pending && !closed) {
          pending = false;
          schedule();
        }
      }
    })();
  };

  function schedule(): void {
    if (closed) return;
    if (timer !== undefined) clearTimeout(timer);
    timer = setTimeout(() => {
      timer = undefined;
      run();
    }, debounceMs);
  }

  /**
   * Intenta detectar y procesar un rename/move.
   * Retorna true si se procesó como rename/move, false si hay que reindexar.
   */
  const tryHandleRename = (oldPath: string, newPath: string): boolean => {
    // Verificar que el archivo viejo existía en la BD
    const oldFile = options.repositories.fileRepo.findByPath(oldPath);
    if (!oldFile) return false;

    // Verificar que el nuevo path NO existe ya en la BD (evitar colisiones)
    const newFile = options.repositories.fileRepo.findByPath(newPath);
    if (newFile) return false;

    // Actualizar la ruta del fichero
    const fileUpdated = options.repositories.fileRepo.updateFilePath(oldPath, newPath);
    if (!fileUpdated) return false;

    // Si el directorio cambió, también actualizar la carpeta en media
    const oldDir = path.dirname(oldPath);
    const newDir = path.dirname(newPath);
    if (oldDir !== newDir) {
      // Actualizar folder_path en media para todos los archivos de esa carpeta
      options.repositories.mediaRepo.updateFolderPath(oldDir, newDir);
    }

    options.logger?.debug('watch.rename.handled', { oldPath, newPath });
    return true;
  };

  const onEvent = (event: string, filePath: string): void => {
    if (closed) return;

    if (event === 'unlinkDir') {
      options.logger?.debug('watch.event', { event, path: filePath });
      schedule();
      return;
    }

    // Ignorar eventos de no-vídeo (salvo unlink para detectar renames)
    const isVideo = isVideoFile(filePath);
    if (!isVideo && event !== 'unlink') return;

    if (event === 'unlink') {
      // Guardar como posible rename; se emparejará con un 'add' posterior
      pendingUnlinks.set(filePath, path.dirname(filePath));
      options.logger?.debug('watch.event', { event, path: filePath });
      schedule();
      return;
    }

    if (event === 'add') {
      // Intentar emparejar con un unlink previo (rename/move)
      // Buscamos en pendingUnlinks el unlink más reciente que coincida
      if (pendingUnlinks.size > 0) {
        // Tomar el último unlink pendiente (el más reciente)
        const entries = [...pendingUnlinks.entries()];
        const lastEntry = entries[entries.length - 1];
        if (lastEntry !== undefined) {
          const oldPath = lastEntry[0];
          pendingUnlinks.delete(oldPath);

          if (tryHandleRename(oldPath, filePath)) {
            return; // Procesado como rename, no reindexar
          }
          // Si falló el rename, reindexar
        }
      }
    }

    if (event !== 'add' && event !== 'change' && event !== 'unlink') return;
    if (!isVideo) return;
    options.logger?.debug('watch.event', { event, path: filePath });
    schedule();
  };

  watcher.on('all', onEvent);
  watcher.on('error', (error) => {
    options.onError?.(error);
  });

  const ready = new Promise<void>((resolve, reject) => {
    watcher.once('ready', () => {
      resolve();
    });
    watcher.once('error', reject);
  });

  return {
    ready,
    async close(): Promise<void> {
      closed = true;
      if (timer !== undefined) {
        clearTimeout(timer);
        timer = undefined;
      }
      watcher.off('all', onEvent);
      await watcher.close();
      if (running !== null) await running;
    },
  };
}
