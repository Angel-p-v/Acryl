import { mkdtemp, writeFile } from 'node:fs/promises';
import { homedir, tmpdir } from 'node:os';
import path from 'node:path';

import { describe, expect, it } from 'vitest';

import { defaultConfig } from '../../../src/core/config/defaults.js';
import {
  applyEnvOverrides,
  loadConfig,
  readConfigFile,
  resolveConfigPath,
  saveConfig,
  validateConfig,
  watchConfigFile,
} from '../../../src/core/config/loader.js';
import { ConfigError } from '../../../src/shared/errors.js';

import type { AppConfig } from '../../../src/core/config/schema.js';

async function tempConfig(content: string): Promise<string> {
  const dir = await mkdtemp(path.join(tmpdir(), 'mm-config-'));
  const file = path.join(dir, 'config.yaml');
  await writeFile(file, content, 'utf8');
  return file;
}

describe('resolveConfigPath', () => {
  it('prioriza la ruta explícita', () => {
    const result = resolveConfigPath({
      configPath: '/tmp/explicita.yaml',
      env: { MM_CONFIG: '/tmp/env.yaml' },
    });

    expect(result).toBe('/tmp/explicita.yaml');
  });

  it('usa MM_CONFIG cuando no hay ruta explícita', () => {
    const result = resolveConfigPath({ env: { MM_CONFIG: '/tmp/env.yaml' } });

    expect(result).toBe('/tmp/env.yaml');
  });

  it('cae en la ruta por defecto', () => {
    const result = resolveConfigPath({ env: {} });

    expect(result).toContain('media-manager');
    expect(result.endsWith('config.yaml')).toBe(true);
  });
});

describe('readConfigFile', () => {
  it('devuelve un objeto vacío si el archivo no existe', async () => {
    const result = await readConfigFile('/tmp/no-existe-esta-cosa.yaml');

    expect(result).toEqual({});
  });

  it('lanza ConfigError si el YAML es inválido', async () => {
    const file = await tempConfig('library:\n  roots:\n   - path: "sin cerrar\n');

    await expect(readConfigFile(file)).rejects.toBeInstanceOf(ConfigError);
  });

  it('lanza ConfigError si el YAML no es un objeto', async () => {
    const file = await tempConfig('- esto\n- es\n- una\n- lista\n');

    await expect(readConfigFile(file)).rejects.toBeInstanceOf(ConfigError);
  });
});

describe('validateConfig', () => {
  it('devuelve la configuración validada', () => {
    const result = validateConfig({ player: { default: 'mpv' } }, '/tmp/config.yaml');

    expect(result.player.default).toBe('mpv');
  });

  it('indica el campo que incumple el esquema', () => {
    expect(() =>
      validateConfig(
        { player: { default: 'mpv' }, library: { scan: { maxDepth: 99 } } },
        '/tmp/c.yaml',
      ),
    ).toThrow(/library\.scan\.maxDepth/);
  });
});

describe('applyEnvOverrides', () => {
  it('sobrescribe la ruta de la base de datos', () => {
    const result = applyEnvOverrides(defaultConfig, { MM_DATABASE_PATH: '/tmp/otra.db' });

    expect(result.database.path).toBe('/tmp/otra.db');
  });

  it('sobrescribe el reproductor por defecto', () => {
    const result = applyEnvOverrides(defaultConfig, { MM_PLAYER: 'vlc' });

    expect(result.player.default).toBe('vlc');
  });

  it('sobrescribe el nivel y el formato de logging', () => {
    const result = applyEnvOverrides(defaultConfig, {
      MM_LOG_LEVEL: 'debug',
      MM_LOG_FORMAT: 'json',
    });

    expect(result.logging).toEqual({ level: 'debug', format: 'json' });
  });

  it('rechaza un nivel o formato de logging inválido', () => {
    expect(() => applyEnvOverrides(defaultConfig, { MM_LOG_LEVEL: 'trace' })).toThrow(
      'MM_LOG_LEVEL inválido',
    );
    expect(() => applyEnvOverrides(defaultConfig, { MM_LOG_FORMAT: 'yaml' })).toThrow(
      'MM_LOG_FORMAT inválido',
    );
  });

  it('no toca la configuración si no hay variables', () => {
    const result = applyEnvOverrides(defaultConfig, {});

    expect(result.database.path).toBe(defaultConfig.database.path);
  });
});

