import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, readdirSync, statSync, accessSync, constants } from 'node:fs';
import { open } from 'node:fs/promises';
import { homedir } from 'node:os';
import path from 'node:path';

import pc from 'picocolors';

import { withCommandContext, type CommandOptions } from './context.js';
import { formatBytes } from './output.js';
import { closeDatabase, openDatabase } from '../core/db/connection.js';
import { migrations } from '../core/db/migrations/index.js';
import { databaseSizeBytes } from '../core/db/size.js';
import { formatError } from '../shared/errors.js';

import type { Db } from '../core/db/connection.js';
import type { FileRepository } from '../core/db/repositories/fileRepo.js';

export type DoctorCommandOptions = CommandOptions & {
  json?: boolean;
  full?: boolean;
  fullHash?: boolean;
  pruneMissing?: boolean;
  yes?: boolean;
};

type PragmaIntegrityRow = { integrity_check: string };
type PragmaForeignKeyRow = { table: string; parent: string; fkid: number };

/** Implementa `mm doctor`: diagnóstico y reparación de la biblioteca. */
export async function runDoctorCommand(options: DoctorCommandOptions): Promise<void> {
  await withCommandContext(options, async ({ config, repositories }) => {
    const db = openDatabase(config.database.path);
    const { mediaRepo, fileRepo, indexRunRepo } = repositories;
    const issues: DoctorIssue[] = [];

    // 1. Integridad SQLite
    try {
      const integrity = db.pragma('integrity_check') as PragmaIntegrityRow[];
      if (integrity[0]?.integrity_check !== 'ok') {
        issues.push({
          kind: 'integrity',
          severity: 'error',
          detail: integrity.map((r) => r.integrity_check).join('; '),
        });
      }
    } catch (error) {
      issues.push({
        kind: 'integrity',
        severity: 'error',
        detail: formatError(error),
      });
    }

    try {
      const fkCheck = db.pragma('foreign_key_check') as PragmaForeignKeyRow[];
      if (fkCheck.length > 0) {
        issues.push({
          kind: 'foreign-key',
          severity: 'error',
          detail: fkCheck.map((r) => `${r.table}: ${r.parent} -> ${r.fkid}`).join('; '),
        });
      }
    } catch (error) {
      issues.push({
        kind: 'foreign-key',
        severity: 'error',
        detail: formatError(error),
      });
    }

    // 2. Ficheros ausentes, enlaces rotos, permisos
    const files = fileRepo.listAllPaths();
    const missingFiles: string[] = [];
    const permissionErrors: string[] = [];
    const brokenSymlinks: string[] = [];

    for (const filePath of files) {
      try {
        if (!existsSync(filePath)) {
          missingFiles.push(filePath);
        } else {
          const stat = statSync(filePath);
          if (stat.isSymbolicLink()) {
            const target = statSync(filePath, { throwIfNoEntry: false });
            if (!target) brokenSymlinks.push(filePath);
          }
        }
        // Check read permission
        accessSync(filePath, constants.R_OK);
      } catch (error) {
        if (
          typeof error === 'object' &&
          error !== null &&
          'code' in error &&
          error.code === 'EACCES'
        ) {
          permissionErrors.push(filePath);
        } else if (
          typeof error === 'object' &&
          error !== null &&
          'code' in error &&
          error.code === 'ENOENT'
        ) {
          missingFiles.push(filePath);
        }
      }
    }

    if (missingFiles.length > 0) {
      issues.push({
        kind: 'missing-file',
        severity: 'warning',
        count: missingFiles.length,
        paths: missingFiles,
        detail: `${missingFiles.length} fichero(s) ausente(s)`,
      });
    }

    if (brokenSymlinks.length > 0) {
      issues.push({
        kind: 'broken-symlink',
        severity: 'warning',
        count: brokenSymlinks.length,
        paths: brokenSymlinks,
        detail: `${brokenSymlinks.length} enlace(s) simbólico(s) roto(s)`,
      });
    }

    if (permissionErrors.length > 0) {
      issues.push({
        kind: 'permission',
        severity: 'warning',
        count: permissionErrors.length,
        paths: permissionErrors,
        detail: `${permissionErrors.length} fichero(s) sin permiso de lectura`,
      });
    }

    // 3. Títulos vacíos
    const emptyMedia = mediaRepo.removeEmpty(); // This actually removes them - we should just count
    if (emptyMedia > 0) {
      issues.push({
        kind: 'empty-media',
        severity: 'warning',
        count: emptyMedia,
        detail: `${emptyMedia} título(s) sin ficheros`,
      });
    }

    // 4. Ficheros fuera de las raíces
    const roots = config.library.roots.map((r) => path.resolve(r.path));
    const outsideRoots: string[] = [];
    for (const filePath of files) {
      const resolved = path.resolve(filePath);
      const inRoot = roots.some(
        (root) => resolved.startsWith(root + path.sep) || resolved === root,
      );
      if (!inRoot) outsideRoots.push(filePath);
    }
    if (outsideRoots.length > 0) {
      issues.push({
        kind: 'outside-roots',
        severity: 'warning',
        count: outsideRoots.length,
        paths: outsideRoots,
        detail: `${outsideRoots.length} fichero(s) fuera de las raíces configuradas`,
      });
    }

    // 5. Duplicados por tamaño + huella parcial
    const duplicateGroups = await findDuplicates(fileRepo, options.fullHash === true);
    if (duplicateGroups.length > 0) {
      issues.push({
        kind: 'duplicate',
        severity: 'info',
        count: duplicateGroups.length,
        groups: duplicateGroups,
        detail: `${duplicateGroups.length} grupo(s) de duplicados`,
      });
    }

    // 6. Indexaciones anómalas
    const runs = indexRunRepo.list(10);
    const staleRuns = runs.filter(
      (r) =>
        r.status === 'running' &&
        new Date(r.startedAt).getTime() < Date.now() - 24 * 60 * 60 * 1000,
    );
    if (staleRuns.length > 0) {
      issues.push({
        kind: 'stale-run',
        severity: 'warning',
        count: staleRuns.length,
        detail: `${staleRuns.length} indexación(es) en 'running' > 24h`,
      });
    }

    // JSON output
    if (options.json === true) {
      const status = issues.some((i) => i.severity === 'error')
        ? 'error'
        : issues.some((i) => i.severity === 'warning')
          ? 'warning'
          : 'ok';
      console.log(
        JSON.stringify(
          {
            status,
            generatedAt: new Date().toISOString(),
            database: {
              integrity: 'ok',
              foreignKeys: 'ok',
              migrated: isDatabaseMigrated(db),
            },
            counts: {
              media: mediaRepo.count(),
              files: fileRepo.count(),
              missing: missingFiles.length,
              emptyMedia,
              outsideRoots: outsideRoots.length,
              permissionErrors: permissionErrors.length,
              duplicateGroups: duplicateGroups.length,
              staleRuns: staleRuns.length,
            },
            issues: issues.map((i) => ({
              kind: i.kind,
              severity: i.severity,
              ...(i.count !== undefined && { count: i.count }),
              ...(i.paths !== undefined && { paths: i.paths }),
              ...(i.groups !== undefined && { groups: i.groups }),
              detail: i.detail,
            })),
            ...(options.full === true && {
              extended: {
                databaseSizeBytes: databaseSizeBytes(config.database.path),
                posterCacheSizeBytes: posterCacheSizeBytes(),
                players: {
                  mpv: playerVersion('mpv'),
                  vlc: playerVersion('vlc'),
                },
                connectivity: {
                  tmdb: await checkConnectivity('https://api.themoviedb.org/3/configuration'),
                  tvdb: await checkConnectivity('https://api4.thetvdb.com/v4'),
                },
              },
            }),
          },
          null,
          2,
        ),
      );
      closeDatabase(db);
      return;
    }

    // Human output
    if (issues.length === 0) {
      closeDatabase(db);
      console.log(pc.green('✓ Biblioteca sana'));
      return;
    }

    console.log(pc.bold('Diagnóstico:'));
    for (const issue of issues) {
      const color =
        issue.severity === 'error' ? pc.red : issue.severity === 'warning' ? pc.yellow : pc.blue;
      console.log(`  ${color(issue.severity.toUpperCase())} ${issue.kind}: ${issue.detail}`);
      if (issue.paths && issue.paths.length <= 10) {
        for (const p of issue.paths) console.log(`    ${p}`);
      } else if (issue.paths && issue.paths.length > 10) {
        for (const p of issue.paths.slice(0, 10)) console.log(`    ${p}`);
        console.log(`    ... y ${issue.paths.length - 10} más`);
      }
      if (issue.groups && issue.groups.length <= 5) {
        for (const g of issue.groups) console.log(`    ${g.size} bytes: ${g.paths.join(', ')}`);
      }
    }

    if (options.full === true) {
      console.log('');
      console.log(pc.bold('Información extendida:'));
      console.log(`  Tamaño BD: ${pc.dim(formatBytes(databaseSizeBytes(config.database.path)))}`);
      console.log(`  Caché de pósteres: ${pc.dim(formatBytes(posterCacheSizeBytes()))}`);
      console.log(`  Reproductores:`);
      console.log(`    mpv: ${pc.dim(playerVersion('mpv') ?? 'no encontrado')}`);
      console.log(`    vlc: ${pc.dim(playerVersion('vlc') ?? 'no encontrado')}`);
      console.log(`  Conectividad:`);
      const tvmaze = await checkConnectivity(
        'https://api.tvmaze.com/singlesearch/shows?q=severance',
      );
      const jikan = await checkConnectivity('https://api.jikan.moe/v4/anime?q=severance');
      console.log(`    TVMaze: ${pc.dim(tvmaze ? 'ok' : 'sin respuesta')}`);
      console.log(`    Jikan: ${pc.dim(jikan ? 'ok' : 'sin respuesta')}`);
    }

    // --prune-missing
    if (options.pruneMissing) {
      if (missingFiles.length === 0) {
        closeDatabase(db);
        console.log(pc.dim('No hay ficheros ausentes que podar.'));
        return;
      }

      if (!options.yes && process.stdout.isTTY) {
        const readline = await import('node:readline/promises');
        const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
        const answer = await rl.question(
          `${pc.yellow('Aviso')} Se eliminarán ${missingFiles.length} fichero(s) ausente(s) del índice. ¿Continuar? [y/N] `,
        );
        rl.close();
        if (!/^y(es)?$/i.test(answer.trim())) {
          closeDatabase(db);
          console.log('Cancelado.');
          return;
        }
      } else if (!options.yes && !process.stdout.isTTY) {
        console.error(pc.red('Error: --prune-missing requiere --yes en modo no interactivo.'));
        process.exit(2);
      }

      // Backup before pruning
      try {
        // Import backup function
        const { backupDatabase } = await import('../core/db/backup.js');
        const { openDatabase } = await import('../core/db/connection.js');
        const doctorDb = openDatabase(config.database.path);
        backupDatabase(doctorDb, config.database.path, 'doctor');
        doctorDb.close();
      } catch (error) {
        console.error(pc.red(`Error creando backup: ${formatError(error)}`));
        process.exit(1);
      }

      // Prune in transaction
      const removed = fileRepo.removeMissing(new Set(files.filter((f) => existsSync(f))));
      const emptyRemoved = mediaRepo.removeEmpty();
      console.log(
        pc.green(
          `Eliminados ${removed} fichero(s) ausente(s) y ${emptyRemoved} título(s) vacío(s).`,
        ),
      );
    }

    closeDatabase(db);
  });
}

