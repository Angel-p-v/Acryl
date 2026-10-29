#!/usr/bin/env node
import process from 'node:process';

import { Command, CommanderError } from 'commander';

import { generateCompletion } from './commands/completion.js';
import { runConfigCommand } from './commands/config.js';
import { runContinueCommand } from './commands/continue.js';
import {
  runDbBackupCommand,
  runDbOptimizeCommand,
  runDbRestoreCommand,
  runDbStatsCommand,
} from './commands/db.js';
import { runDoctorCommand } from './commands/doctor.js';
import { reportFatalError } from './commands/errorHandler.js';
import { runFavoriteCommand } from './commands/favorite.js';
import { runHistoryCommand } from './commands/history.js';
import { runIndexCommand } from './commands/index.js';
import { runListCommand } from './commands/list.js';
import { runMarkCommand } from './commands/mark.js';
import { runMetadataCommand } from './commands/metadata.js';
import {
  parseMediaType,
  parseNonNegativeInteger,
  parsePositiveInteger,
  parseSortOrder,
} from './commands/options.js';
import { runPlayCommand } from './commands/play.js';
import { runPlaylistCommand, type PlaylistCommandOptions } from './commands/playlist.js';
import { runSearchCommand } from './commands/search.js';
import { runSelfUpdateCommand } from './commands/self-update.js';
import { runShowCommand } from './commands/show.js';
import { runTagCommand, parseTagOptions } from './commands/tag.js';
import { runWatchCommand } from './commands/watch.js';
import { loadConfig, resolveConfigPath } from './core/config/loader.js';
import { getVersion } from './shared/version.js';
import { runTui, defaultLogFile } from './ui/run.js';

const program = new Command()
  .name('mm')
  .description(
    'Media Manager — gestor de contenido multimedia para terminal (series, anime y películas)',
  )
  .version(getVersion(), '-V, --version', 'muestra la versión')
  .option('-v, --verbose', 'activa logs de depuración y pilas de error')
  .option('--log-file <ruta>', 'escribe los logs en un archivo con rotación automática (10 MB)')
  .showHelpAfterError()
  .addHelpText(
    'before',
    '\nÍndice de comandos:\n  list / search / show   explorar la biblioteca\n  play / continue        reproducir títulos\n  index / watch          mantener la biblioteca actualizada\n  metadata / doctor      metadatos externos y diagnóstico\n  config / tag / favorite  ajustes y etiquetas\n  db / self-update / completion  utilidades avanzadas\n',
  )
  .addHelpText(
    'after',
    '\nEjemplos de uso:\n  mm                              abre la interfaz interactiva (TUI)\n  mm list                         lista todos los títulos indexados\n  mm search "Black Torch"         busca un título por nombre\n  mm show 1                       muestra el detalle del título #1\n  mm play 1                       reproduce el título #1\n  mm index                        indexa la biblioteca completa\n  mm metadata search "Black Torch"busca metadatos externos (TVMaze/Jikan)\n  mm doctor                       diagnostica y repara la biblioteca\n  mm completion bash              genera script de autocompletado para bash\n\nCódigos de salida:\n  0  éxito\n  1  error de ejecución\n  2  uso incorrecto\n',
  )
  .exitOverride();

// Commander conserva las opciones globales en el programa; se copian al subcomando
// para que todos los `run*Command(options)` reciban el mismo `verbose` y `logFile`.
program.hook('preAction', (thisCommand, actionCommand) => {
  const opts = thisCommand.opts();
  if (opts.verbose === true) {
    actionCommand.setOptionValue('verbose', true);
  }
  if (opts.logFile !== undefined) {
    actionCommand.setOptionValue('logFile', opts.logFile);
  }
});

program
  .command('config')
  .description('muestra y edita la configuración efectiva')
  .option('-c, --config <ruta>', 'archivo de configuración a usar')
  .option('--path', 'muestra solo la ruta del archivo de configuración')
  .option('--validate', 'valida el archivo de configuración y termina')
  .option('--json', 'salida en formato JSON')
  .option('--init', 'crea el archivo de configuración por defecto si no existe')
  .option('--set <clave=valor>', 'establece una clave anidada, p. ej. player.default=vlc')
  .action(runConfigCommand);

program
  .command('index')
  .description('indexa la biblioteca (escanea carpetas y añade títulos a la BD)')
  .option('-c, --config <ruta>', 'archivo de configuración a usar')
  .option('--dry-run', 'simula la indexación sin escribir en la base de datos')
  .option('--root <ruta>', 'indexa solo la raíz indicada')
  .option('--folder <ruta>', 'indexa solo la carpeta indicada dentro de la raíz')
  .option('--json', 'salida en formato JSON')
  .action(runIndexCommand);

