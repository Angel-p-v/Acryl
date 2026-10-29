/**
 * Detección de capacidades gráficas de la terminal.
 *
 * Esta utilidad centraliza la detección de si la terminal soporta el
 * Protocolo Gráfico de Kitty (SGP) y si `chafa` está disponible para el
 * modo fallback ANSI.
 *
 * La detección es conservadora: no se anuncia soporte para Kitty SGP salvo
 * que existan indicios claros de que el terminal lo implementa (Ghostty,
 * Kitty, WezTerm). Las variables de entorno solas no son suficiente prueba
 * para evitar falsos positivos que corromperían el renderizado de Ink.
 */

export type GraphicsCapability = 'none' | 'symbols' | 'native';

/** Entornos de terminal que implementan el protocolo gráfico de Kitty (SGP). */
const KITTY_TERMINAL_PROGRAMS = ['kitty', 'WezTerm', 'Ghostty'];

/** Comprueba si el terminal actual soporta el protocolo gráfico de Kitty (SGP). */
export function supportsKittyGraphics(): boolean {
  const forced = process.env.MEDIA_MANAGER_GRAPHICS ?? '';
  const forcedLower = forced.toLowerCase();
  if (forcedLower === 'native' || forcedLower === 'kitty') return true;
  if (forcedLower === 'symbols' || forcedLower === 'ansi' || forcedLower === 'none') return false;

  const ghosttyResources = process.env.GHOSTTY_RESOURCES_DIR ?? '';
  const termProgram = process.env.TERM_PROGRAM ?? '';
  const kittyWindowId = process.env.KITTY_WINDOW_ID ?? '';
  const term = process.env.TERM ?? '';

  if (ghosttyResources) return true;
  if (KITTY_TERMINAL_PROGRAMS.includes(termProgram)) return true;
  if (kittyWindowId) return true;
  if (term.toLowerCase().includes('ghostty')) return true;
  if (term.toLowerCase().includes('kitty')) return true;

  return false;
}

/**
 * Detecta la capacidad gráfica del terminal actual.
 *
 * Devuelve 'native' solo cuando se detecta soporte claro para Kitty SGP
 * (Ghostty, Kitty, WezTerm o forzado explícito). En cualquier otro caso
 * devuelve 'symbols', que se renderiza mediante chafa dentro del layout de
 * Ink sin riesgos de descolocamiento.
 */
export function detectGraphicsCapability(): GraphicsCapability {
  return supportsKittyGraphics() ? 'native' : 'symbols';
}

/** Comprueba si `chafa` está disponible en el sistema. */
export async function isChafaAvailable(): Promise<boolean> {
  const { execFile } = await import('node:child_process');
  try {
    await new Promise<void>((resolve, reject) => {
      execFile('chafa', ['--version'], { encoding: 'utf-8' }, (error) => {
        if (error)
          reject(new Error(error instanceof Error ? error.message : 'chafa no disponible'));
        else resolve();
      });
    });
    return true;
  } catch {
    return false;
  }
}