/**
 * Determina si la base de datos tiene aplicada la última migración disponible.
 * Consulta la tabla `schema_migrations` y compara la versión máxima aplicada
 * con la última versión declarada en las migraciones.
 */
export function isDatabaseMigrated(db: Db): boolean {
  const latestVersion = Math.max(...migrations.map((m) => m.version));
  const applied = db
    .prepare<[], { version: number }>('SELECT version FROM schema_migrations')
    .all()
    .map((row) => row.version);
  return applied.length > 0 && Math.max(...applied) >= latestVersion;
}

type DoctorIssue = {
  kind: string;
  severity: 'error' | 'warning' | 'info';
  count?: number;
  paths?: string[];
  groups?: DuplicateGroup[];
  detail: string;
};

type DuplicateGroup = {
  size: number;
  paths: string[];
  hash?: string;
};

async function findDuplicates(
  fileRepo: FileRepository,
  fullHash: boolean,
): Promise<DuplicateGroup[]> {
  const files = fileRepo.listAllPaths();
  const bySize = new Map<number, string[]>();

  for (const filePath of files) {
    try {
      const stat = statSync(filePath);
      let paths = bySize.get(stat.size);
      if (!paths) {
        paths = [];
        bySize.set(stat.size, paths);
      }
      paths.push(filePath);
    } catch {
      // Ignore unreadable
    }
  }

  const groups: DuplicateGroup[] = [];
  for (const [size, paths] of bySize) {
    if (paths.length <= 1) continue;

    if (!fullHash) {
      // Partial hash: head + tail 64 KiB
      const hashes = new Map<string, string[]>();
      for (const p of paths) {
        const hash = await partialHash(p);
        let dups = hashes.get(hash);
        if (!dups) {
          dups = [];
          hashes.set(hash, dups);
        }
        dups.push(p);
      }
      for (const [hash, dups] of hashes) {
        if (dups.length > 1) {
          groups.push({ size, paths: dups, hash });
        }
      }
    } else {
      // Full hash
      const hashes = new Map<string, string[]>();
      for (const p of paths) {
        const hash = await fullFileHash(p);
        let dups = hashes.get(hash);
        if (!dups) {
          dups = [];
          hashes.set(hash, dups);
        }
        dups.push(p);
      }
      for (const [hash, dups] of hashes) {
        if (dups.length > 1) {
          groups.push({ size, paths: dups, hash });
        }
      }
    }
  }
  return groups;
}