program
  .command('list')
  .description('enumera los títulos de la biblioteca con filtros y paginación')
  .option('-c, --config <ruta>', 'archivo de configuración a usar')
  .option('-t, --type <tipo>', 'filtra por tipo (series, anime, movie)', parseMediaType)
  .option('--sort <orden>', 'ordena por title, added o recent', parseSortOrder)
  .option('--limit <n>', 'número máximo de resultados', parsePositiveInteger)
  .option('--offset <n>', 'resultados a saltar', parseNonNegativeInteger)
  .option(
    '--tag <id>',
    'filtra por id de etiqueta (puede repetirse)',
    (val: string, prev: number[] = []) => [...prev, Number(val)],
    [],
  )
  .option('--json', 'salida en formato JSON')
  .option('--export <formato>', 'exporta a archivo: json o csv')
  .action(runListCommand);

program
  .command('search')
  .description('busca títulos por texto en la biblioteca indexada')
  .argument('<consulta>', 'texto a buscar')
  .option('-c, --config <ruta>', 'archivo de configuración a usar')
  .option('-t, --type <tipo>', 'filtra por tipo (series, anime, movie)', parseMediaType)
  .option('--limit <n>', 'número máximo de resultados', parsePositiveInteger)
  .option('--json', 'salida en formato JSON')
  .action(runSearchCommand);

program
  .command('show')
  .description('muestra el detalle completo de un título (metadatos, ficheros, póster)')
  .argument('<id-o-titulo>', 'id numérico o título a consultar')
  .option('-c, --config <ruta>', 'archivo de configuración a usar')
  .option('--json', 'salida en formato JSON')
  .action(runShowCommand);

program
  .command('play')
  .description('reproduce un título con el reproductor configurado (mpv/vlc)')
  .argument('<id-o-titulo>', 'id numérico o título a reproducir')
  .option('-c, --config <ruta>', 'archivo de configuración a usar')
  .option('-f, --file <id>', 'id del fichero a reproducir', parsePositiveInteger)
  .option('-p, --player <nombre>', 'reproductor alternativo al configurado')
  .option('--foreground', 'espera a que el reproductor termine')
  .option('--dry-run', 'muestra el comando sin lanzar el reproductor')
  .option('--json', 'salida en formato JSON')
  .action(runPlayCommand);

program
  .command('playlist <titulos...>')
  .description('reproduce varios títulos en secuencia (playlist)')
  .option('-c, --config <ruta>', 'archivo de configuración a usar')
  .option('--dry-run', 'muestra qué se reproduciría sin lanzar nada')
  .option('--json', 'salida en formato JSON')
  .action((rawOptions: Record<string, unknown>, titles: string[]) => {
    const options = {
      ...rawOptions,
      _: titles,
    };
    return runPlaylistCommand(titles, options as PlaylistCommandOptions);
  });

program
  .command('continue')
  .description('lista títulos en curso (reproducción pendiente)')
  .option('-c, --config <ruta>', 'archivo de configuración a usar')
  .option('-l, --limit <n>', 'máximo de resultados', parsePositiveInteger)
  .option('--json', 'salida en formato JSON')
  .action(runContinueCommand);

program
  .command('history')
  .description('muestra el historial reciente de reproducciones')
  .option('-c, --config <ruta>', 'archivo de configuración a usar')
  .option('-l, --limit <n>', 'máximo de resultados', parsePositiveInteger)
  .option('--json', 'salida en formato JSON')
  .action(runHistoryCommand);

program
  .command('mark')
  .description('marca estado visto/no visto o posición de reproducción de un título')
  .argument('<id-o-titulo>', 'id numérico o título a marcar')
  .option('-c, --config <ruta>', 'archivo de configuración a usar')
  .option('-f, --file <id>', 'id del fichero (requerido si hay varios)', parsePositiveInteger)
  .option('--watched', 'marca como visto')
  .option('--unwatched', 'marca como no visto')
  .option('--position <segundos>', 'guarda posición concreta', parseNonNegativeInteger)
  .option('--json', 'salida en formato JSON')
  .action(runMarkCommand);

program
  .command('favorite')
  .description('marca o desmarca un título como favorito')
  .argument('<id-o-titulo>', 'id numérico o título')
  .option('-c, --config <ruta>', 'archivo de configuración a usar')
  .option('--off', 'desmarca el favorito')
  .option('--json', 'salida en formato JSON')
  .action(runFavoriteCommand);

program
  .command('tag')
  .description('gestiona etiquetas y su asignación a títulos')
  .option('-c, --config <ruta>', 'archivo de configuración a usar')
  .option('-a, --action <add|remove|list>', 'acción: add, remove o list')
  .option('-m, --media <id>', 'id del título (requerido para add/remove)')
  .option('-t, --tag <nombre>', 'nombre de la etiqueta (requerido para add/remove)')
  .option('--json', 'salida en formato JSON')
  .action((rawOptions: Record<string, unknown>) => {
    const parsed = parseTagOptions(rawOptions);
    // parseTagOptions retorna null si hay error de validación (ya estableció exitCode = 2)
    if (parsed === null) {
      return;
    }
    return runTagCommand(parsed);
  });

