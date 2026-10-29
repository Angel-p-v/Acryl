/**
 * Componente que renderiza un póster de imagen en la terminal.
 *
 * El póster se renderiza SIEMPRE dentro del flujo de Ink (dentro de un
 * `<Box>`), de modo que queda exactamente en la posición que el layout de
 * Ink le asigne. No se escriben secuencias gráficas fuera de Ink que
 * pudieran descolocarse al redibujar.
 *
 * Modos de renderizado:
 * - 'auto' (por defecto): usa Sharp para convertir la imagen a bloques de
 *   color ANSI truecolor dentro del Box de Ink. Buena calidad (colores
 *   reales 24-bit) y posicionamiento perfecto en cualquier terminal.
 * - 'symbols': usa Chafa dentro del Box de Ink. Posicionamiento perfecto,
 *   compatible con cualquier terminal, pero calidad limitada (arte ANSI).
 * - 'native': usa el Protocolo Gráfico de Kitty como overlay directo sobre
 *   stdout. Máxima calidad en Ghostty/Kitty/WezTerm, pero el posicionamiento
 *   es aproximado (esquina basada en `placement`) porque Ink no expone las
 *   coordenadas físicas de pantalla de sus componentes.
 *
 * Cada vista controla la posición mediante la estructura de componentes
 * (dónde coloca el <PosterImage> en el árbol de React), no mediante
 * coordenadas absolutas de terminal.
 *
 * Ningún mensaje de depuración se escribe a stdout/stderr.
 */

import { Box, Text, useStdout } from 'ink';
import React, { useCallback, useEffect, useRef, useState } from 'react';

import { computePixelDimensions, processImage } from '../../shared/imageProcessor.js';
import { imageToAnsi } from '../../shared/imageToAnsi.js';
import { encodeKittyGraphic } from '../../shared/kittyProtocol.js';
import { renderImageToAnsi } from '../../shared/terminalImage.js';
import { theme } from '../theme.js';

export type PosterImageProps = {
  /** Ruta local al archivo de imagen (póster cacheado). Puede ser null. */
  imagePath: string | null;
  /** Ancho del póster en caracteres de terminal. Si se omite, se calcula de forma responsiva. */
  width?: number;
  /** Alto del póster en caracteres de terminal. Si se omite, se calcula de forma responsiva. */
  height?: number;
  /** Modo de renderizado: 'auto' (Sharp truecolor dentro de Ink), 'symbols' (Chafa dentro de Ink) o 'native' (Kitty overlay sobre stdout). */
  mode?: 'auto' | 'native' | 'symbols';
  /** Posición relativa sugerida para el modo nativo (overlay). Ignorado en modos dentro de Ink. */
  placement?: 'left' | 'right' | 'top' | 'bottom-right';
};

/** Dimensiones por defecto del póster cuando no se puede calcular de forma responsiva. */
const DEFAULT_WIDTH = 30;
const DEFAULT_HEIGHT = 20;

/**
 * Calcula dimensiones responsivas para el póster basándose en el tamaño de la
 * terminal, reservando espacio para la cabecera, la barra de estado y los
 * márgenes. Exportado para tests.
 */
export function computePosterSize(): { width: number; height: number } {
  const terminalRows: number = process.stdout.rows ?? 24;
  const terminalCols: number = process.stdout.columns ?? 80;

  // El póster vive en un panel lateral: ocupa como máximo un tercio del ancho.
  const availableCols = Math.max(10, Math.floor(terminalCols / 3) - 4);
  // Cabecera (1 fila) + barra de estado (1 fila) + márgenes y padding (≈2 filas).
  const availableRows = Math.max(6, terminalRows - 4);

  return {
    width: Math.min(DEFAULT_WIDTH, availableCols),
    height: Math.min(DEFAULT_HEIGHT, availableRows),
  };
}

