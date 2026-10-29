import { describe, it, expect } from 'vitest';

import { App } from '../App';

describe('App', () => {
  it('es una función componente de React', () => {
    expect(typeof App).toBe('function');
  });
});
