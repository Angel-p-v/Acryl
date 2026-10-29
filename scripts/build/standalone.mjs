#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import console from 'node:console';
import { createHash } from 'node:crypto';
import {
  chmod,
  copyFile,
  cp,
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  rm,
  writeFile,
} from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

import { fetchNodeRuntime } from './fetch-node.mjs';
import { buildNotices } from './notices.mjs';
import { artifactName, bundleName, normalizePlatform } from './platform.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const NODE_VERSION_FILE = path.join(ROOT, 'scripts', 'build', 'node-version.txt');
const DISTRIBUTION_FILES = [
  'README.md',
  'PLAN.md',
  'CHANGELOG.md',
  'CONTRIBUTING.md',
  'SECURITY.md',
  'LICENSE',
];

/**
 * Crea una distribución portable con el runtime de Node incluido.
 *
 * No es un único ejecutable de Node SEA: `better-sqlite3` incluye un addon
 * nativo y necesita seguir en disco. El resultado es un directorio (y un
 * `.tar.gz` con su checksum) que funciona en la misma plataforma/arquitectura
 * sin Node instalado.
 */
async function main() {
  const options = parseArguments(process.argv.slice(2));
  const packageJson = JSON.parse(await readFile(path.join(ROOT, 'package.json'), 'utf8'));
  const platform = normalizePlatform();
  const version = packageJson.version;
  const name = bundleName(version, platform);
  const outputDirectory = path.join(ROOT, 'build', 'standalone');
  const bundleDirectory = path.join(outputDirectory, name);
  const workspace = await mkdtemp(path.join(tmpdir(), 'mm-standalone-'));

  try {
    await rm(bundleDirectory, { recursive: true, force: true });
    await mkdir(bundleDirectory, { recursive: true });
    await mkdir(path.join(workspace, 'pack'), { recursive: true });

    step('empaquetando la aplicación');
    run('npm', ['pack', '--pack-destination', path.join(workspace, 'pack'), '--silent'], ROOT);
    const tarballs = (await readdir(path.join(workspace, 'pack'))).filter((file) =>
      file.endsWith('.tgz'),
    );
    if (tarballs.length !== 1) {
      throw new Error(`Se esperaba un tarball y hay ${tarballs.length}`);
    }

    await writeFile(
      path.join(bundleDirectory, 'package.json'),
      `${JSON.stringify(
        {
          name: `${packageJson.name}-standalone`,
          private: true,
          allowScripts: { 'better-sqlite3': true },
        },
        null,
        2,
      )}\n`,
    );

    step('instalando dependencias de producción');
    run(
      'npm',
      [
        'install',
        '--no-audit',
        '--no-fund',
        '--omit=dev',
        path.join(workspace, 'pack', tarballs[0]),
      ],
      bundleDirectory,
    );

    step('copiando documentación y configuración');
    for (const file of DISTRIBUTION_FILES) {
      await copyFile(path.join(ROOT, file), path.join(bundleDirectory, file));
    }
    await cp(path.join(ROOT, 'config'), path.join(bundleDirectory, 'config'), {
      recursive: true,
    });
    await cp(path.join(ROOT, 'docs'), path.join(bundleDirectory, 'docs'), { recursive: true });

    step('preparando el runtime de Node');
    const nodeInfo = await prepareRuntime(options, bundleDirectory);

    step('generando install.json y avisos de terceros');
    await writeFile(
      path.join(bundleDirectory, 'install.json'),
      `${JSON.stringify(
        {
          schemaVersion: 1,
          name: packageJson.name,
          version,
          platform,
        },
        null,
        2,
      )}\n`,
    );
    const notices = await buildNotices({
      lockfilePath: path.join(ROOT, 'package-lock.json'),
      nodeLicense: nodeInfo.license,
      nodeVersion: nodeInfo.version,
      root: ROOT,
    });
    await writeFile(path.join(bundleDirectory, 'THIRD_PARTY_NOTICES.md'), notices);

    const packagePath = packageJson.name.split('/').join(path.sep);
    const isWindows = platform.startsWith('windows');
    if (isWindows) {
      const launcher = createWindowsLauncher(packagePath);
      const launcherPath = path.join(bundleDirectory, 'mm.bat');
      await writeFile(launcherPath, launcher);
    } else {
      const launcher = createPosixLauncher(packagePath);
      const launcherPath = path.join(bundleDirectory, 'mm');
      await writeFile(launcherPath, launcher);
      await chmod(launcherPath, 0o755);
    }

    step('comprobando el lanzador');
    const launcherName = isWindows ? 'mm.bat' : 'mm';
    const launcherPath = path.join(bundleDirectory, launcherName);
    // Solo verificar el lanzador si estamos construyendo para la plataforma actual
    // (no se puede ejecutar .bat en Linux ni .sh en Windows sin WSL).
    if (process.platform === (isWindows ? 'win32' : process.platform)) {
      const reportedVersion = run(launcherPath, ['--version'], bundleDirectory).stdout.trim();
      if (reportedVersion !== version) {
        throw new Error(`El lanzador devolvió ${reportedVersion} y se esperaba ${version}`);
      }
    } else {
      step(`lanzador ${launcherName} generado (verificación omitida en compilación cruzada)`);
    }

    step('creando el archivo portable y el checksum');
    const archiveName = artifactName(version, platform);
    const archivePath = path.join(outputDirectory, archiveName);
    if (isWindows) {
      // Windows: .zip con PowerShell (Expand-Archive nativo).
      // Se comprime el directorio completo (sin `/*`) para que las entradas
      // del ZIP lleven el nombre del directorio como raíz única y el
      // instalador pueda validar una estructura con un solo directorio raíz.
      const zipExe = 'powershell.exe';
      run(
        zipExe,
        [
          '-NoProfile',
          '-NonInteractive',
          '-Command',
          `Compress-Archive -Path "${path.join(outputDirectory, name)}" -DestinationPath "${archivePath}" -Force`,
        ],
        ROOT,
      );
    } else {
      run('tar', ['-czf', archivePath, '-C', outputDirectory, name], ROOT);
    }
    const hash = createHash('sha256')
      .update(await readFile(archivePath))
      .digest('hex');
    await writeFile(`${archivePath}.sha256`, `${hash}  ${archiveName}\n`);

    console.log(`\n✔ Distribución portable: ${bundleDirectory}`);
    console.log(`✔ Archivo: ${archivePath}`);
    console.log(`✔ SHA-256: ${hash}`);
    console.log(`  Runtime: Node v${nodeInfo.version} (${nodeInfo.source})`);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
}

function parseArguments(argv) {
  const options = { nodeSource: 'system', nodeVersion: undefined };

  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === '--help' || argument === '-h') {
      printHelp();
      process.exit(0);
    }
    if (argument.startsWith('--node-source=')) {
      options.nodeSource = argument.slice('--node-source='.length);
      continue;
    }
    if (argument === '--node-source') {
      options.nodeSource = nextValue(argv, index, '--node-source');
      index += 1;
      continue;
    }
    if (argument.startsWith('--node-version=')) {
      options.nodeVersion = argument.slice('--node-version='.length);
      continue;
    }
    if (argument === '--node-version') {
      options.nodeVersion = nextValue(argv, index, '--node-version');
      index += 1;
      continue;
    }
    throw new Error(`Opción desconocida: ${argument} (usa --help)`);
  }

  if (!['official', 'system'].includes(options.nodeSource)) {
    throw new Error('--node-source debe ser "official" o "system"');
  }
  return options;
}

