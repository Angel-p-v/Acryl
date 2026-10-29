import { describe, expect, it } from 'vitest';

import { defaultConfig } from '../../../src/core/config/defaults.js';
import {
  configSchema,
  mediaTypeSchema,
  playerDefinitionSchema,
} from '../../../src/core/config/schema.js';

describe('configSchema', () => {
  it('acepta una configuración mínima válida', () => {
    const result = configSchema.safeParse({ player: { default: 'mpv' } });

    expect(result.success).toBe(true);
  });

  it('rellena los valores por defecto', () => {
    const result = configSchema.safeParse({ player: { default: 'mpv' } });

    expect(result.success).toBe(true);
    if (!result.success) return;

    expect(result.data.version).toBe(1);
    expect(result.data.library.roots).toEqual([]);
    expect(result.data.library.scan.maxDepth).toBe(4);
    expect(result.data.ui.layout).toBe('split');
    expect(result.data.player.onLaunch).toBe('detach');
    expect(result.data.logging).toEqual({ level: 'warn', format: 'text' });
  });

  it('rechaza un nivel de logging desconocido', () => {
    const result = configSchema.safeParse({ logging: { level: 'trace' } });

    expect(result.success).toBe(false);
  });

  it('rechaza un tipo de medio desconocido', () => {
    const result = configSchema.safeParse({
      player: { default: 'mpv' },
      library: { roots: [{ path: '/tmp', type: 'documental' }] },
    });

    expect(result.success).toBe(false);
  });

  it('rechaza una versión de esquema no soportada', () => {
    const result = configSchema.safeParse({ version: 99, player: { default: 'mpv' } });

    expect(result.success).toBe(false);
  });

  it('exige un reproductor por defecto válido', () => {
    expect(configSchema.safeParse({ player: { default: '' } }).success).toBe(false);
  });

  it('ofrece mpv y vlc como reproductores de fábrica', () => {
    const result = configSchema.safeParse({});

    expect(result.success).toBe(true);
    if (!result.success) return;

    expect(result.data.player.default).toBe('mpv');
    expect(Object.keys(result.data.player.players)).toContain('mpv');
    expect(Object.keys(result.data.player.players)).toContain('vlc');
  });

  it('incluye el atajo goToDetails con la tecla m por defecto', () => {
    expect(defaultConfig.keybindings.goToDetails).toBe('m');
  });

  it('informa del campo exacto que falla', () => {
    const result = configSchema.safeParse({
      player: { default: 'mpv' },
      library: { roots: [{ path: '', type: 'anime' }] },
    });

    expect(result.success).toBe(false);
    if (result.success) return;

    expect(result.error.issues[0]?.path).toEqual(['library', 'roots', 0, 'path']);
  });
});

describe('mediaTypeSchema', () => {
  it.each(['series', 'anime', 'movie'] as const)('acepta el tipo %s', (type) => {
    expect(mediaTypeSchema.safeParse(type).success).toBe(true);
  });

  it('rechaza otros valores', () => {
    expect(mediaTypeSchema.safeParse('musical').success).toBe(false);
  });
});

describe('playerDefinitionSchema', () => {
  it('exige un comando', () => {
    expect(playerDefinitionSchema.safeParse({ args: [] }).success).toBe(false);
  });

  it('aplica una lista de argumentos vacía por defecto', () => {
    const result = playerDefinitionSchema.safeParse({ command: 'mpv' });

    expect(result.success).toBe(true);
    if (!result.success) return;
    expect(result.data.args).toEqual([]);
  });
});
