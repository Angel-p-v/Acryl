import { homedir } from 'node:os';
import path from 'node:path';

/** Devuelve el directorio de configuración del usuario respetando XDG. */
export function configHome(env: NodeJS.ProcessEnv = process.env): string {
  const xdg = env.XDG_CONFIG_HOME;
  return xdg && path.isAbsolute(xdg) ? xdg : path.join(homedir(), '.config');
}

/** Devuelve el directorio de datos del usuario respetando XDG. */
export function dataHome(env: NodeJS.ProcessEnv = process.env): string {
  const xdg = env.XDG_DATA_HOME;
  return xdg && path.isAbsolute(xdg) ? xdg : path.join(homedir(), '.local', 'share');
}

/** Devuelve el directorio de estado del usuario respetando XDG. */
export function stateHome(env: NodeJS.ProcessEnv = process.env): string {
  const xdg = env.XDG_STATE_HOME;
  return xdg && path.isAbsolute(xdg) ? xdg : path.join(homedir(), '.local', 'state');
}

/** Devuelve el directorio de caché del usuario respetando XDG. */
export function cacheHome(env: NodeJS.ProcessEnv = process.env): string {
  const xdg = env.XDG_CACHE_HOME;
  return xdg && path.isAbsolute(xdg) ? xdg : path.join(homedir(), '.cache');
}

/** Carpeta que contiene la configuración de media-manager. */
export function defaultConfigDir(env: NodeJS.ProcessEnv = process.env): string {
  return path.join(configHome(env), 'media-manager');
}

/** Ruta por defecto del archivo de configuración. */
export function defaultConfigPath(env: NodeJS.ProcessEnv = process.env): string {
  return path.join(defaultConfigDir(env), 'config.yaml');
}

/** Carpeta que contiene los datos persistidos (base de datos, caché). */
export function defaultDataDir(env: NodeJS.ProcessEnv = process.env): string {
  return path.join(dataHome(env), 'media-manager');
}

/** Ruta por defecto de la base de datos. */
export function defaultDatabasePath(env: NodeJS.ProcessEnv = process.env): string {
  return path.join(defaultDataDir(env), 'library.db');
}