program
  .command('doctor')
  .description('diagnóstico y reparación de la biblioteca (índice, metadatos, pósters)')
  .option('-c, --config <ruta>', 'archivo de configuración a usar')
  .option('--json', 'salida en formato JSON')
  .option('--full-hash', 'hash completo para confirmar duplicados')
  .option('--prune-missing', 'elimina del índice los ficheros ausentes')
  .option('-y, --yes', 'no pide confirmación para --prune-missing')
  .action(runDoctorCommand);

program
  .command('watch')
  .description('vigila las raíces y reindexa al detectar cambios')
  .option('-c, --config <ruta>', 'archivo de configuración a usar')
  .action(runWatchCommand);

program
  .command('self-update')
  .description('busca e instala actualizaciones desde GitHub Releases')
  .option('-c, --config <ruta>', 'archivo de configuración a usar')
  .option('--check', 'solo comprueba si hay actualización disponible')
  .option('--dry-run', 'muestra qué se descargaría sin instalar')
  .action(runSelfUpdateCommand);

program
  .command('metadata')
  .description('gestiona metadatos externos (TVMaze para series/películas, Jikan para anime)')
  .option('-c, --config <ruta>', 'archivo de configuración a usar')
  .option('-a, --action <fetch|search|clear|cache>', 'acción: fetch, search, clear o cache')
  .option('-m, --media-id <id>', 'id del título (requerido para fetch/clear)')
  .option('-q, --query <texto>', 'consulta de búsqueda (requerido para search)')
  .option('-p, --provider <tvmaze|jikan>', 'proveedor de metadatos (tvmaze o jikan)')
  .option('-e, --external-id <id>', 'id externo (requerido para fetch)')
  .option('--json', 'salida en formato JSON')
  .action(runMetadataCommand);

program
  .command('completion <shell>')
  .description('genera script de autocompletado para shell (bash, zsh, fish)')
  .action((shell: string) => {
    try {
      console.log(generateCompletion(shell));
    } catch (error: unknown) {
      console.error((error as Error).message);
      process.exitCode = 1;
    }
  });

const dbCommand = program.command('db').description('operaciones sobre la base de datos local');

dbCommand
  .command('stats')
  .description('muestra el tamaño y el contenido del índice')
  .option('-c, --config <ruta>', 'archivo de configuración a usar')
  .option('--json', 'salida en formato JSON')
  .action(runDbStatsCommand);

dbCommand
  .command('optimize')
  .description('actualiza estadísticas y compacta la base de datos')
  .option('-c, --config <ruta>', 'archivo de configuración a usar')
  .option('--vacuum', 'compacta el fichero (puede tardar)')
  .option('--json', 'salida en formato JSON')
  .action(runDbOptimizeCommand);

dbCommand
  .command('backup')
  .description('crea una copia de seguridad de la base de datos')
  .option('-c, --config <ruta>', 'archivo de configuración a usar')
  .option('--label <etiqueta>', 'etiqueta para identificar el backup', 'manual')
  .option('--json', 'salida en formato JSON')
  .action(runDbBackupCommand);

dbCommand
  .command('restore')
  .description('restaura la base de datos desde un fichero de backup')
  .option('-c, --config <ruta>', 'archivo de configuración a usar')
  .option('--from <ruta>', 'ruta al fichero de backup a restaurar', '')
  .option('--json', 'salida en formato JSON')
  .action(runDbRestoreCommand);

async function main(): Promise<void> {
  // Sin subcomando se abre la interfaz interactiva; con subcomando, la CLI headless.
  const rawArgs = process.argv.slice(2);
  const verbose = rawArgs.includes('--verbose') || rawArgs.includes('-v');
  const logFile = verbose ? defaultLogFile() : undefined;
  const args = rawArgs.filter((arg) => arg !== '--verbose' && arg !== '-v');
  if (args.length === 0) {
    const configPath = resolveConfigPath();
    const config = await loadConfig({ configPath });
    const tuiOptions: { verbose?: boolean; logFile?: string } = { verbose };
    if (logFile !== undefined) {
      tuiOptions.logFile = logFile;
    }
    await runTui(config, configPath, tuiOptions);
    return;
  }

  try {
    await program.parseAsync(process.argv);
  } catch (error) {
    // Commander ya ha impreso la ayuda, la versión o el error de uso. Solo `help`
    // y `--version` salen con 0; el resto son errores de uso (2).
    if (error instanceof CommanderError) {
      process.exitCode = error.exitCode === 0 ? 0 : 2;
      return;
    }
    throw error;
  }
}

main().catch((error: unknown) => {
  reportFatalError(error, { verbose: program.opts().verbose === true });
  process.exitCode = 1;
});
