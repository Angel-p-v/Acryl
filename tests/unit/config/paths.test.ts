import { describe, expect, it } from 'vitest';

import {
  cacheHome,
  configHome,
  dataHome,
  defaultConfigPath,
  defaultDatabasePath,
  stateHome,
} from '../../../src/core/config/paths.js';

describe('rutas XDG', () => {
  it('usa XDG_CONFIG_HOME si es una ruta absoluta', () => {
    expect(configHome({ XDG_CONFIG_HOME: '/tmp/config' })).toBe('/tmp/config');
  });

  it('ignora XDG_CONFIG_HOME si no es absoluta', () => {
    const result = configHome({ XDG_CONFIG_HOME: 'relativo' });

    expect(result.startsWith('/')).toBe(true);
    expect(result).toContain('.config');
  });

  it('resuelve data, state y cache', () => {
    expect(dataHome({ XDG_DATA_HOME: '/tmp/data' })).toBe('/tmp/data');
    expect(stateHome({ XDG_STATE_HOME: '/tmp/state' })).toBe('/tmp/state');
    expect(cacheHome({ XDG_CACHE_HOME: '/tmp/cache' })).toBe('/tmp/cache');
  });

  it('sitúa el archivo de configuración en la carpeta de la aplicación', () => {
    expect(defaultConfigPath({ XDG_CONFIG_HOME: '/tmp/config' })).toBe(
      '/tmp/config/media-manager/config.yaml',
    );
  });

  it('sitúa la base de datos en la carpeta de datos de la aplicación', () => {
    expect(defaultDatabasePath({ XDG_DATA_HOME: '/tmp/data' })).toBe(
      '/tmp/data/media-manager/library.db',
    );
  });
});
