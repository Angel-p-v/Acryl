/**
 * Codificador crudo del Protocolo Gráfico de Kitty (Kitty Graphics Protocol, SGP).
 *
 * Recibe el `Buffer` de salida de Sharp (ya en formato PNG), lo convierte a
 * Base64 y lo fragmenta en bloques de 4096 bytes (requerido por el protocolo).
 * Genera la secuencia de escape exacta:
 *
 *   \x1b_Ga=T,f=100,c=<cols>,r=<filas>,i=<ID>,z=1,m=<0|1>;<base64>\x1b\\
 *
 * `m=1` indica que siguen más fragmentos; `m=0` es el último.
 *
 * Esta utilidad es pura y no depende de `terminal-image`, `chafa`,
 * `jimp`, `ink-picture` ni de ningún subproceso externo. Nunca lee ficheros
 * del disco: el llamante ya ha procesado la imagen con Sharp.
 *
 * Nunca lanza: si el buffer está vacío, retorna un array vacío.
 *
 * Referencia: https://sw.kovidgoyal.net/kitty/graphics-protocol/
 */

export const CHUNK_SIZE = 4096;

/** Formato PNG (valor de `f` del protocolo Kitty) que emite Sharp. */
export const FORMAT_PNG = 100;

/** Parámetros de entrada para codificar un Buffer al protocolo Kitty. */
export type KittyEncodeOptions = {
  /** Buffer de la imagen (procesado por Sharp, formato PNG). */
  buffer: Buffer;
  /** Número de columnas (celdas de terminal) que ocupa el póster. */
  columns: number;
  /** Número de filas (celdas de terminal) que ocupa el póster. */
  rows: number;
  /** Identificador único de la imagen (para borrado selectivo). */
  imageId: number;
};

/** Resultado del codificado: una secuencia de escape Kitty por fragmento. */
export type KittyFrame = {
  /** Secuencia completa de escape Kitty para este fragmento. */
  sequence: string;
  /** Indica si es el último fragmento (m=0). */
  isLast: boolean;
};

/**
 * Codifica un Buffer (imagen PNG de Sharp) en una o más secuencias de escape
 * del Protocolo Gráfico de Kitty.
 *
 * Los datos Base64 se dividen en bloques de como máximo 4096 bytes. Cada
 * fragmento lleva los parámetros completos del protocolo; `m=1` indica que
 * hay más datos y `m=0` que es el último.
 *
 * Nunca lanza: si el buffer está vacío, retorna [].
 *
 * @param options - Buffer, columnas, filas e ID de la imagen.
 * @returns Array de secuencias de escape listas para escribirse a stdout.
 */
export function encodeKittyGraphic(options: KittyEncodeOptions): KittyFrame[] {
  const { buffer, columns, rows, imageId } = options;

  if (!buffer || buffer.length === 0) {
    return [];
  }

  // Codificar a Base64 (estándar RFC 4648, sin saltos de línea).
  const base64: string = buffer.toString('base64');
  if (base64.length === 0) {
    return [];
  }

  // Dividir en fragmentos de como máximo CHUNK_SIZE bytes del Base64.
  const chunks: string[] = [];
  for (let i = 0; i < base64.length; i += CHUNK_SIZE) {
    chunks.push(base64.slice(i, i + CHUNK_SIZE));
  }

  const frames: KittyFrame[] = [];
  const lastIndex: number = chunks.length - 1;

  for (let index = 0; index < chunks.length; index += 1) {
    const chunk: string = chunks[index] ?? '';
    const isLast: boolean = index === lastIndex;
    // m=1 indica que hay más chunks; m=0 es el último.
    const message: string = isLast ? '0' : '1';

    // Todos los fragmentos llevan los parámetros completos; `f=100` (PNG)
    // porque Sharp siempre emite PNG. `c=` y `r=` expresan celdas de
    // terminal, no píxeles (los píxeles se fijan al redimensionar con Sharp).
    const params = `a=T,f=${FORMAT_PNG},t=d,c=${columns},r=${rows},i=${imageId},z=1,m=${message}`;
    const sequence = `\x1b_G${params};${chunk}\x1b\\`;
    frames.push({ sequence, isLast });
  }

  return frames;
}

/**
 * Genera la secuencia de borrado de Kitty para un ID de imagen específico.
 *
 * Formato: ESC _ G a=d,d=i,i=<imageId> ESC \
 *
 * @param imageId - Identificador único de la imagen a borrar.
 * @returns Secuencia de escape de borrado Kitty.
 */
export function deleteKittyImage(imageId: number): string {
  return `\x1b_Ga=d,d,i,i=${imageId}\x1b\\`;
}

/**
 * Genera la secuencia de borrado de todas las imágenes gráficas Kitty.
 *
 * Formato: ESC _ G a=d ESC \
 *
 * @returns Secuencia de escape de borrado de todas las imágenes.
 */
export function deleteAllKittyImages(): string {
  return `\x1b_Ga=d;\x1b\\`;
}
