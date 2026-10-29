import path from 'node:path';

import { describe, expect, it } from 'vitest';

import { expandHome, resolvePath } from '../../../src/shared/paths.js';
import { getVersion } from '../../../src/shared/version.js';

describe('getVersion', () => {
  it('devuelve la versión declarada en package.json', () => {
    expect(getVersion()).toMatch(/^\d+\.\d+\.\d+/);
  });
});

describe('expandHome', () => {
  it('expande ~ al directorio personal', () => {
    expect(expandHome('~/videos', '/home/usuario')).toBe('/home/usuario/videos');
  });

  it('expande ~ suelto', () => {
    expect(expandHome('~', '/home/usuario')).toBe('/home/usuario');
  });

  it('deja intactas las rutas absolutas', () => {
    expect(expandHome('/media/videos', '/home/usuario')).toBe('/media/videos');
  });

  it('deja intactas las rutas relativas', () => {
    expect(expandHome('videos', '/home/usuario')).toBe('videos');
  });
});

describe('resolvePath', () => {
  it('resuelve las rutas relativas contra el directorio base', () => {
    expect(resolvePath('./videos', '/base')).toBe('/base/videos');
  });

  it('normaliza las rutas absolutas', () => {
    expect(resolvePath('/media/../media/videos', '/base')).toBe('/media/videos');
  });

  it('expande ~ antes de resolver', () => {
    expect(resolvePath('~/videos', '/base')).toBe(path.join(process.env.HOME ?? '/root', 'videos'));
  });
});
