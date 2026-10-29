import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { access, mkdtemp, readFile, rename, rm, symlink } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import process from 'node:process';

import { withCommandContext, type CommandOptions } from './context.js';
import { SelfUpdateError, formatError } from '../shared/errors.js';

import type { Logger } from '../shared/logger.js';

/** Información de una release de GitHub. */
type GitHubRelease = {
  tag_name: string;
  assets: GitHubReleaseAsset[];
};

/** Asset de una release de GitHub. */
type GitHubReleaseAsset = {
  name: string;
  browser_download_url: string;
  size: number;
};

/** Resultado de la comprobación de actualización. */
type UpdateCheckResult = {
  currentVersion: string;
  latestVersion: string;
  updateAvailable: boolean;
  assetUrl: string | null;
  assetName: string | null;
};

/**
 * Implementa `mm self-update`: comprueba y aplica actualizaciones del binario
 * desde GitHub Releases.
 *
 * Flujo:
 * 1. Lee la versión actual desde package.json.
 * 2. Consulta la API de GitHub Releases para obtener la última release.
 * 3. Si hay una versión más reciente, descarga el asset correspondiente.
 * 4. Verifica el checksum SHA-256.
 * 5. Extrae el bundle a un directorio versionado.
 * 6. Actualiza el enlace `current` para apuntar a la nueva versión.
 * 7. Verifica que el nuevo binario responde `--version` (rollback si falla).
 */
export async function runSelfUpdateCommand(
  options: CommandOptions & {
    check?: boolean;
    dryRun?: boolean;
  },
): Promise<void> {
  await withCommandContext(options, async ({ logger }) => {
    const currentVersion = getCurrentVersion();
    logger.debug('self-update: versión actual', { version: currentVersion });

    const checkResult = await checkForUpdate(currentVersion, logger);
    if (!checkResult.updateAvailable) {
      console.log(`Ya tienes la última versión (${currentVersion}).`);
      return;
    }

    console.log(`Actualización disponible: ${currentVersion} → ${checkResult.latestVersion}`);

    if (options.check === true) {
      return;
    }

    if (options.dryRun === true) {
      console.log(`[dry-run] Se descargaría: ${checkResult.assetName} (${checkResult.assetUrl})`);
      return;
    }

    await applyUpdate(checkResult, logger);
  });
}

/** Lee la versión actual desde package.json. */
function getCurrentVersion(): string {
  try {
    const packageJsonUrl = new URL('../../package.json', import.meta.url);
    const content = readFileSync(packageJsonUrl, 'utf8');
    const parsed: unknown = JSON.parse(content);
    if (typeof parsed === 'object' && parsed !== null && 'version' in parsed) {
      const version = parsed.version;
      if (typeof version === 'string') return version;
    }
  } catch {
    // Si no se puede leer, usar 0.0.0 para forzar actualización.
  }
  return '0.0.0';
}

/**
 * Consulta la API de GitHub Releases y compara versiones.
 */
async function checkForUpdate(currentVersion: string, logger: Logger): Promise<UpdateCheckResult> {
  const platform = getPlatformIdentifier();
  const arch = process.arch;
  const assetSuffix = platform === 'windows' ? '.zip' : '.tar.gz';

  let release: GitHubRelease;
  try {
    const response = await fetch(
      'https://api.github.com/repos/Angel-p-v/media-manager/releases/latest',
      {
        headers: { Accept: 'application/vnd.github+json' },
        signal: AbortSignal.timeout(10_000),
      },
    );
    if (!response.ok) {
      throw new SelfUpdateError(`GitHub API respondió ${response.status}`);
    }
    release = (await response.json()) as GitHubRelease;
  } catch (error) {
    logger.warn('self-update: no se pudo consultar GitHub', {
      error: formatError(error),
    });
    throw new SelfUpdateError(
      `No se pudo consultar la última release de GitHub: ${formatError(error)}`,
      { cause: error },
    );
  }

  const latestTag = release.tag_name.replace(/^v/, '');
  const isNewer = isVersionNewer(latestTag, currentVersion);
  if (!isNewer) {
    return {
      currentVersion,
      latestVersion: latestTag,
      updateAvailable: false,
      assetUrl: null,
      assetName: null,
    };
  }

  const assetName = `media-manager-cli-${latestTag}-${platform}-${arch}${assetSuffix}`;
  const asset = release.assets.find((a) => a.name === assetName);

  return {
    currentVersion,
    latestVersion: latestTag,
    updateAvailable: true,
    assetUrl: asset?.browser_download_url ?? null,
    assetName: asset?.name ?? null,
  };
}

