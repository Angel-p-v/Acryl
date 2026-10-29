import { describe, it, expect } from 'vitest';

import {
  computeDisplayDimensions,
  computePixelDimensions,
  CELL_PIXELS,
} from '../../../src/shared/imageProcessor.js';

describe('CELL_PIXELS', () => {
  it('tiene ancho 16 y alto 32', () => {
    expect(CELL_PIXELS.width).toBe(16);
    expect(CELL_PIXELS.height).toBe(32);
  });
});

describe('computePixelDimensions', () => {
  it('convierte celdas a píxeles (16x32 por celda)', () => {
    expect(computePixelDimensions(1, 1)).toEqual({ widthPx: 16, heightPx: 32 });
    expect(computePixelDimensions(45, 32)).toEqual({ widthPx: 720, heightPx: 1024 });
  });

  it('nunca devuelve cero ni negativos', () => {
    expect(computePixelDimensions(0, 0)).toEqual({ widthPx: 16, heightPx: 32 });
    expect(computePixelDimensions(-3, 5)).toEqual({ widthPx: 16, heightPx: 160 });
  });
});

describe('computeDisplayDimensions', () => {
  it('preserva aspect ratio cuando la imagen es más ancha que alta', () => {
    // Imagen 800x400 (aspect 2:1), terminal 40x20.
    // Ajustar al ancho: cols=40, rows=20. Cabe en 20 filas.
    const result = computeDisplayDimensions(800, 400, 40, 20);
    expect(result.cols).toBe(40);
    expect(result.rows).toBe(20);
  });

  it('ajusta al alto disponible cuando la imagen es muy alta', () => {
    // Imagen 400x800 (aspect 0.5:1), terminal 40x20.
    // Ajustar al ancho: cols=40, rows=80. No cabe en 20 filas.
    // Ajustar al alto: rows=20, cols=10.
    const result = computeDisplayDimensions(400, 800, 40, 20);
    expect(result.cols).toBe(10);
    expect(result.rows).toBe(20);
  });

  it('nunca devuelve cero ni negativos', () => {
    const result = computeDisplayDimensions(100, 100, 0, 0);
    expect(result.cols).toBeGreaterThanOrEqual(1);
    expect(result.rows).toBeGreaterThanOrEqual(1);
  });

  it('mantiene proporción 16:9 en terminal estándar', () => {
    // Imagen 1920x1080 (16:9), terminal 80x24.
    // Ajustar al ancho: cols=80, rows=45. No cabe en 24 filas.
    // Ajustar al alto: rows=24, cols=42 (24 * 16/9 = 42.67 -> 42).
    const result = computeDisplayDimensions(1920, 1080, 80, 24);
    expect(result.rows).toBe(24);
    expect(result.cols).toBe(42);
  });
});
