#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import console from 'node:console';
import { mkdtemp, mkdir, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');

/**
 * Comprueba el paquete real: `npm pack` → instalación limpia en un directorio
 * temporal → ejecución de la CLI instalada. No usa el `dist/` del repositorio,
 * así que detecta ficheros que falten en el tarball o dependencias mal declaradas.
 */
async function main() {
  const packageJson = JSON.parse(await readFile(path.join(ROOT, 'package.json'), 'utf8'));
  const workspace = await mkdtemp(path.join(tmpdir(), 'mm-smoke-'));
  const packDirectory = path.join(workspace, 'pack');
  const installDirectory = path.join(workspace, 'install');
  const libraryDirectory = path.join(workspace, 'library');
  const configPath = path.join(workspace, 'config.yaml');

  try {
    await mkdir(packDirectory, { recursive: true });
    await mkdir(installDirectory, { recursive: true });
    await mkdir(path.join(libraryDirectory, 'Show (2020)'), { recursive: true });
    await writeFile(path.join(libraryDirectory, 'Show (2020)', 'Show.S01E01.mkv'), '');

    step('empaquetando el tarball');
    run('npm', ['pack', '--pack-destination', packDirectory, '--silent'], ROOT);

    const tarballs = (await readdir(packDirectory)).filter((name) => name.endsWith('.tgz'));
    if (tarballs.length !== 1) {
      throw new Error(`Se esperaba un tarball en ${packDirectory} y hay ${tarballs.length}`);
    }
    const tarballPath = path.join(packDirectory, tarballs[0]);

    step('instalando el tarball en un proyecto limpio');
    await writeFile(
      path.join(installDirectory, 'package.json'),
      `${JSON.stringify(
        {
          name: 'media-manager-smoke',
          private: true,
          allowScripts: { 'better-sqlite3': true },
        },
        null,
        2,
      )}\n`,
    );
    run('npm', ['install', '--no-audit', '--no-fund', '--omit=dev', tarballPath], installDirectory);

    await writeFile(
      configPath,
      [
        'library:',
        '  roots:',
        `    - path: ${yamlString(libraryDirectory)}`,
        '      type: series',
        'database:',
        `  path: ${yamlString(path.join(workspace, 'library.db'))}`,
        '',
      ].join('\n'),
    );

    const packageDirectory = path.join(
      installDirectory,
      'node_modules',
      ...packageJson.name.split('/'),
    );
    const cliPath = path.join(packageDirectory, 'dist', 'cli.js');

    step('ejecutando --version');
    const version = run(process.execPath, [cliPath, '--version'], installDirectory).stdout.trim();
    if (version !== packageJson.version) {
      throw new Error(`Versión inesperada: ${version} (esperada ${packageJson.version})`);
    }

    step('validando la configuración');
    const validation = run(
      process.execPath,
      [cliPath, 'config', '--validate', '--config', configPath],
      installDirectory,
    ).stdout;
    if (!validation.includes('Configuración válida')) {
      throw new Error(`Salida inesperada de config --validate: ${validation.trim()}`);
    }

    step('simulando la indexación');
    const indexed = run(
      process.execPath,
      [cliPath, 'index', '--dry-run', '--json', '--config', configPath],
      installDirectory,
    ).stdout;
    const stats = JSON.parse(indexed);
    if (stats.dryRun !== true || stats.filesSeen !== 1) {
      throw new Error(`Estadísticas inesperadas de index --dry-run: ${indexed.trim()}`);
    }

    console.log(`\n✔ Smoke del tarball correcto (${tarballs[0]})`);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
}

function step(message) {
  console.log(`· ${message}`);
}

function run(command, args, cwd) {
  const result = spawnSync(command, args, {
    cwd,
    encoding: 'utf8',
    env: { ...process.env, NO_COLOR: '1' },
    timeout: 180_000,
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

function yamlString(value) {
  return `'${value.replaceAll("'", "''")}'`;
}

await main();
