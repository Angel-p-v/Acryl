import { existsSync, mkdirSync } from 'node:fs';
import { writeFile } from 'node:fs/promises';
import path from 'node:path';

import pc from 'picocolors';
import { stringify as stringifyYaml } from 'yaml';

import { defaultConfig } from '../core/config/defaults.js';
import { loadConfig, resolveConfigPath, saveConfig } from '../core/config/loader.js';
import { ValidationError } from '../shared/errors.js';

export type ConfigCommandOptions = {
  path?: boolean;
  validate?: boolean;
  json?: boolean;
  init?: boolean;
  config?: string;
  /** Establece una clave anidada, p. ej. `player.default=vlc`. */
  set?: string;
};

/** Asigna un valor en un objeto anidado usando una ruta con puntos (p. ej. `player.default`). */
function setNestedValue(obj: Record<string, unknown>, dottedKey: string, value: unknown): void {
  const keys = dottedKey.split('.');
  const lastKey = keys.at(-1);
  if (lastKey === undefined) {
    throw new ValidationError(`Clave vacía: '${dottedKey}'.`);
  }

  let current: Record<string, unknown> = obj;
  let pathSoFar = '';
  for (const key of keys.slice(0, -1)) {
    pathSoFar = pathSoFar ? `${pathSoFar}.${key}` : key;
    const next: Record<string, unknown> = current[key] as Record<string, unknown>;
    if (next === null || next === undefined || typeof next !== 'object' || Array.isArray(next)) {
      throw new ValidationError(
        `No se puede establecer '${dottedKey}': la ruta '${pathSoFar}' no es un objeto.`,
      );
    }
    current = next;
  }
  current[lastKey] = value;
}

/** Coerciona el valor de cadena de `--set` a booleano, número o cadena. */
function coerceValue(value: string): string | boolean | number {
  if (value === 'true') return true;
  if (value === 'false') return false;
  if (value !== '' && !Number.isNaN(Number(value))) return Number(value);
  return value;
}

/** Implementa `mm config`. */
export async function runConfigCommand(options: ConfigCommandOptions): Promise<void> {
  const configPath =
    options.config === undefined
      ? resolveConfigPath()
      : resolveConfigPath({ configPath: options.config });

  if (options.path) {
    const exists = existsSync(configPath);
    if (exists) {
      console.log(configPath);
    } else {
      console.log(`${configPath} (no existe, se usará por defecto)`);
    }
    return;
  }

  if (options.init) {
    const configDir = path.dirname(configPath);
    mkdirSync(configDir, { recursive: true });
    const yaml = stringifyYaml(defaultConfig);
    await writeFile(configPath, yaml, 'utf8');
    console.log(pc.green('✔') + ` Configuración creada en ${configPath}`);
    return;
  }

  if (options.set !== undefined) {
    const equals = options.set.indexOf('=');
    if (equals <= 0) {
      throw new ValidationError(
        `Opción --set inválida: '${options.set}'. Usa el formato clave=valor, p. ej. player.default=vlc.`,
      );
    }
    const key = options.set.slice(0, equals);
    const rawValue = options.set.slice(equals + 1);

    const config = await loadConfig({ configPath });
    setNestedValue(config, key, coerceValue(rawValue));
    await saveConfig(config, configPath);
    console.log(pc.green('✔') + ` ${key} = ${rawValue}`);
    return;
  }

  const exists = existsSync(configPath);

  if (options.validate) {
    await loadConfig({ configPath });
    console.log(
      pc.green('✔') +
        ` Configuración válida${exists ? '' : pc.dim(' (se usarán los valores por defecto)')}`,
    );
    return;
  }

  const config = await loadConfig({ configPath });

  if (options.json) {
    console.log(
      JSON.stringify({ configPath, source: exists ? configPath : 'defaults', config }, null, 2),
    );
    return;
  }

  console.log(pc.bold(pc.cyan('Configuración efectiva')));
  console.log(pc.dim(`  archivo: ${exists ? configPath : 'sin archivo (valores por defecto)'}`));
  console.log('');
  console.log(pc.bold('  biblioteca'));
  if (config.library.roots.length === 0) {
    console.log(pc.dim('    (sin raíces configuradas)'));
  }
  for (const root of config.library.roots) {
    console.log(`    ${pc.green('•')} ${root.path} ${pc.dim(`[${root.type}]`)}`);
  }
  console.log('');
  console.log(pc.bold('  base de datos'));
  console.log(`    ${config.database.path}`);
  console.log('');
  console.log(pc.bold('  reproductor'));
  console.log(`    por defecto: ${config.player.default}`);
  console.log(`    disponibles: ${Object.keys(config.player.players).join(', ') || '(ninguno)'}`);
  console.log('');
  console.log(pc.bold('  exclusiones'));
  for (const pattern of config.library.exclude) {
    console.log(`    ${pc.dim(pattern)}`);
  }
  console.log('');
  console.log(pc.dim(`  atajos: ${Object.keys(config.keybindings).length} definidos`));
}
