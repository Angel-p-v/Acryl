import sharp from 'sharp';

/** Opciones para convertir una imagen a bloques de color ANSI truecolor. */
export type ImageToAnsiOptions = {
  /** Ruta local al fichero de imagen. */
  readonly imagePath: string;
  /** Ancho del área en celdas de terminal. */
  readonly width: number;
  /** Alto del área en celdas de terminal. */
  readonly height: number;
};

/** Resultado de la conversión: líneas de texto con secuencias ANSI truecolor. */
export type AnsiArtResult = {
  /** Líneas de texto con códigos de color ANSI truecolor. */
  readonly lines: string[];
  /** Ancho en caracteres de la salida. */
  readonly width: number;
  /** Alto en caracteres de la salida. */
  readonly height: number;
};

/**
 * Convierte una imagen a bloques de color ANSI truecolor para renderizar
 * dentro de un componente de Ink.
 *
 * Sharp lee la imagen, la redimensiona al ancho y alto indicados (en
 * píxeles, mapeados 1:1 a celdas de terminal) rellenando completamente el
 * área con `fit: 'cover'` (recortando si es necesario para preservar el
 * aspect ratio sin dejar padding), y extrae el color RGB de cada píxel.
 * Cada píxel se renderiza como un carácter de bloque lleno (`█`) con el
 * color del píxel como color de fondo ANSI truecolor
 * (`\x1b[48;2;r;g;bm`).
 *
 * El resultado es un array de líneas que puede renderizarse línea a línea
 * dentro de `<Text>` de Ink, por lo que el póster queda exactamente dentro
 * de su `<Box>` y en la posición que el layout de Ink le asigne.
 *
 * Nunca lanza: si la imagen no se puede leer o procesar, retorna un array
 * vacío para que el llamante muestre un placeholder.
 *
 * @param options - Ruta, ancho y alto en celdas de terminal.
 * @returns Líneas con bloques de color ANSI truecolor (puede estar vacío).
 */
export async function imageToAnsi(options: ImageToAnsiOptions): Promise<AnsiArtResult> {
  const { imagePath, width, height } = options;

  let data: Buffer;
  try {
    const result = await sharp(imagePath)
      .resize(width, height, {
        fit: 'cover',
        kernel: sharp.kernel.lanczos3,
        position: 'center',
      })
      .raw()
      .toBuffer({ resolveWithObject: true });
    data = result.data;
  } catch {
    return { lines: [], width: 0, height: 0 };
  }

  // Sharp con fit: 'cover' devuelve exactamente width×height píxeles sin
  // padding. Usamos width directamente para el índice, que es la dimensión
  // real del buffer de salida.
  const channels = 3;
  const lines: string[] = [];
  for (let y = 0; y < height; y += 1) {
    const rowParts: string[] = [];
    for (let x = 0; x < width; x += 1) {
      const idx: number = (y * width + x) * channels;
      if (idx + 2 >= data.length) {
        rowParts.push(' ');
        continue;
      }

      const r: number = data[idx] ?? 0;
      const g: number = data[idx + 1] ?? 0;
      const b: number = data[idx + 2] ?? 0;

      // Carácter de bloque lleno con color de fondo truecolor del píxel.
      rowParts.push(`\x1b[48;2;${r};${g};${b}m█\x1b[0m`);
    }
    lines.push(rowParts.join(''));
  }

  return { lines, width, height };
}