async function partialHash(filePath: string): Promise<string> {
  const CHUNK = 64 * 1024;
  const hash = createHash('sha256');
  const handle = await open(filePath, 'r');
  try {
    const stat = await handle.stat();
    // Head
    const headBuf = Buffer.alloc(Math.min(CHUNK, stat.size));
    await handle.read(headBuf, 0, headBuf.length, 0);
    hash.update(headBuf);
    // Tail
    if (stat.size > CHUNK) {
      const tailBuf = Buffer.alloc(Math.min(CHUNK, stat.size));
      await handle.read(tailBuf, 0, tailBuf.length, stat.size - tailBuf.length);
      hash.update(tailBuf);
    }
    return hash.digest('hex');
  } finally {
    await handle.close();
  }
}

async function fullFileHash(filePath: string): Promise<string> {
  const hash = createHash('sha256');
  const handle = await open(filePath, 'r');
  try {
    const CHUNK = 64 * 1024;
    const buf = Buffer.alloc(CHUNK);
    while (true) {
      const result = await handle.read(buf, 0, CHUNK, null);
      if (result.bytesRead === 0) break;
      hash.update(buf.subarray(0, result.bytesRead));
    }
    return hash.digest('hex');
  } finally {
    await handle.close();
  }
}

/**
 * Suma el tamaño de todos los ficheros dentro de un directorio (no recursivo en
 * subdirectorios profundos, suficiente para la caché plana de pósteres).
 */