describe('loadConfig', () => {
  it('fusiona el archivo con los valores por defecto', async () => {
    const file = await tempConfig('player:\n  default: vlc\n');

    const config = await loadConfig({ configPath: file, env: {} });

    expect(config.player.default).toBe('vlc');
    expect(config.library.scan.maxDepth).toBe(4);
    expect(config.ui.layout).toBe('split');
  });

  it('devuelve los valores por defecto si el archivo no existe', async () => {
    const config = await loadConfig({ configPath: '/tmp/no-existe-esta-cosa.yaml', env: {} });

    expect(config.player.default).toBe(defaultConfig.player.default);
  });

  it('reemplaza los arrays en lugar de concatenarlos', async () => {
    const file = await tempConfig("library:\n  exclude:\n    - '**/prueba/**'\n");

    const config = await loadConfig({ configPath: file, env: {} });

    expect(config.library.exclude).toEqual(['**/prueba/**']);
  });

  it('convierte las rutas relativas en absolutas respecto al archivo', async () => {
    const file = await tempConfig('library:\n  roots:\n    - path: ./videos\n      type: anime\n');

    const config = await loadConfig({ configPath: file, env: {} });

    expect(config.library.roots[0]?.path).toBe(path.join(path.dirname(file), 'videos'));
  });

  it('lanza ConfigError cuando el archivo no cumple el esquema', async () => {
    const file = await tempConfig(
      'player:\n  default: mpv\nlibrary:\n  roots:\n    - path: /tmp\n      type: documental\n',
    );

    await expect(loadConfig({ configPath: file, env: {} })).rejects.toBeInstanceOf(ConfigError);
  });

  it('aplica las variables de entorno por encima del archivo', async () => {
    const file = await tempConfig('player:\n  default: mpv\n');

    const config = await loadConfig({
      configPath: file,
      env: { MM_PLAYER: 'vlc', MM_DATABASE_PATH: '/tmp/env.db' },
    });

    expect(config.player.default).toBe('vlc');
    expect(config.database.path).toBe('/tmp/env.db');
  });
});

describe('watchConfigFile', () => {
  it('llama al callback con la config recargada cuando el archivo cambia', async () => {
    const file = await tempConfig('library:\n  exclude:\n    - "**/prueba/**"\n');

    const calls: string[][] = [];
    const { close } = watchConfigFile(file, (config) => {
      calls.push([...config.library.exclude]);
    });

    // Esperar a que el watcher se active y cargue el valor inicial
    await new Promise((resolve) => setTimeout(resolve, 150));

    // Modificar el archivo
    await writeFile(file, 'library:\n  exclude:\n    - "**/nuevo/**"\n', 'utf8');

    // Esperar a que se procese el cambio (debounce de 200ms)
    await new Promise((resolve) => setTimeout(resolve, 400));

    close();

    // Primera llamada: valor inicial, segunda: valor tras el cambio
    expect(calls).toHaveLength(2);
    expect(calls[0]).toEqual(['**/prueba/**']);
    expect(calls[1]).toEqual(['**/nuevo/**']);
  });

  it('no falla si el archivo se vuelve inválido durante la recarga', async () => {
    const file = await tempConfig('library:\n  exclude:\n    - "**/valido/**"\n');

    const calls: string[][] = [];
    const { close } = watchConfigFile(file, (config) => {
      calls.push([...config.library.exclude]);
    });

    await new Promise((resolve) => setTimeout(resolve, 150));

    // Escribir YAML inválido (sintaxis rota)
    await writeFile(file, 'library:\n  exclude: [no cerrado\n', 'utf8');

    await new Promise((resolve) => setTimeout(resolve, 400));

    close();

    // Debe haber llamado al menos una vez con el valor válido inicial
    expect(calls.length).toBeGreaterThanOrEqual(1);
    expect(calls[0]).toEqual(['**/valido/**']);
  });

  it('no crashea si el archivo de configuración no existe', () => {
    // Primera ejecución sin config previa (p. ej. contenedor sin MM_CONFIG):
    // no debe lanzar ENOENT al intentar vigilar el archivo.
    const { close } = watchConfigFile('/tmp/no-existe-esta-cosa.yaml', () => undefined);
    expect(typeof close).toBe('function');
    close();
  });
});

describe('saveConfig', () => {
  it('escribe el YAML y lo puede recargar con loadConfig', async () => {
    const dir = await mkdtemp(path.join(tmpdir(), 'mm-save-'));
    const file = path.join(dir, 'config.yaml');

    await saveConfig(defaultConfig, file);

    const reloaded = await loadConfig({ configPath: file, env: {} });
    expect(reloaded.player.default).toBe(defaultConfig.player.default);
    expect(reloaded.database.path).toBe(
      path.resolve(homedir(), '.local/share/media-manager/library.db'),
    );
  });

  it('rechaza una configuración inválida antes de escribir', async () => {
    const dir = await mkdtemp(path.join(tmpdir(), 'mm-save-'));
    const file = path.join(dir, 'config.yaml');

    const invalidConfig: AppConfig = {
      ...defaultConfig,
      library: { ...defaultConfig.library, scan: { ...defaultConfig.library.scan, maxDepth: 99 } },
    };

    await expect(saveConfig(invalidConfig, file)).rejects.toBeInstanceOf(ConfigError);

    // El archivo no debe haberse creado
    expect(await readConfigFile(file)).toEqual({});
  });

  it('crea el directorio padre y el archivo si no existen (primera ejecución)', async () => {
    const dir = await mkdtemp(path.join(tmpdir(), 'mm-save-'));
    // La subcarpeta no existe: saveConfig debe crearla antes de escribir.
    const file = path.join(dir, 'aun-no-existe', 'config.yaml');

    await saveConfig(defaultConfig, file);

    const reloaded = await loadConfig({ configPath: file, env: {} });
    expect(reloaded.player.default).toBe(defaultConfig.player.default);
    expect(reloaded.database.path).toBe(
      path.resolve(homedir(), '.local/share/media-manager/library.db'),
    );
  });
});
