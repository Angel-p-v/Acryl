import process from 'node:process';

const OS_NAMES = { darwin: 'macos', linux: 'linux', win32: 'windows' };
const NODE_OS_NAMES = { darwin: 'darwin', linux: 'linux', win32: 'win' };
const ARCH_NAMES = { arm64: 'arm64', x64: 'x64' };

/**
 * Normaliza sistema y arquitectura al identificador de los assets.
 *
 * @param {NodeJS.Platform} [system]
 * @param {string} [arch]
 * @returns {string}
 */
export function normalizePlatform(system = process.platform, arch = process.arch) {
  const os = OS_NAMES[system];
  const cpu = ARCH_NAMES[arch];
  if (os === undefined || cpu === undefined) {
    throw new Error(`Plataforma no soportada: ${system}-${arch}`);
  }
  return `${os}-${cpu}`;
}

/**
 * Nombre que usa Node.js para sus distribuciones oficiales.
 *
 * @param {NodeJS.Platform} [system]
 * @param {string} [arch]
 * @returns {string}
 */
export function nodePlatform(system = process.platform, arch = process.arch) {
  const os = NODE_OS_NAMES[system];
  const cpu = ARCH_NAMES[arch];
  if (os === undefined || cpu === undefined) {
    throw new Error(`Node no publica binarios para ${system}-${arch}`);
  }
  return `${os}-${cpu}`;
}

/**
 * @param {string} version
 * @param {string} platform
 * @returns {string}
 */
export function artifactName(version, platform) {
  const ext = platform.startsWith('windows') ? '.zip' : '.tar.gz';
  return `media-manager-cli-${version}-${platform}${ext}`;
}

/**
 * @param {string} version
 * @param {string} platform
 * @returns {string}
 */
export function bundleName(version, platform) {
  return `media-manager-cli-${version}-${platform}`;
}

/**
 * @param {string} version
 * @param {NodeJS.Platform} [system]
 * @param {string} [arch]
 * @returns {string}
 */
export function nodeArchiveName(version, system = process.platform, arch = process.arch) {
  const ext = system === 'win32' ? '.zip' : '.tar.gz';
  return `node-v${version}-${nodePlatform(system, arch)}${ext}`;
}