/**
 * Calcula una posición fija y predecible en la terminal para el modo nativo
 * (overlay). Se usa solo cuando el usuario fuerza `mode='native'`, porque
 * Ink no expone las coordenadas físicas de pantalla de sus componentes.
 *
 * @param width - Ancho del póster en celdas de terminal.
 * @param height - Alto del póster en celdas de terminal.
 * @param placement - Posición relativa sugerida.
 * @returns Coordenadas (x, y) absolutas de terminal.
 */
function computeNativePosition(
  width: number,
  height: number,
  placement: 'left' | 'right' | 'top' | 'bottom-right' = 'right',
): { x: number; y: number } {
  const terminalCols: number = process.stdout.columns ?? 80;
  const terminalRows: number = process.stdout.rows ?? 24;

  let x: number;
  let y: number;

  switch (placement) {
    case 'left':
      x = Math.max(0, 1);
      y = Math.max(1, Math.floor(terminalRows / 4));
      break;
    case 'top':
      x = Math.max(0, Math.floor((terminalCols - width) / 2));
      y = Math.max(1, 1);
      break;
    case 'bottom-right':
      x = Math.max(0, terminalCols - width - 1);
      y = Math.max(1, terminalRows - height - 2);
      break;
    case 'right':
    default:
      x = Math.max(0, terminalCols - width - 1);
      y = Math.max(1, Math.floor(terminalRows / 4));
      break;
  }

  // Limitar para no salirse de la pantalla.
  x = Math.min(x, Math.max(0, terminalCols - width));
  y = Math.min(y, Math.max(1, terminalRows - height));

  return { x, y };
}

/**
 * Componente modo nativo (Kitty SGP). Overlay directo sobre stdout con
 * coordenadas aproximadas. Solo se usa cuando el usuario fuerza
 * `mode='native'`.
 */
function NativePosterImage({
  imagePath,
  width,
  height,
  placement,
}: {
  imagePath: string;
  width: number;
  height: number;
  placement: 'left' | 'right' | 'top' | 'bottom-right';
}): React.JSX.Element {
  const stdout = useStdout();
  const mountedRef = useRef(true);
  const imageIdRef = useRef<number | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const hideCursor = '\x1b[?25l';
  const showCursor = '\x1b[?25h';

  const drawPoster = useCallback(async (): Promise<void> => {
    const imageId: number = Math.floor(Math.random() * 100000);
    imageIdRef.current = imageId;

    const { x, y } = computeNativePosition(width, height, placement);
    const moveCursor = `\x1b[${y};${x}H`;

    try {
      stdout.write(hideCursor);
      stdout.write(moveCursor);
    } catch {
      // No hay nada más que hacer si no podemos escribir a stdout.
    }

    try {
      const pixels = computePixelDimensions(width, height);
      const buffer = await processImage(imagePath, pixels);

      if (!mountedRef.current) return;

      const frames = encodeKittyGraphic({
        buffer,
        columns: width,
        rows: height,
        imageId,
      });

      for (let index = 0; index < frames.length; index += 1) {
        const frame = frames[index] ?? { sequence: '', isLast: false };

        try {
          if (index === 0) {
            stdout.write(moveCursor);
          }
          stdout.write(frame.sequence);
        } catch {
          // Ignorar errores de escritura individual.
        }

        if (frame.isLast) {
          try {
            stdout.write(showCursor);
          } catch {
            // No hay nada más que hacer.
          }
        }
      }
    } catch {
      try {
        stdout.write(showCursor);
      } catch {
        // No hay nada más que hacer.
      }
    }
  }, [imagePath, width, height, placement, stdout]);

  useEffect(() => {
    mountedRef.current = true;
    void drawPoster();

    return () => {
      mountedRef.current = false;
      if (timerRef.current !== null) {
        clearTimeout(timerRef.current);
        timerRef.current = null;
      }
      // Limpieza obligatoria: borrar la imagen por su ID.
      if (imageIdRef.current !== null) {
        try {
          stdout.write(`\x1b_Ga=d,d,i,i=${imageIdRef.current}\x1b\\`);
        } catch {
          // Si el borrado falla, se ignora.
        }
      }
      try {
        stdout.write(showCursor);
      } catch {
        // No hay nada más que hacer.
      }
    };
  }, [imagePath, width, height, placement, stdout, drawPoster]);

  // Return únicamente un placeholder vacío; la imagen se dibuja sobre él vía stdout.
  return (
    <Box borderColor="gray" borderStyle="round" flexShrink={0} height={height} width={width} />
  );
}