/** Identificador de plataforma para nombres de asset (coincide con platform.mjs). */
export function getPlatformIdentifier(): 'linux' | 'windows' | 'darwin' {
  switch (process.platform) {
    case 'linux':
      return 'linux';
    case 'win32':
      return 'windows';
    case 'darwin':
      return 'darwin';
    default:
      return 'linux';
  }
}

/**
 * Compara dos versiones semver simples (major.minor.patch).
 * Devuelve true si `a` es mayor que `b`.
 */
export function isVersionNewer(a: string, b: string): boolean {
  const parse = (v: string): [number, number, number] => {
    const parts = v
      .replace(/^v/, '')
      .split('.')
      .map((p) => parseInt(p, 10) || 0);
    return [parts[0]!, parts[1] ?? 0, parts[2] ?? 0];
  };
  const [aMajor, aMinor, aPatch] = parse(a);
  const [bMajor, bMinor, bPatch] = parse(b);
  if (aMajor !== bMajor) return aMajor > bMajor;
  if (aMinor !== bMinor) return aMinor > bMinor;
  return aPatch > bPatch;
}

/** Descarga y aplica la actualización de forma atómica. */
async function applyUpdate(checkResult: UpdateCheckResult, logger: Logger): Promise<void> {
  if (!checkResult.assetUrl || !checkResult.assetName) {
    throw new SelfUpdateError('No hay asset de actualización disponible.');
  }

  const tempDir = await mkdtemp(path.join(tmpdir(), 'mm-update-'));
  let backupDir: string | null = null;
  let stagingDir: string | null = null;

  try {
    // 1. Descargar el asset
    const archivePath = path.join(tempDir, checkResult.assetName);
    logger.debug('self-update: descargando', { url: checkResult.assetUrl });
    await downloadFile(checkResult.assetUrl, archivePath);

    // 2. Descargar y verificar checksum SHA-256
    const checksumResponse = await fetch(`${checkResult.assetUrl}.sha256`, {
      signal: AbortSignal.timeout(10_000),
    });
    if (!checksumResponse.ok) {
      throw new SelfUpdateError('No se pudo descargar el checksum SHA-256.');
    }
    const checksumText = await checksumResponse.text();
    const expectedHash = checksumText.split(' ')[0]?.trim();
    if (!expectedHash) {
      throw new SelfUpdateError('Checksum SHA-256 no tiene formato válido.');
    }
    const actualHash = await sha256File(archivePath);
    if (actualHash !== expectedHash) {
      throw new SelfUpdateError(
        `Checksum no coincide: esperado ${expectedHash}, obtenido ${actualHash}`,
      );
    }
    logger.debug('self-update: checksum verificado');

    // 3. Detectar el directorio de instalación actual
    const installDir = await detectInstallDir();
    if (!installDir) {
      logger.warn('self-update: no se pudo detectar el directorio de instalación');
      console.log(
        'No se pudo detectar la instalación. Descarga la nueva versión desde GitHub Releases.',
      );
      return;
    }

    // 4. Extraer el bundle a un directorio de staging
    stagingDir = await mkdtemp(path.join(tempDir, 'stage-'));
    await extractBundle(archivePath, stagingDir, checkResult.assetName.endsWith('.zip'));

    // Leer install.json del staging para obtener la versión
    const installJsonPath = path.join(stagingDir, 'install.json');
    await access(installJsonPath);
    const installJsonContent = await readFile(installJsonPath, 'utf8');
    const installJson = JSON.parse(installJsonContent) as { version: string };
    const newVersion = installJson.version;

    // 5. Hacer backup del directorio actual (renombrar a .old)
    const installParent = path.dirname(installDir);
    const versionedDir = path.join(installParent, newVersion);
    const currentLink = path.join(installParent, 'current');

    backupDir = path.join(installParent, `.old-${newVersion}-${process.pid}`);
    if (await dirExists(installDir)) {
      await rename(installDir, backupDir);
    }

    // 6. Mover el nuevo directorio a su lugar
    await rename(stagingDir, versionedDir);
    stagingDir = null;

    // 7. Actualizar el enlace current
    const tmpLink = path.join(installParent, `.current-${process.pid}`);
    if (process.platform === 'win32') {
      const { spawnSync } = await import('node:child_process');
      spawnSync('cmd', ['/c', 'mklink', '/D', tmpLink, newVersion], {
        encoding: 'utf8',
        stdio: 'ignore',
      });
    } else {
      await symlink(newVersion, tmpLink);
    }
    await rename(tmpLink, currentLink);

    // 8. Verificar que el nuevo binario funciona
    const binName = process.platform === 'win32' ? 'mm.bat' : 'mm';
    const binPath = path.join(currentLink, binName);
    const { spawnSync } = await import('node:child_process');
    const versionCheck = spawnSync(binPath, ['--version'], {
      encoding: 'utf8',
      timeout: 10_000,
      cwd: path.dirname(binPath),
    });
    if (versionCheck.status !== 0) {
      throw new SelfUpdateError(
        `El nuevo binario falló al ejecutar --version (código ${versionCheck.status})`,
      );
    }

    console.log(`✔ Actualizado a ${newVersion}. El cambio surtirá efecto en la próxima ejecución.`);

    // 9. Eliminar backup (ya no se necesita)
    if (backupDir) {
      await rm(backupDir, { recursive: true, force: true });
      backupDir = null;
    }
  } catch (error) {
    // Rollback: restaurar backup
    if (backupDir && stagingDir === null) {
      try {
        if (await dirExists(backupDir)) {
          const installDir = await detectInstallDir();
          if (installDir) {
            await rename(backupDir, installDir);
          }
        }
      } catch {
        // Rollback best-effort
      }
    }
    throw new SelfUpdateError(`Fallo al aplicar la actualización: ${formatError(error)}`, {
      cause: error,
    });
  } finally {
    // Limpieza
    if (stagingDir) {
      try {
        await rm(stagingDir, { recursive: true, force: true });
      } catch {
        // Ignorado
      }
    }
    if (backupDir) {
      try {
        await rm(backupDir, { recursive: true, force: true });
      } catch {
        // Ignorado
      }
    }
    await rm(tempDir, { recursive: true, force: true });
  }
}

