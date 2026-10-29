import { useMemo } from 'react';

import { defaultConfig } from '../../core/config/defaults.js';

import type { AppConfig } from '../../core/config/schema.js';

/** Atajos efectivos: los valores por defecto sobrescritos por el YAML. */
export type Keybindings = Record<string, string>;

export type UseConfigResult = {
  config: AppConfig;
  keybindings: Keybindings;
};

/** Resuelve la configuración de la interfaz una sola vez por objeto de configuración. */
export function useConfig(config: AppConfig): UseConfigResult {
  return useMemo(
    () => ({
      config,
      keybindings: { ...defaultConfig.keybindings, ...config.keybindings },
    }),
    [config],
  );
}