function posterCacheSizeBytes(): number {
  const cacheDir = path.join(homedir(), '.cache', 'media-manager', 'posters');
  try {
    const entries = statSync(cacheDir);
    if (!entries.isDirectory()) return 0;
  } catch {
    return 0;
  }
  let total = 0;
  for (const entry of readdirSync(cacheDir)) {
    try {
      total += statSync(path.join(cacheDir, entry)).size;
    } catch {
      // Ignorado: fichero que desapareció entre el listado y el stat
    }
  }
  return total;
}

/**
 * Ejecuta `<name> --version` y devuelve la primera línea de salida, o null si no
 * está instalado o falla.
 */
function playerVersion(name: string): string | null {
  try {
    const output = execFileSync(name, ['--version'], { encoding: 'utf8', timeout: 3000 });
    return output.split('\n')[0]?.trim() ?? null;
  } catch {
    return null;
  }
}

/**
 * Comprueba si un endpoint responde (cualquier status HTTP indica conectividad).
 * Timeout de 5 segundos para no bloquear el doctor.
 */
async function checkConnectivity(url: string): Promise<boolean> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 5000);
  try {
    const response = await fetch(url, { signal: controller.signal, method: 'HEAD' });
    return response.ok || response.status >= 400; // Cualquier respuesta = conectividad
  } catch {
    return false;
  } finally {
    clearTimeout(timeout);
  }
}