/** Detecta el directorio de instalación del bundle (donde está `node` y `node_modules`). */
async function detectInstallDir(): Promise<string | null> {
  // Cuando se ejecuta desde un bundle, process.execPath apunta a bundle/node (o node.exe)
  // y el bundle directorio es el padre de ese ejecutable.
  const execDir = path.dirname(process.execPath);
  const nodeModulesDir = path.join(execDir, 'node_modules');
  try {
    await access(nodeModulesDir);
    return execDir;
  } catch {
    // No es un bundle: buscar en el PATH
    const { spawnSync } = await import('node:child_process');
    const whichCmd = process.platform === 'win32' ? 'where' : 'which';
    const result = spawnSync(whichCmd, ['mm'], { encoding: 'utf8' });
    if (result.status === 0 && result.stdout) {
      const mmPath = result.stdout.trim().split('\n')[0];
      if (!mmPath) return null;
      const mmDir = path.dirname(mmPath);
      // El launcher apunta a ../share/media-manager-cli/current
      const parentShare = path.join(mmDir, '..', 'share', 'media-manager-cli');
      try {
        await access(parentShare);
        return path.resolve(parentShare, 'current');
      } catch {
        // No se encontró la estructura esperada
      }
    }
  }
  return null;
}

/** Verifica si un directorio existe. */
async function dirExists(dirPath: string): Promise<boolean> {
  try {
    await access(dirPath);
    return true;
  } catch {
    return false;
  }
}

/** Descarga un fichero desde una URL. */
async function downloadFile(url: string, destination: string): Promise<void> {
  const response = await fetch(url, {
    signal: AbortSignal.timeout(60_000),
  });
  if (!response.ok) {
    throw new SelfUpdateError(`No se pudo descargar ${url} (HTTP ${response.status})`);
  }
  if (response.body === null) {
    throw new SelfUpdateError(`Respuesta vacía al descargar ${url}`);
  }
  const { createWriteStream } = await import('node:fs');
  const { pipeline } = await import('node:stream/promises');
  const { Readable } = await import('node:stream');
  await pipeline(Readable.fromWeb(response.body), createWriteStream(destination));
}

/** Calcula el SHA-256 de un fichero. */
async function sha256File(filePath: string): Promise<string> {
  const content = await readFile(filePath);
  return createHash('sha256').update(content).digest('hex');
}

/** Extrae un bundle (.tar.gz o .zip) a un directorio de destino. */
async function extractBundle(
  archivePath: string,
  destination: string,
  isZip: boolean,
): Promise<void> {
  if (isZip && process.platform === 'win32') {
    const { spawnSync } = await import('node:child_process');
    const result = spawnSync(
      'powershell.exe',
      [
        '-NoProfile',
        '-NonInteractive',
        '-Command',
        `Expand-Archive -Path "${archivePath}" -DestinationPath "${destination}" -Force`,
      ],
      { encoding: 'utf8' },
    );
    if (result.status !== 0) {
      throw new SelfUpdateError(`No se pudo extraer ${archivePath}: ${result.stderr}`);
    }
    return;
  }
  const { spawnSync } = await import('node:child_process');
  const result = spawnSync(
    'tar',
    ['-xzf', archivePath, '-C', destination, '--strip-components=1'],
    { encoding: 'utf8' },
  );
  if (result.status !== 0) {
    throw new SelfUpdateError(`No se pudo extraer ${archivePath}: ${result.stderr}`);
  }
}
