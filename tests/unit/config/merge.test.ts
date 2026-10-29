import { describe, expect, it } from 'vitest';

import { mergeDeep } from '../../../src/core/config/merge.js';

describe('mergeDeep', () => {
  it('fusióna objetos anidados', () => {
    const result = mergeDeep({ a: { b: 1, c: 2 } }, { a: { c: 3, d: 4 } });

    expect(result).toEqual({ a: { b: 1, c: 3, d: 4 } });
  });

  it('reemplaza los arrays en lugar de concatenarlos', () => {
    const result = mergeDeep({ list: [1, 2, 3] }, { list: [9] });

    expect(result).toEqual({ list: [9] });
  });

  it('ignora undefined y null en la capa superior', () => {
    const result = mergeDeep({ a: 1, b: 2 }, { a: undefined, b: null });

    expect(result).toEqual({ a: 1, b: 2 });
  });

  it('sustituye valores escalares', () => {
    const result = mergeDeep({ level: 'info' }, { level: 'debug' });

    expect(result).toEqual({ level: 'debug' });
  });

  it('no muta los objetos de origen', () => {
    const base = { a: { b: 1 } };
    const override = { a: { b: 2 } };

    mergeDeep(base, override);

    expect(base.a.b).toBe(1);
    expect(override.a.b).toBe(2);
  });

  it('añade claves que no existen en la base', () => {
    const result = mergeDeep({ a: 1 }, { b: 2 });

    expect(result).toEqual({ a: 1, b: 2 });
  });
});
