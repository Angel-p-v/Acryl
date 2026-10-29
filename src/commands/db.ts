import { chmodSync, copyFileSync, existsSync } from 'node:fs';

import pc from 'picocolors';

import { withCommandContext, type CommandOptions } from './context.js';
import { formatBytes } from './output.js';
import { backupDatabase } from '../core/db/backup.js';
import { closeDatabase, openDatabase } from '../core/db/connection.js';
import { databaseSizeBytes } from '../core/db/size.js';
import { DbError } from '../shared/errors.js';

export type DbStatsCommandOptions = CommandOptions & {
  json?: boolean;
};

export type DbOptimizeCommandOptions = CommandOptions & {
  vacuum?: boolean;
  json?: boolean;
};

export type DbBackupCommandOptions = CommandOptions & {
  /** Etiqueta para identificar el backup (ej. 'manual', 'antes-de-migrar'). */
  label?: string;
  json?: boolean;
};

export type DbRestoreCommandOptions = CommandOptions & {
  /** Ruta al fichero de backup a restaurar. */
  from: string;
  json?: boolean;
};

/** Implementa `mm db stats`: tamaño y contenido del índice local. */
export async function runDbStatsCommand(options: DbStatsCommandOptions): Promise<void> {
  await withCommandContext(options, ({ config, library }) => {
    const stats = library.stats();
    const databasePath = config.database.path;
    const sizeBytes = databaseSizeBytes(databasePath);

    if (options.json) {
      console.log(
        JSON.stringify(
          {
            database: { path: databasePath, sizeBytes },
            media: stats.media,
            files: stats.files,
            lastRun:
              stats.lastRun === null
                ? null
                : {
                    id: stats.lastRun.id,
                    status: stats.lastRun.status,
                    startedAt: stats.lastRun.startedAt,
                    finishedAt: stats.lastRun.finishedAt,
                    filesSeen: stats.lastRun.filesSeen,
                    filesAdded: stats.lastRun.filesAdded,
                    filesRemoved: stats.lastRun.filesRemoved,
                  },
          },
          null,
          2,
        ),
      );
      return;
    }

    console.log(pc.bold('Base de datos'));
    console.log(`  ${databasePath} ${pc.dim(`(${formatBytes(sizeBytes)})`)}`);
    console.log('');
    console.log(pc.bold('Contenido'));
    console.log(`  ${stats.media.total} títulos · ${stats.files} ficheros`);
    console.log(
      `  ${stats.media.series} series · ${stats.media.anime} anime · ${stats.media.movie} películas`,
    );
    console.log('');
    console.log(pc.bold('Última indexación'));
    if (stats.lastRun === null) {
      console.log(pc.dim('  (todavía no se ha indexado nada)'));
    } else {
      const run = stats.lastRun;
      console.log(
        `  #${run.id} ${run.status} · ${run.startedAt} · ${run.filesSeen} vistos · ` +
          `${run.filesAdded} añadidos · ${run.filesRemoved} eliminados`,
      );
    }
  });
}

/** Implementa `mm db optimize`: actualiza estadísticas y, opcionalmente, compacta. */
export async function runDbOptimizeCommand(options: DbOptimizeCommandOptions): Promise<void> {
  await withCommandContext(options, ({ config, maintenance, logger }) => {
    const beforeBytes = databaseSizeBytes(config.database.path);
    const startedAt = performance.now();

    maintenance.optimize();
    if (options.vacuum === true) maintenance.vacuum();

    const durationMs = Math.round(performance.now() - startedAt);
    const afterBytes = databaseSizeBytes(config.database.path);
    logger.debug('db.optimized', {
      vacuum: options.vacuum === true,
      beforeBytes,
      afterBytes,
      durationMs,
    });

    if (options.json) {
      console.log(
        JSON.stringify(
          {
            optimized: true,
            vacuum: options.vacuum === true,
            beforeBytes,
            afterBytes,
            durationMs,
          },
          null,
          2,
        ),
      );
      return;
    }

    const method = options.vacuum === true ? 'PRAGMA optimize + VACUUM' : 'PRAGMA optimize';
    console.log(pc.green('✔') + ` Base de datos optimizada (${method}) en ${durationMs} ms`);
    console.log(pc.dim(`  ${formatBytes(beforeBytes)} → ${formatBytes(afterBytes)}`));
  });
}

/** Implementa `mm db backup`: crea una copia de seguridad de la base de datos. */
export async function runDbBackupCommand(options: DbBackupCommandOptions): Promise<void> {
  const config = await loadConfig(
    options.config === undefined ? {} : { configPath: options.config },
  );
  const dbPath = config.database.path;

  if (dbPath === ':memory:') {
    throw new DbError('No se puede hacer backup de una base de datos en memoria.');
  }

  const db = openDatabase(dbPath);
  try {
    const backup = backupDatabase(db, dbPath, options.label ?? 'manual');

    if (options.json) {
      console.log(
        JSON.stringify({ backup: { path: backup.path, createdAt: backup.createdAt } }, null, 2),
      );
      return;
    }

    console.log(pc.green('✔') + ` Backup creado: ${backup.path}`);
    console.log(pc.dim(`  ${databaseSizeBytes(backup.path)} bytes`));
  } finally {
    closeDatabase(db);
  }
}

/** Implementa `mm db restore`: restaura la base de datos desde un fichero de backup. */
export async function runDbRestoreCommand(options: DbRestoreCommandOptions): Promise<void> {
  const config = await loadConfig(
    options.config === undefined ? {} : { configPath: options.config },
  );
  const dbPath = config.database.path;

  if (!existsSync(options.from)) {
    throw new DbError(`El archivo de backup no existe: ${options.from}`);
  }

  if (dbPath === ':memory:') {
    throw new DbError('No se puede restaurar sobre una base de datos en memoria.');
  }

  // Sobrescribir la base de datos actual con el backup.
  copyFileSync(options.from, dbPath);
  chmodSync(dbPath, 0o600);

  // Verificar que la base restaurada es válida abriéndola y cerrándola.
  const db = openDatabase(dbPath);
  closeDatabase(db);

  if (options.json) {
    console.log(JSON.stringify({ restored: true, from: options.from, to: dbPath }, null, 2));
    return;
  }

  console.log(pc.green('✔') + ` Base de datos restaurada desde: ${options.from}`);
  console.log(pc.dim(`  ${databaseSizeBytes(dbPath)} bytes`));
}

/** Carga la configuración sin abrir la biblioteca (usado por backup/restore). */
async function loadConfig(options: {
  configPath?: string;
}): Promise<{ database: { path: string } }> {
  const { loadConfig: loadConfigFn } = await import('../core/config/loader.js');
  return await loadConfigFn(options);
}
