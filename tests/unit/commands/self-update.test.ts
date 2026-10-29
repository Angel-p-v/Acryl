import { afterEach, describe, expect, it } from 'vitest';

import { isVersionNewer, getPlatformIdentifier } from '../../../src/commands/self-update.js';

describe('isVersionNewer', () => {
  it('devuelve true si a es mayor que b', () => {
    expect(isVersionNewer('2.1.0', '2.0.0')).toBe(true);
    expect(isVersionNewer('2.0.1', '2.0.0')).toBe(true);
    expect(isVersionNewer('3.0.0', '2.9.9')).toBe(true);
  });

  it('devuelve false si a es igual a b', () => {
    expect(isVersionNewer('2.0.0', '2.0.0')).toBe(false);
    expect(isVersionNewer('1.2.3', '1.2.3')).toBe(false);
  });

  it('devuelve false si a es menor que b', () => {
    expect(isVersionNewer('2.0.0', '2.1.0')).toBe(false);
    expect(isVersionNewer('1.9.9', '2.0.0')).toBe(false);
    expect(isVersionNewer('2.0.0', '2.0.1')).toBe(false);
  });

  it('ignora el prefijo v', () => {
    expect(isVersionNewer('v2.1.0', '2.0.0')).toBe(true);
    expect(isVersionNewer('2.1.0', 'v2.0.0')).toBe(true);
  });

  it('maneja versiones con un solo componente', () => {
    expect(isVersionNewer('2', '1')).toBe(true);
    expect(isVersionNewer('1', '2')).toBe(false);
  });

  it('maneja versiones con componentes faltantes como cero', () => {
    expect(isVersionNewer('2.1', '2.1.0')).toBe(false);
    expect(isVersionNewer('2.1.1', '2.1')).toBe(true);
  });
});

describe('getPlatformIdentifier', () => {
  const originalPlatform = process.platform;

  afterEach(() => {
    Object.defineProperty(process, 'platform', {
      value: originalPlatform,
      configurable: true,
    });
  });

  it('devuelve linux para plataforma linux', () => {
    Object.defineProperty(process, 'platform', { value: 'linux', configurable: true });
    expect(getPlatformIdentifier()).toBe('linux');
  });

  it('devuelve windows para plataforma win32', () => {
    Object.defineProperty(process, 'platform', { value: 'win32', configurable: true });
    expect(getPlatformIdentifier()).toBe('windows');
  });

  it('devuelve darwin para plataforma darwin', () => {
    Object.defineProperty(process, 'platform', { value: 'darwin', configurable: true });
    expect(getPlatformIdentifier()).toBe('darwin');
  });

  it('devuelve linux como fallback para plataformas desconocidas', () => {
    Object.defineProperty(process, 'platform', { value: 'freebsd', configurable: true });
    expect(getPlatformIdentifier()).toBe('linux');
  });
});