/**
 * Componente que renderiza el póster como bloques de color ANSI truecolor
 * dentro del Box de Ink.
 *
 * Usa Sharp para procesar la imagen y generar bloques de color (█) con el
 * color de fondo truecolor de cada píxel. Como se renderiza dentro del
 * flujo de Ink, el póster queda exactamente en la posición que el layout de
 * Ink le asigne.
 */
function SharpPosterImage({
  imagePath,
  width,
  height,
}: {
  imagePath: string;
  width: number;
  height: number;
}): React.JSX.Element {
  const [lines, setLines] = useState<string[]>([]);
  const [loading, setLoading] = useState<boolean>(!!imagePath);
  const mountedRef = useRef(true);
  const abortRef = useRef<AbortController | null>(null);

  const renderPoster = useCallback(async (): Promise<void> => {
    if (!mountedRef.current) return;

    abortRef.current?.abort();
    const abortController = new AbortController();
    abortRef.current = abortController;

    if (!imagePath) {
      setLines([]);
      setLoading(false);
      return;
    }

    setLoading(true);

    await new Promise((resolve) => setTimeout(resolve, 0));

    if (!mountedRef.current || abortController.signal.aborted) return;

    try {
      const result = await imageToAnsi({ imagePath, width, height });
      if (abortController.signal.aborted || !mountedRef.current) return;

      if (result.lines.length > 0) {
        setLines(result.lines);
        setLoading(false);
      } else {
        setLines([]);
        setLoading(false);
      }
    } catch {
      if (abortController.signal.aborted || !mountedRef.current) return;
      setLines([]);
      setLoading(false);
    }
  }, [imagePath, width, height]);

  useEffect(() => {
    mountedRef.current = true;
    void renderPoster();

    return () => {
      mountedRef.current = false;
      abortRef.current?.abort();
    };
  }, [renderPoster]);

  if (loading) {
    return (
      <Box
        borderStyle="round"
        borderColor={theme.border}
        width={width}
        height={height}
        flexShrink={0}
      >
        <Text>{theme.muted('  [ PÓSTER ]')}</Text>
        <Text>{theme.muted('  Cargando póster…')}</Text>
      </Box>
    );
  }

  if (lines.length > 0) {
    return (
      <Box
        borderStyle="round"
        borderColor={theme.border}
        width={width}
        height={height}
        flexShrink={0}
        flexDirection="column"
      >
        {lines.map((line: string, index: number) => (
          <Text key={index} wrap="truncate-end">
            {line}
          </Text>
        ))}
      </Box>
    );
  }

  return (
    <Box
      borderStyle="round"
      borderColor={theme.border}
      width={width}
      height={height}
      flexShrink={0}
    >
      <Text>{theme.muted('  [ PÓSTER ]')}</Text>
      <Text>{theme.muted(imagePath ? '  (póster no disponible en disco)' : '  (sin póster)')}</Text>
    </Box>
  );
}

/**
 * Componente modo fallback ANSI (chafa símbolos).
 *
 * Renderiza el arte Unicode de chafa dentro del Box. La salida se divide por
 * líneas (`.split('\n')`) y cada línea se renderiza como un `<Text>`. Esto
 * garantiza que el póster quede exactamente en la posición que Ink le asigne
 * dentro del layout.
 */
