/**
 * Utilidad para renderizar imágenes en la terminal en modo ANSI (fallback).
 *
 * Cuando el terminal NO soporta protocolo gráfico nativo (Ghostty, Kitty,
 * WezTerm), se usa el modo símbolos de chafa con
 * `--symbols=sextant` (6 sub-píxeles por carácter, 3× más resolución
 * vertical que medios bloques) y `--dither=none` para colores sólidos
 * puros sin ruido.
 *
 * Si `chafa` no está instalado o la imagen no existe, se devuelve una
 * cadena vacía y el llamante debe mostrar un placeholder.
 *
 * Soporta cancelación mediante `AbortSignal`.
 */

import { execFile } from 'node:child_process';
import { access } from 'node:fs/promises';

/** Tamaño por defecto del póster en la vista de detalles (ancho x alto en caracteres). */
export const DEFAULT_POSTER_SIZE = { width: 22, height: 12 };

/** Protocolo de renderizado detectado. */
export type GraphicsFormat = 'symbols' | 'native';

/** Resultado del renderizado de una imagen en modo ANSI. */
export type RenderResult = {
  /** Salida de chafa (secuencias ANSI). */
  output: string;
  /** Protocolo de renderizado utilizado. Siempre 'symbols' en este módulo. */
  format: GraphicsFormat;
};

/** Entornos de terminal que implementan el protocolo gráfico de Kitty (SGP).
 *
 * iTerm2 y Sixel se excluyen deliberadamente: este proyecto solo transmite
 * mediante su propio encoder Kitty (véase `kittyProtocol.ts`), así que no debe
 * anunciarse soporte para protocolos que no se ejercitan activamente.
 */
const KITTY_TERMINAL_PROGRAMS = ['kitty', 'WezTerm', 'Ghostty'];

/** Comprueba si el terminal actual soporta el protocolo gráfico de Kitty (SGP).
 *
 * Orden de prioridad:
 * 1. Variable de entorno MEDIA_MANAGER_GRAPHICS (forzado explícito a 'native' o 'kitty').
 * 2. GHOSTTY_RESOURCES_DIR (Ghostty).
 * 3. KITTY_WINDOW_ID o TERM_PROGRAM ∈ {kitty, WezTerm, Ghostty} o TERM que los mencione.
 *
 * @returns `true` si se debe usar Kitty SGP; `false` si se debe usar el fallback ANSI.
 */
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

/** Comprueba si `chafa` está disponible en el sistema. */
export async function isChafaAvailable(): Promise<boolean> {
  try {
    await execFilePromise('chafa', ['--version'], { encoding: 'utf-8' });
    return true;
  } catch {
    return false;
  }
}

/**
 * Detecta si el terminal actual soporta protocolo gráfico nativo
 * (Kitty SGP, usado por Ghostty, Kitty, WezTerm, etc.).
 *
 * Orden de prioridad:
 * 1. Variable de entorno MEDIA_MANAGER_GRAPHICS (forzado explícito)
 * 2. GHOSTTY_RESOURCES_DIR (Ghostty)
 * 3. KITTY_WINDOW_ID o TERM_PROGRAM=Kitty/WezTerm/Ghostty (Kitty SGP)
 *
 * Devuelve 'native' si se detecta soporte nativo, 'symbols' en caso contrario.
 */
export function detectNativeFormat(): GraphicsFormat {
  // Solo Kitty SGP se considera "native": iTerm2 y Sixel no se implementan
  // activamente y no deben anunciarse como soportados.
  return supportsKittyGraphics() ? 'native' : 'symbols';
}

/**
 * Convierte una imagen a arte Unicode con colores ANSI usando `chafa`.
 *
 * Usa `--format=symbols` con `--symbols=sextant` para bloques de
 * sextante (6 sub-píxeles por carácter, 3× más resolución vertical que
 * medios bloques), `--colors=full` para TrueColor (24-bit),
 * `--dither=none` para evitar puntuado aleatorio, y `--stretch` junto a
 * `--size=WxH` para forzar que chafa ocupe el 100% del canvas del Box
 * contenedor.
 *
 * El resultado se limpia con `.trimEnd()` para eliminar saltos de línea o
 * retornos de carro finales.
 *
 * Devuelve una cadena de texto con secuencias de escape ANSI que se puede
 * mostrar directamente en la terminal (incluido dentro de un `<Text>` de Ink).
 *
 * Devuelve cadena vacía si `chafa` no está disponible o la imagen no existe.
 *
 * @param signal - `AbortSignal` opcional para cancelar la ejecución de chafa.
 */
export async function renderImageToAnsi(
  imagePath: string,
  options: { width?: number; height?: number; signal?: AbortSignal } = {},
): Promise<RenderResult> {
  const {
    width = DEFAULT_POSTER_SIZE.width,
    height = DEFAULT_POSTER_SIZE.height,
    signal,
  } = options;

  try {
    await access(imagePath);
  } catch {
    return { output: '', format: 'symbols' };
  }

  try {
    const { stdout } = await execFilePromiseWithAbort(
      'chafa',
      [
        '--format=symbols',
        '--colors=full',
        '--symbols=sextant',
        '--dither=none',
        '--stretch',
        '--align=top,left',
        `--size=${width}x${height}`,
        '--animate=false',
        imagePath,
      ],
      { encoding: 'utf-8' },
      signal,
    );
    return { output: stdout.trimEnd(), format: 'symbols' };
  } catch {
    return { output: '', format: 'symbols' };
  }
}

/**
 * Ejecuta `execFile` envuelto en una promesa con soporte para `AbortSignal`.
 * Si se emite 'abort', se mata el proceso hijo y se rechaza la promesa.
 */
function execFilePromiseWithAbort(
  file: string,
  args: string[],
  options: { encoding: string },
  signal?: AbortSignal,
): Promise<{ stdout: string; stderr: string }> {
  return new Promise((resolve, reject) => {
    const child = execFile(file, args, options, (error, stdout, stderr) => {
      if (error) {
        reject(error instanceof Error ? error : new Error('Error ejecutando chafa'));
      } else {
        resolve({ stdout: String(stdout), stderr: String(stderr) });
      }
    });

    if (signal) {
      const onAbort = () => {
        try {
          child.kill('SIGKILL');
        } catch {
          // El proceso ya puede haber terminado.
        }
        reject(new Error('Renderizado cancelado por el usuario.'));
      };

      if (signal.aborted) {
        onAbort();
      } else {
        signal.addEventListener('abort', onAbort, { once: true });
      }
    }
  });
}

/** Ejecuta `execFile` envuelto en una promesa (para facilitar tests). */
function execFilePromise(
  file: string,
  args: string[],
  options: { encoding: string },
): Promise<{ stdout: string; stderr: string }> {
  return new Promise((resolve, reject) => {
    execFile(file, args, options, (error, stdout, stderr) => {
      if (error) {
        reject(error instanceof Error ? error : new Error('Error ejecutando chafa'));
      } else {
        resolve({ stdout: String(stdout), stderr: String(stderr) });
      }
    });
  });
}
