import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

import {
  detectGraphicsCapability,
  supportsKittyGraphics,
} from '../../../src/shared/terminalCapabilities.js';

beforeEach(() => {
  // Limpiar variables de entorno relacionadas con gráficos.
  delete process.env.MEDIA_MANAGER_GRAPHICS;
  delete process.env.GHOSTTY_RESOURCES_DIR;
  delete process.env.KITTY_WINDOW_ID;
  delete process.env.TERM_PROGRAM;
  delete process.env.TERM;
});

afterEach(() => {
  vi.unstubAllGlobals();
  delete process.env.MEDIA_MANAGER_GRAPHICS;
  delete process.env.GHOSTTY_RESOURCES_DIR;
  delete process.env.KITTY_WINDOW_ID;
  delete process.env.TERM_PROGRAM;
  delete process.env.TERM;
});

describe('supportsKittyGraphics', () => {
  it('devuelve true con MEDIA_MANAGER_GRAPHICS=native', () => {
    process.env.MEDIA_MANAGER_GRAPHICS = 'native';
    expect(supportsKittyGraphics()).toBe(true);
  });

  it('devuelve true con MEDIA_MANAGER_GRAPHICS=kitty', () => {
    process.env.MEDIA_MANAGER_GRAPHICS = 'kitty';
    expect(supportsKittyGraphics()).toBe(true);
  });

  it('devuelve false con MEDIA_MANAGER_GRAPHICS=symbols', () => {
    process.env.MEDIA_MANAGER_GRAPHICS = 'symbols';
    expect(supportsKittyGraphics()).toBe(false);
  });

  it('devuelve false con MEDIA_MANAGER_GRAPHICS=none', () => {
    process.env.MEDIA_MANAGER_GRAPHICS = 'none';
    expect(supportsKittyGraphics()).toBe(false);
  });

  it('devuelve true con GHOSTTY_RESOURCES_DIR', () => {
    process.env.GHOSTTY_RESOURCES_DIR = '/usr/share/ghostty';
    expect(supportsKittyGraphics()).toBe(true);
  });

  it('devuelve true con TERM_PROGRAM=kitty', () => {
    process.env.TERM_PROGRAM = 'kitty';
    expect(supportsKittyGraphics()).toBe(true);
  });

  it('devuelve true con TERM_PROGRAM=WezTerm', () => {
    process.env.TERM_PROGRAM = 'WezTerm';
    expect(supportsKittyGraphics()).toBe(true);
  });

  it('devuelve true con TERM_PROGRAM=Ghostty', () => {
    process.env.TERM_PROGRAM = 'Ghostty';
    expect(supportsKittyGraphics()).toBe(true);
  });

  it('devuelve true con KITTY_WINDOW_ID', () => {
    process.env.KITTY_WINDOW_ID = '123';
    expect(supportsKittyGraphics()).toBe(true);
  });

  it('devuelve false sin variables de terminal nativo', () => {
    expect(supportsKittyGraphics()).toBe(false);
  });
});

describe('detectGraphicsCapability', () => {
  it('devuelve "native" cuando soporta Kitty', () => {
    process.env.MEDIA_MANAGER_GRAPHICS = 'native';
    expect(detectGraphicsCapability()).toBe('native');
  });

  it('devuelve "symbols" cuando no soporta Kitty', () => {
    expect(detectGraphicsCapability()).toBe('symbols');
  });

  it('devuelve "symbols" con MEDIA_MANAGER_GRAPHICS=symbols', () => {
    process.env.MEDIA_MANAGER_GRAPHICS = 'symbols';
    expect(detectGraphicsCapability()).toBe('symbols');
  });
});
