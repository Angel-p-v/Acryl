import { homedir } from 'node:os';
import path from 'node:path';

export { homedir };

/**
 * Expande una ruta de usuario: `~` apunta al home, `~/x` a `home/x`.
 */
export function expandHome(target: string, home: string = homedir()): string {
  if (target === '~') return home;
  if (target.startsWith('~/') || target.startsWith('~\\')) {
    return path.join(home, target.slice(2));
  }
  return target;
}

/**
 * Resuelve una ruta a forma absoluta.
 * Las rutas relativas se resuelven contra `baseDir` (por defecto, el directorio actual).
 */
export function resolvePath(target: string, baseDir: string = process.cwd()): string {
  const expanded = expandHome(target);
  return path.isAbsolute(expanded) ? path.normalize(expanded) : path.resolve(baseDir, expanded);
}
