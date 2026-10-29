import { readFile } from 'node:fs/promises';
import path from 'node:path';

/**
 * Genera los avisos de terceros del bundle a partir del lockfile.
 *
 * @param {object} options
 * @param {string} options.root
 * @param {string} options.lockfilePath
 * @param {string} options.nodeVersion
 * @param {string} [options.nodeLicense]
 * @returns {Promise<string>}
 */
export async function buildNotices(options) {
  const lockfile = JSON.parse(await readFile(options.lockfilePath, 'utf8'));
  const packages = [];

  for (const [key, metadata] of Object.entries(lockfile.packages ?? {})) {
    if (!key.startsWith('node_modules/') || metadata.dev === true || metadata.link === true) {
      continue;
    }
    let name = key.slice('node_modules/'.length);
    let license = metadata.license;
    try {
      const manifest = JSON.parse(
        await readFile(path.join(options.root, key, 'package.json'), 'utf8'),
      );
      name = manifest.name ?? name;
      license = manifest.license ?? license;
    } catch {
      // La dependencia no está instalada: se usa lo que declare el lockfile.
    }
    packages.push({
      license: typeof license === 'string' ? license : 'desconocida',
      name,
      version: metadata.version ?? 'desconocida',
    });
  }

  const unique = new Map(packages.map((entry) => [`${entry.name}@${entry.version}`, entry]));
  const sorted = [...unique.values()].sort((left, right) =>
    `${left.name}@${left.version}`.localeCompare(`${right.name}@${right.version}`),
  );

  const lines = [
    '# Avisos de terceros',
    '',
    'Este bundle portable incluye software de terceros. Las licencias completas de cada',
    'paquete están en `node_modules/<paquete>/`.',
    '',
    '## Dependencias de producción',
    '',
    '| Paquete | Versión | Licencia |',
    '| ------- | ------- | -------- |',
    ...sorted.map((entry) => `| \`${entry.name}\` | ${entry.version} | ${entry.license} |`),
    '',
    '## Runtime de Node.js',
    '',
    `Node.js v${options.nodeVersion} se distribuye con su propia licencia.`,
    '',
  ];
  if (options.nodeLicense !== undefined) {
    lines.push('```text', options.nodeLicense.trim(), '```', '');
  }
  return `${lines.join('\n')}\n`;
}
