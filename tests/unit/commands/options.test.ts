import { InvalidArgumentError } from 'commander';
import { describe, expect, it } from 'vitest';

import {
  parseMediaType,
  parseNonNegativeInteger,
  parsePositiveInteger,
  parseSortOrder,
} from '../../../src/commands/options.js';

describe('opciones de la CLI', () => {
  describe('parseMediaType', () => {
    it.each(['series', 'anime', 'movie'] as const)('acepta %s', (value) => {
      expect(parseMediaType(value)).toBe(value);
    });

    it('rechaza un tipo desconocido', () => {
      expect(() => parseMediaType('libros')).toThrow(InvalidArgumentError);
      expect(() => parseMediaType('libros')).toThrow(/series, anime o movie/);
    });
  });

  describe('parseSortOrder', () => {
    it.each(['title', 'added', 'recent'] as const)('acepta %s', (value) => {
      expect(parseSortOrder(value)).toBe(value);
    });

    it('rechaza un orden desconocido', () => {
      expect(() => parseSortOrder('duracion')).toThrow(/title, added o recent/);
    });
  });

  describe('parsePositiveInteger', () => {
    it('acepta un entero positivo', () => {
      expect(parsePositiveInteger('25')).toBe(25);
    });

    it.each(['0', '-1', '1.5', 'abc', ''])('rechaza «%s»', (value) => {
      expect(() => parsePositiveInteger(value)).toThrow(InvalidArgumentError);
    });
  });

  describe('parseNonNegativeInteger', () => {
    it('acepta cero y enteros positivos', () => {
      expect(parseNonNegativeInteger('0')).toBe(0);
      expect(parseNonNegativeInteger('10')).toBe(10);
    });

    it.each(['-1', '2.5', 'muchos'])('rechaza «%s»', (value) => {
      expect(() => parseNonNegativeInteger(value)).toThrow(/entero no negativo/);
    });
  });
});
