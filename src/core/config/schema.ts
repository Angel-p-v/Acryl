import { z } from 'zod';

/** Tipos de contenido que la biblioteca puede contener. */
export const mediaTypeSchema = z.enum(['series', 'anime', 'movie', 'unknown']);

export type MediaType = z.infer<typeof mediaTypeSchema>;

/** Una carpeta raíz de la biblioteca, con el tipo de contenido que contiene. */
export const libraryRootSchema = z.object({
  path: z.string().min(1, 'la ruta no puede estar vacía'),
  type: mediaTypeSchema,
  label: z.string().min(1).optional(),
});

export type LibraryRoot = z.infer<typeof libraryRootSchema>;

/** Configuración de escaneo. */
export const scanOptionsSchema = z.object({
  followSymlinks: z.boolean().default(false),
  maxDepth: z.number().int().min(1).max(12).default(4),
  minSizeBytes: z.number().int().min(0).default(0),
});

export type ScanOptions = z.infer<typeof scanOptionsSchema>;

/** Definición de un reproductor externo. */
export const playerDefinitionSchema = z.object({
  command: z.string().min(1, 'el comando no puede estar vacío'),
  args: z.array(z.string()).default([]),
  env: z.record(z.string(), z.string()).optional(),
});

export type PlayerDefinition = z.infer<typeof playerDefinitionSchema>;

/** Reproductores que la aplicación conoce de fábrica. */
export const defaultPlayers: Record<string, PlayerDefinition> = {
  mpv: {
    command: 'mpv',
    args: ['--force-window=immediate', '--image-display-duration=inf', '{file}'],
  },
  vlc: {
    command: 'vlc',
    args: ['--started-from-file', '{file}'],
  },
};

/** Reproductores disponibles y política de lanzamiento. */
export const playerConfigSchema = z.object({
  default: z.string().min(1, 'debe indicarse un reproductor por defecto').default('mpv'),
  onLaunch: z.enum(['detach', 'foreground']).default('detach'),
  players: z.record(z.string(), playerDefinitionSchema).default(defaultPlayers),
});

export type PlayerConfig = z.infer<typeof playerConfigSchema>;

/** Colores de un tema personalizado definido por el usuario en el YAML. */
export const themeColorsSchema = z.object({
  primary: z.string().default('cyan'),
  accent: z.string().default('magenta'),
  success: z.string().default('green'),
  warning: z.string().default('yellow'),
  error: z.string().default('red'),
  muted: z.string().default('gray'),
  border: z.string().default('gray'),
});

export type ThemeColors = z.infer<typeof themeColorsSchema>;

/** Preferencias de la interfaz de terminal. */
export const uiConfigSchema = z.object({
  theme: z.union([z.enum(['default', 'dark', 'light']), themeColorsSchema]).default('default'),
  layout: z.enum(['split', 'list']).default('split'),
  sort: z.enum(['title', 'recent', 'added']).default('title'),
  showDetails: z.boolean().default(true),
  /** Vista compacta: más títulos por pantalla, menos detalle por fila. */
  compact: z.boolean().default(false),
});

export type UiConfig = z.infer<typeof uiConfigSchema>;

/** Niveles de logging ordenados de menor a mayor detalle. */
export const logLevelSchema = z.enum(['silent', 'error', 'warn', 'info', 'debug']);

export type LoggingLevel = z.infer<typeof logLevelSchema>;

/** Formato de los logs: legible para humanos o una línea JSON por evento. */
export const logFormatSchema = z.enum(['text', 'json']);

export type LoggingFormat = z.infer<typeof logFormatSchema>;

/** Diagnóstico de la aplicación; los logs van siempre a `stderr`. */
export const loggingConfigSchema = z.object({
  level: logLevelSchema.default('warn'),
  format: logFormatSchema.default('text'),
});

export type LoggingConfig = z.infer<typeof loggingConfigSchema>;

/** Configuración de reproducción y estado. */
export const playbackConfigSchema = z.object({
  tracking: z.enum(['auto', 'manual', 'off']).default('auto'),
  resume: z.boolean().default(true),
  resumeMinSeconds: z.number().int().min(0).default(30),
  completedThreshold: z.number().min(0).max(1).default(0.9),
  completedTailSeconds: z.number().int().min(0).default(30),
});

export type PlaybackConfig = z.infer<typeof playbackConfigSchema>;

/** Configuración de metadatos externos (proveedores públicos gratuitos). */
export const metadataConfigSchema = z.object({
  cacheTtlHours: z.number().int().min(1).default(24),
  /** Obtener metadatos automáticamente para títulos nuevos al indexar. */
  autoFetch: z.boolean().default(false),
});

export type MetadataConfig = z.infer<typeof metadataConfigSchema>;

/** Esquema completo del archivo de configuración. */
export const configSchema = z.object({
  version: z.literal(1).default(1),
  library: z
    .object({
      roots: z.array(libraryRootSchema).default([]),
      exclude: z
        .array(z.string())
        .default(['**/*.sample.*', '**/sample/**', '**/.hidden/**', '**/@eaDir/**']),
      scan: scanOptionsSchema.prefault({}),
    })
    .prefault({}),
  database: z
    .object({
      path: z.string().min(1).default('~/.local/share/media-manager/library.db'),
    })
    .prefault({}),
  player: playerConfigSchema.prefault({}),
  ui: uiConfigSchema.prefault({}),
  logging: loggingConfigSchema.prefault({}),
  playback: playbackConfigSchema.prefault({}),
  metadata: metadataConfigSchema.prefault({}),
  keybindings: z.record(z.string(), z.string()).default({}),
});

export type AppConfig = z.infer<typeof configSchema>;

/** Extrae el esquema en JSON Schema para documentarlo o generarlo desde el código. */
export function toJsonSchema(): Record<string, unknown> {
  return z.toJSONSchema(configSchema, { io: 'input' });
}
