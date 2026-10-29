import { describe, expect, it } from 'vitest';

import { matchesKey } from '../../../src/ui/hooks/useKeymap.js';

import type { Key } from 'ink';

function key(overrides: Partial<Key> = {}): Key {
  return {
    upArrow: false,
    downArrow: false,
    leftArrow: false,
    rightArrow: false,
    pageDown: false,
    pageUp: false,
    home: false,
    end: false,
    return: false,
    escape: false,
    ctrl: false,
    shift: false,
    tab: false,
    backspace: false,
    delete: false,
    meta: false,
    super: false,
    hyper: false,
    capsLock: false,
    numLock: false,
    ...overrides,
  };
}

describe('matchesKey', () => {
  it('reconoce caracteres sueltos', () => {
    expect(matchesKey('q', 'q', key())).toBe(true);
    expect(matchesKey('q', 'x', key())).toBe(false);
  });

  it('reconoce las flechas y teclas especiales', () => {
    expect(matchesKey('up', '', key({ upArrow: true }))).toBe(true);
    expect(matchesKey('down', '', key({ downArrow: true }))).toBe(true);
    expect(matchesKey('left', '', key({ leftArrow: true }))).toBe(true);
    expect(matchesKey('right', '', key({ rightArrow: true }))).toBe(true);
    expect(matchesKey('tab', '', key({ tab: true }))).toBe(true);
    expect(matchesKey('return', '', key({ return: true }))).toBe(true);
    expect(matchesKey('enter', '', key({ return: true }))).toBe(true);
    expect(matchesKey('escape', '', key({ escape: true }))).toBe(true);
    expect(matchesKey('esc', '', key({ escape: true }))).toBe(true);
    expect(matchesKey('backspace', '', key({ backspace: true }))).toBe(true);
    expect(matchesKey('delete', '', key({ delete: true }))).toBe(true);
    expect(matchesKey('space', ' ', key())).toBe(true);
  });

  it('no confunde una tecla especial con otra', () => {
    expect(matchesKey('up', '', key({ downArrow: true }))).toBe(false);
    expect(matchesKey('tab', '\t', key())).toBe(false);
  });

  it('ignora atajos de más de un carácter que no sean especiales', () => {
    expect(matchesKey('ctrl+k', 'k', key({ ctrl: true }))).toBe(false);
  });
});
