import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { createWriteStream } from 'node:fs';
import {
  access,
  chmod,
  copyFile,
  cp,
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  rm,
} from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import process from 'node:process';
import { Readable } from 'node:stream';
import { pipeline } from 'node:stream/promises';
import { URL } from 'node:url';

import { nodeArchiveName, nodePlatform } from './platform.mjs';

const NODE_DIST_URL = 'https://nodejs.org/dist';

/**
 * Busca el SHA-256 de un archivo en el `SHASUMS256.txt` oficial.
 *
 * @param {string} contents
 * @param {string} filename
 * @returns {string | undefined}
 */
export function findChecksum(contents, filename) {
  for (const line of contents.split('\n')) {
    const [hash, name] = line.trim().split(/\s+/);
    if (name === filename && /^[0-9a-f]{64}$/.test(hash)) {
      return hash;
    }
  }
  return undefined;
}

/**
 * Descarga el runtime oficial de Node, comprueba su SHA-256 y extrae solo el
 * binario y la licencia. El resultado se cachea por versión y plataforma.
 *
 * @param {object} options
 * @param {string} options.version
 * @param {string} [options.cacheRoot]
 * @param {NodeJS.Platform} [options.system]
 * @param {string} [options.arch]
 * @returns {Promise<{binaryPath: string, licensePath: string, nodePlatform: string, version: string}>}
 */
export async function fetchNodeRuntime(options) {
  const version = options.version;
  if (!/^\d+\.\d+\.\d+$/.test(version)) {
    throw new Error(`Versión de Node inválida: ${version}`);
  }
  const system = options.system ?? process.platform;
  const arch = options.arch ?? process.arch;
  const platform = nodePlatform(system, arch);
  const archive = nodeArchiveName(version, system, arch);
  const root = `node-v${version}-${platform}`;
  const cacheRoot = options.cacheRoot ?? path.join(process.cwd(), 'build', 'cache', 'node');
  const cacheDirectory = path.join(cacheRoot, root);
  const binaryName = process.platform === 'win32' ? 'node.exe' : 'node';
  const binaryPath = path.join(cacheDirectory, binaryName);
  const licensePath = path.join(cacheDirectory, 'LICENSE');

  try {
    await access(binaryPath);
    await access(licensePath);
    return { binaryPath, licensePath, nodePlatform: platform, version };
  } catch {
    // Caché incompleta: se vuelve a descargar.
  }

  const workspace = await mkdtemp(path.join(tmpdir(), 'mm-node-'));
  try {
    const releaseUrl = `${NODE_DIST_URL}/v${version}`;
    const checksumUrl = `${releaseUrl}/SHASUMS256.txt`;
    const checksumText = await downloadText(checksumUrl);
    const expected = findChecksum(checksumText, archive);
    if (expected === undefined) {
      throw new Error(`El checksum oficial no incluye ${archive}`);
    }

    const archivePath = path.join(workspace, archive);
    await downloadFile(`${releaseUrl}/${archive}`, archivePath);
    const actual = await sha256File(archivePath);
    if (actual !== expected) {
      throw new Error(`Checksum incorrecto para ${archive}: ${actual} != ${expected}`);
    }

    const extractionDirectory = path.join(workspace, 'extract');
    await mkdir(extractionDirectory, { recursive: true });
    await extract(archivePath, extractionDirectory, [`${root}/bin/node`, `${root}/LICENSE`]);

    await mkdir(cacheDirectory, { recursive: true });
    const binarySource =
      process.platform === 'win32'
        ? path.join(extractionDirectory, 'node.exe')
        : path.join(extractionDirectory, 'bin', 'node');
    await copyFile(binarySource, binaryPath);
    await chmod(binaryPath, 0o755);
    await copyFile(path.join(extractionDirectory, 'LICENSE'), licensePath);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }

  return { binaryPath, licensePath, nodePlatform: platform, version };
}

async function downloadText(url) {
  const response = await request(url);
  return response.text();
}

async function downloadFile(url, destination) {
  const response = await request(url);
  if (response.body === null) {
    throw new Error(`Respuesta vacía al descargar ${url}`);
  }
  await pipeline(Readable.fromWeb(response.body), createWriteStream(destination));
}

async function request(url) {
  const parsed = new URL(url);
  if (parsed.protocol !== 'https:') {
    throw new Error(`Solo se permite HTTPS: ${url}`);
  }
  const response = await globalThis.fetch(url, {
    redirect: 'follow',
    signal: globalThis.AbortSignal.timeout(120_000),
  });
  if (!response.ok) {
    throw new Error(`No se pudo descargar ${url} (HTTP ${response.status})`);
  }
  return response;
}

async function sha256File(filePath) {
  return createHash('sha256')
    .update(await readFile(filePath))
    .digest('hex');
}

async function extract(archivePath, destination, members) {
  const isZip = archivePath.endsWith('.zip');
  if (isZip && process.platform === 'win32') {
    // Windows: Expand-Archive no tiene --strip-components; extrae a una carpeta
    // temporal y aclara su contenido al destino para que quede plano.
    const tmpDir = path.join(destination, '__extract_tmp');
    await mkdir(tmpDir, { recursive: true });
    const result = spawnSync(
      'powershell.exe',
      [
        '-NoProfile',
        '-NonInteractive',
        '-Command',
        `Expand-Archive -Path "${archivePath}" -DestinationPath "${tmpDir}" -Force`,
      ],
      { encoding: 'utf8' },
    );
    if (result.status !== 0) {
      throw new Error(`No se pudo extraer ${archivePath}: ${result.stderr}`);
    }
    const [inner] = await readdir(tmpDir);
    const innerPath = path.join(tmpDir, inner);
    for (const entry of await readdir(innerPath)) {
      await cp(path.join(innerPath, entry), path.join(destination, entry), {
        recursive: true,
      });
    }
    await rm(tmpDir, { recursive: true, force: true });
    return;
  }
  const result = spawnSync(
    'tar',
    ['-xzf', archivePath, '-C', destination, '--strip-components=1', ...members],
    { encoding: 'utf8' },
  );
  if (result.status !== 0) {
    throw new Error(`No se pudo extraer ${archivePath}: ${result.stderr}`);
  }
}