function nextValue(argv, index, option) {
  const value = argv[index + 1];
  if (value === undefined || value.startsWith('--')) {
    throw new Error(`Falta el valor de ${option}`);
  }
  return value;
}

function printHelp() {
  console.log(`Uso: npm run standalone -- [opciones]

Opciones:
  --node-source official   descarga y verifica el Node oficial (obligatorio en CI)
  --node-source system     usa el Node que ejecuta el script (desarrollo local)
  --node-version <x.y.z>   versión exacta del runtime oficial
  --help                   muestra esta ayuda`);
}

async function prepareRuntime(options, bundleDirectory) {
  const binaryName = process.platform === 'win32' ? 'node.exe' : 'node';
  const destination = path.join(bundleDirectory, binaryName);

  if (options.nodeSource === 'official') {
    const version = options.nodeVersion ?? (await readFile(NODE_VERSION_FILE, 'utf8')).trim();
    const runtime = await fetchNodeRuntime({
      cacheRoot: path.join(ROOT, 'build', 'cache', 'node'),
      version,
    });
    await copyFile(runtime.binaryPath, destination);
    await chmod(destination, 0o755);
    return {
      license: await readFile(runtime.licensePath, 'utf8'),
      source: 'official',
      version: runtime.version,
    };
  }

  await copyFile(process.execPath, destination);
  await chmod(destination, 0o755);
  return {
    license: undefined,
    source: 'system',
    version: process.version.replace(/^v/, ''),
  };
}

function createPosixLauncher(packagePath) {
  return [
    '#!/bin/sh',
    'set -eu',
    'HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)',
    `exec "$HERE/node" "$HERE/node_modules/${packagePath}/dist/cli.js" "$@"`,
    '',
  ].join('\n');
}

/**
 * Crea un launcher .bat para Windows que apunta al Node incluido y al CLI.
 */
function createWindowsLauncher(packagePath) {
  return (
    [
      '@echo off',
      'setlocal',
      'set HERE=%~dp0',
      `set NODE_CMD="%HERE%node.exe"`,
      `set CLI_CMD="%HERE%node_modules\\${packagePath.replaceAll('/', '\\')}\\dist\\cli.js"`,
      '%NODE_CMD% %CLI_CMD% %*',
    ].join('\r\n') + '\r\n'
  );
}

function step(message) {
  console.log(`· ${message}`);
}

function run(command, args, cwd) {
  // En Windows, npm es npm.cmd y spawnSync no lo resuelve sin extensión.
  const cmd = process.platform === 'win32' && !path.extname(command) ? `${command}.cmd` : command;
  const result = spawnSync(cmd, args, {
    cwd,
    encoding: 'utf8',
    env: { ...process.env, NO_COLOR: '1' },
    shell: process.platform === 'win32',
    timeout: 300_000,
  });

  if (result.error !== undefined) {
    throw result.error;
  }
  if (result.status !== 0) {
    const details = [result.stdout, result.stderr].filter(Boolean).join('\n').trim();
    throw new Error(`Falló \`${command} ${args.join(' ')}\` (código ${result.status})\n${details}`);
  }

  return result;
}

await main();