function ChafaPosterImage({
  imagePath,
  width,
  height,
}: {
  imagePath: string | null;
  width: number;
  height: number;
}): React.JSX.Element {
  const [poster, setPoster] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const mountedRef = useRef(true);
  const abortRef = useRef<AbortController | null>(null);

  const renderPoster = useCallback(async (): Promise<void> => {
    if (!mountedRef.current) return;

    abortRef.current?.abort();
    const abortController = new AbortController();
    abortRef.current = abortController;

    if (!imagePath) {
      setPoster(null);
      setLoading(false);
      return;
    }

    setLoading(true);

    await new Promise((resolve) => setTimeout(resolve, 0));

    if (!mountedRef.current || abortController.signal.aborted) return;

    try {
      const result = await renderImageToAnsi(imagePath, {
        width,
        height,
        signal: abortController.signal,
      });
      if (abortController.signal.aborted || !mountedRef.current) return;

      if (result.output) {
        setPoster(result.output);
        setLoading(false);
      } else {
        setPoster(null);
        setLoading(false);
      }
    } catch {
      if (abortController.signal.aborted || !mountedRef.current) return;
      setPoster(null);
      setLoading(false);
    }
  }, [imagePath, width, height]);

  useEffect(() => {
    mountedRef.current = true;
    void renderPoster();

    return () => {
      mountedRef.current = false;
      abortRef.current?.abort();
    };
  }, [renderPoster]);

  if (!imagePath) {
    return (
      <Box
        borderStyle="round"
        borderColor={theme.border}
        width={width}
        height={height}
        flexShrink={0}
      >
        <Text>{theme.muted('  [ PÓSTER ]')}</Text>
        <Text>{theme.muted('  (sin póster)')}</Text>
      </Box>
    );
  }

  if (loading) {
    return (
      <Box
        borderStyle="round"
        borderColor={theme.border}
        width={width}
        height={height}
        flexShrink={0}
      >
        <Text>{theme.muted('  [ PÓSTER ]')}</Text>
        <Text>{theme.muted('  Cargando póster…')}</Text>
      </Box>
    );
  }

  if (poster) {
    const lines: string[] = poster.split('\n');
    return (
      <Box
        borderStyle="round"
        borderColor={theme.border}
        width={width}
        height={height}
        flexShrink={0}
        flexDirection="column"
      >
        {lines.map((line: string, index: number) => (
          <Text key={index} wrap="truncate-end">
            {line}
          </Text>
        ))}
      </Box>
    );
  }

  return (
    <Box
      borderStyle="round"
      borderColor={theme.border}
      width={width}
      height={height}
      flexShrink={0}
    >
      <Text>{theme.muted('  [ PÓSTER ]')}</Text>
      <Text>{theme.muted('  (póster no disponible en disco)')}</Text>
    </Box>
  );
}

/** Componente principal: renderiza el póster dentro del Box de Ink. */
export function PosterImage({
  imagePath,
  width,
  height,
  mode = 'auto',
  placement = 'right',
}: PosterImageProps): React.JSX.Element {
  const { width: posterWidth, height: posterHeight } = computePosterSize();
  const resolvedWidth: number = width ?? posterWidth;
  const resolvedHeight: number = height ?? posterHeight;

  if (!imagePath) {
    return <SharpPosterImage imagePath="" width={resolvedWidth} height={resolvedHeight} />;
  }

  if (mode === 'symbols') {
    return <ChafaPosterImage imagePath={imagePath} width={resolvedWidth} height={resolvedHeight} />;
  }

  if (mode === 'native') {
    return (
      <NativePosterImage
        imagePath={imagePath}
        width={resolvedWidth}
        height={resolvedHeight}
        placement={placement}
      />
    );
  }

  // Modo 'auto' por defecto: Sharp truecolor dentro de Ink.
  return <SharpPosterImage imagePath={imagePath} width={resolvedWidth} height={resolvedHeight} />;
}
