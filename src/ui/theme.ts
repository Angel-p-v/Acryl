import pc from 'picocolors';

import type { ThemeColors } from '../core/config/schema.js';

/** Una paleta de colores para un tema de la interfaz. */
export type Theme = {
  primary: (text: string) => string;
  accent: (text: string) => string;
  success: (text: string) => string;
  warning: (text: string) => string;
  error: (text: string) => string;
  muted: (text: string) => string;
  border: string;
  title: (text: string) => string;
  highlight: (text: string) => string;
  selected: (text: string) => string;
  label: (text: string) => string;
  value: (text: string) => string;
  type: (type: string) => string;
};

/** Mapea nombres de color a funciones de picocolors. */
const colorMap: Record<string, (text: string) => string> = {
  red: (text) => pc.red(text),
  green: (text) => pc.green(text),
  yellow: (text) => pc.yellow(text),
  blue: (text) => pc.blue(text),
  magenta: (text) => pc.magenta(text),
  cyan: (text) => pc.cyan(text),
  white: (text) => pc.white(text),
  black: (text) => pc.black(text),
  gray: (text) => pc.gray(text),
  redBright: (text) => pc.redBright(text),
  greenBright: (text) => pc.greenBright(text),
  yellowBright: (text) => pc.yellowBright(text),
  blueBright: (text) => pc.blueBright(text),
  magentaBright: (text) => pc.magentaBright(text),
  cyanBright: (text) => pc.cyanBright(text),
};

/** Resuelve una función de color a partir de un nombre; fallback a blanco. */
function resolveColor(name: string): (text: string) => string {
  return colorMap[name.toLowerCase()] ?? ((text) => pc.white(text));
}

/** Crea un Theme a partir de colores definidos por el usuario en el YAML. */
export function createTheme(colors: ThemeColors): Theme {
  const primary = resolveColor(colors.primary);
  const accent = resolveColor(colors.accent);
  const success = resolveColor(colors.success);
  const warning = resolveColor(colors.warning);
  const error = resolveColor(colors.error);
  const muted = resolveColor(colors.muted);

  return {
    primary,
    accent,
    success,
    warning,
    error,
    muted,
    border: colors.border,
    title: (text) => pc.bold(primary(text)),
    highlight: (text) => pc.bold(primary(text)),
    selected: (text) => pc.bold(primary(text)),
    label: (text) => pc.bold(text),
    value: (text) => pc.white(text),
    type: (type) => {
      if (type === 'series') return pc.blue('serie');
      if (type === 'anime') return pc.magenta('anime');
      return pc.yellow('película');
    },
  };
}

/** Paleta por defecto (colores brillantes sobre fondo oscuro). */
export const defaultTheme: Theme = {
  primary: pc.cyan,
  accent: pc.magenta,
  success: pc.green,
  warning: pc.yellow,
  error: pc.red,
  muted: pc.dim,
  border: 'gray',
  title: (text) => pc.bold(pc.cyan(text)),
  highlight: (text) => pc.bold(pc.cyan(text)),
  selected: (text) => pc.bold(pc.cyan(text)),
  label: (text) => pc.bold(text),
  value: (text) => pc.white(text),
  type: (type) => {
    if (type === 'series') return pc.blue('serie');
    if (type === 'anime') return pc.magenta('anime');
    return pc.yellow('película');
  },
};

/** Paleta oscura (colores suaves, menos contraste). */
export const darkTheme: Theme = {
  primary: pc.cyan,
  accent: pc.magenta,
  success: pc.green,
  warning: pc.yellow,
  error: pc.red,
  muted: pc.dim,
  border: 'gray',
  title: (text) => pc.bold(pc.cyan(text)),
  highlight: (text) => pc.bold(pc.cyan(text)),
  selected: (text) => pc.bold(pc.cyan(text)),
  label: (text) => pc.bold(text),
  value: (text) => pc.white(text),
  type: (type) => {
    if (type === 'series') return pc.blue('serie');
    if (type === 'anime') return pc.magenta('anime');
    return pc.yellow('película');
  },
};

/** Paleta clara (colores adaptados para fondos claros). */
export const lightTheme: Theme = {
  primary: pc.blue,
  accent: pc.magenta,
  success: pc.green,
  warning: pc.yellow,
  error: pc.red,
  muted: pc.dim,
  border: 'gray',
  title: (text) => pc.bold(pc.blue(text)),
  highlight: (text) => pc.bold(pc.blue(text)),
  selected: (text) => pc.bold(pc.blue(text)),
  label: (text) => pc.bold(text),
  value: (text) => pc.black(text),
  type: (type) => {
    if (type === 'series') return pc.blue('serie');
    if (type === 'anime') return pc.magenta('anime');
    return pc.yellow('película');
  },
};

/** Temas disponibles por nombre. */
export const themes: Record<string, Theme> = {
  default: defaultTheme,
  dark: darkTheme,
  light: lightTheme,
};

/** Resuelve el tema por nombre o por colores personalizados. */
export function resolveTheme(nameOrColors: string | ThemeColors): Theme {
  if (typeof nameOrColors === 'string') {
    return themes[nameOrColors] ?? defaultTheme;
  }
  return createTheme(nameOrColors);
}

/** Tema activo: se resuelve desde la configuración al arrancar la TUI. */
export let theme = defaultTheme;

/**
 * Establece el tema activo. Acepta un nombre de tema predeterminado
 * ('default', 'dark', 'light') o un objeto con colores personalizados.
 * Los componentes que importan `theme` lo ven reflejado.
 */
export function setTheme(nameOrColors: string | ThemeColors): void {
  theme = resolveTheme(nameOrColors);
}

/** Formatea una ruta acortándola para que quepa en la terminal. */
export function shortenPath(target: string, maxLength = 48): string {
  if (target.length <= maxLength) return target;
  return `…${target.slice(target.length - maxLength + 1)}`;
}
