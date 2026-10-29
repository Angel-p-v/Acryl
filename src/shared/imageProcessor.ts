/** Píxeles que ocupa una celda de terminal (ancho x alto). */
export const CELL_PIXELS: { readonly width: number; readonly height: number } = {
  width: 16,
  height: 32,
} as const;

/** Dimensiones de una imagen en píxeles. */
export type ImageDimensions = {
  readonly widthPx: number;
  readonly heightPx: number;
};

/**
 * Dimensiones de visualización en celdas de terminal, calculadas preservando
 * el aspect ratio original de la imagen.
 */
export type DisplayDimensions = {
  readonly cols: number;
  readonly rows: number;
};

/**
 * Calcula las dimensiones en píxeles a partir de celdas de terminal.
 *
 * Cada celda de la terminal se mapea a 16x32 píxeles, la relación típica de
 * un carácter (ancho x alto en píxeles) para renderizar imágenes nativas
 * con el protocolo Kitty.
 *
 * @param cols - Número de columnas (celdas) de ancho.
 * @param rows - Número de filas (celdas) de alto.
 * @returns Dimensiones en píxeles.
 */
export function computePixelDimensions(cols: number, rows: number): ImageDimensions {
  return {
    widthPx: Math.max(CELL_PIXELS.width, Math.trunc(cols * CELL_PIXELS.width)),
    heightPx: Math.max(CELL_PIXELS.height, Math.trunc(rows * CELL_PIXELS.height)),
  };
}

/**
 * Calcula las dimensiones de visualización en celdas de terminal preservando
 * el aspect ratio de la imagen original.
 *
 * Si la imagen es más ancha que alta, se ajusta al ancho disponible y se
 * calcula la altura proporcional. Si es más alta que ancha, se ajusta al
 * alto disponible. Nunca se distorsiona la imagen.
 *
 * @param imageWidthPx - Ancho original de la imagen en píxeles.
 * @param imageHeightPx - Alto original de la imagen en píxeles.
 * @param maxCols - Máximo de columnas disponibles en la terminal.
 * @param maxRows - Máximo de filas disponibles en la terminal.
 * @returns Dimensiones en celdas que preservan el aspect ratio.
 */
export function computeDisplayDimensions(
  imageWidthPx: number,
  imageHeightPx: number,
  maxCols: number,
  maxRows: number,
): DisplayDimensions {
  const aspectRatio = imageWidthPx / imageHeightPx;

  // Ajustar al ancho disponible.
  let cols = maxCols;
  let rows = Math.trunc(cols / aspectRatio);

  // Si no cabe en el alto disponible, ajustar al alto.
  if (rows > maxRows) {
    rows = maxRows;
    cols = Math.trunc(rows * aspectRatio);
  }

  return {
    cols: Math.max(1, cols),
    rows: Math.max(1, rows),
  };
}

/**
 * Procesa una imagen con Sharp: la redimensiona, la convierte a PNG y
 * devuelve el resultado como `Buffer`.
 *
 * Sharp se importa de forma perezosa dentro de esta función para que la
 * CLI headless (que nunca renderiza un póster) no cargue la librería nativa
 * ni emita advertencias de libvips en `stderr`.
 *
 * Se usa `fit: 'contain'` para preservar el aspect ratio original sin
 * estirar la imagen. `kernel: lanczos3` para máxima nitidez al escalar.
 *
 * @param imagePath - Ruta local al fichero de imagen.
 * @param dimensions - Dimensiones objetivo en píxeles.
 * @returns Buffer con la imagen procesada en formato PNG.
 */
export async function processImage(
  imagePath: string,
  dimensions: ImageDimensions,
): Promise<Buffer> {
  const sharpModule = await import('sharp');
  const sharpFn = sharpModule.default;
  return sharpFn(imagePath)
    .resize(dimensions.widthPx, dimensions.heightPx, {
      fit: 'contain',
      kernel: sharpFn.kernel.lanczos3,
      background: { r: 0, g: 0, b: 0, alpha: 0 },
    })
    .toFormat('png')
    .toBuffer();
}
