import { appendFileSync, existsSync, statSync, renameSync } from 'node:fs';

import pc from 'picocolors';

export type LogLevel = 'silent' | 'error' | 'warn' | 'info' | 'debug';
export type LogFormat = 'text' | 'json';

/** Campos estructurados que acompañan a un evento de log. */
export type LogFields = Record<string, unknown>;

export type Logger = {
  readonly level: LogLevel;
  readonly format: LogFormat;
  error(event: string, fields?: LogFields): void;
  warn(event: string, fields?: LogFields): void;
  info(event: string, fields?: LogFields): void;
  debug(event: string, fields?: LogFields): void;
  /** Devuelve un logger que añade campos fijos a todos sus eventos. */
  child(bindings: LogFields): Logger;
};

export type LoggerOptions = {
  level?: LogLevel;
  format?: LogFormat;
  stream?: NodeJS.WritableStream;
  bindings?: LogFields;
  /** Inyectable para tests deterministas. */
  now?: () => Date;
  /** Ruta de archivo para escribir los logs (además de stderr). */
  logFile?: string;
  /** Tamaño máximo del archivo de log antes de rotar (bytes). Por defecto 10 MB. */
  maxLogSizeBytes?: number;
};

const LEVEL_WEIGHT: Record<LogLevel, number> = {
  silent: 0,
  error: 1,
  warn: 2,
  info: 3,
  debug: 4,
};

type ActiveLevel = Exclude<LogLevel, 'silent'>;

const STYLES: Record<ActiveLevel, { icon: string; color: (text: string) => string }> = {
  error: { icon: '✖', color: pc.red },
  warn: { icon: '▲', color: pc.yellow },
  info: { icon: '•', color: pc.cyan },
  debug: { icon: '·', color: pc.dim },
};

/**
 * Crea un logger estructurado.
 *
 * En formato `text` cada evento es `icono evento clave=valor`; en formato `json`
 * se escribe una línea con `time`, `level`, `event` y los campos. Los detalles
 * nunca se intercalan con `stdout`, de modo que `--json` sigue siendo parseable.
 */
/**
 * Rota el fichero de log si supera el tamaño máximo.
 *
 * Renombra `<logFile>` a `<logFile>.<timestamp>.old` para preservar el historial
 * y permitir que se cree uno nuevo vacío. Si la rotación falla, se ignora
 * deliberadamente (los logs seguirán escribiéndose en el fichero actual).
 */
function rotateLogFile(logFile: string, maxSizeBytes: number): void {
  try {
    if (!existsSync(logFile)) return;
    const { size } = statSync(logFile);
    if (size < maxSizeBytes) return;
    const timestamp = new Date().toISOString().replace(/[-:]/g, '').replace(/\..+/, '');
    renameSync(logFile, `${logFile}.${timestamp}.old`);
  } catch {
    // Ignorado: la rotación es best-effort.
  }
}

export function createLogger(options: LoggerOptions = {}): Logger {
  const level = options.level ?? 'info';
  const format = options.format ?? 'text';
  const now = options.now ?? ((): Date => new Date());

  const outputStream = options.stream ?? process.stderr;
  let logFilePath: string | undefined;
  if (options.logFile !== undefined) {
    const maxSize = options.maxLogSizeBytes ?? 10 * 1024 * 1024;
    rotateLogFile(options.logFile, maxSize);
    logFilePath = options.logFile;
  }

  const make = (bindings: LogFields): Logger => {
    const threshold = LEVEL_WEIGHT[level];

    const writeToFile = (text: string): void => {
      if (logFilePath === undefined) return;
      appendFileSync(logFilePath, text);
    };

    const write = (entryLevel: ActiveLevel, event: string, fields: LogFields | undefined): void => {
      if (LEVEL_WEIGHT[entryLevel] > threshold) return;

      const merged = { ...bindings, ...(fields ?? {}) };
      if (format === 'json') {
        const payload = {
          ...merged,
          time: now().toISOString(),
          level: entryLevel,
          event,
        };
        const text = `${safeStringify(payload)}\n`;
        writeToFile(text);
        outputStream.write(text);
        return;
      }

      const style = STYLES[entryLevel];
      const details = formatFields(merged);
      const text = `${style.color(style.icon)} ${event}${details.length === 0 ? '' : ` ${details}`}\n`;
      writeToFile(text);
      outputStream.write(text);
    };

    return {
      level,
      format,
      error: (event, fields) => {
        write('error', event, fields);
      },
      warn: (event, fields) => {
        write('warn', event, fields);
      },
      info: (event, fields) => {
        write('info', event, fields);
      },
      debug: (event, fields) => {
        write('debug', event, fields);
      },
      child: (extra) => make({ ...bindings, ...extra }),
    };
  };

  return make(options.bindings ?? {});
}

function formatFields(fields: LogFields): string {
  return Object.entries(fields)
    .map(([key, value]) => `${key}=${formatField(value)}`)
    .join(' ');
}

function formatField(value: unknown): string {
  if (value instanceof Error) return `${value.name}: ${value.message}`;
  if (typeof value === 'string') {
    return /\s/.test(value) ? safeStringify(value) : value;
  }
  if (typeof value === 'bigint') return value.toString();
  return safeStringify(value);
}

/** Serializa sin romperse con ciclos, errores o `bigint`. */
function safeStringify(value: unknown): string {
  const seen = new WeakSet<object>();
  try {
    return (
      JSON.stringify(value, (_key, item: unknown) => {
        if (item instanceof Error) {
          return { name: item.name, message: item.message, stack: item.stack };
        }
        if (typeof item === 'bigint') return item.toString();
        if (typeof item === 'object' && item !== null) {
          if (seen.has(item)) return '[Circular]';
          seen.add(item);
        }
        return item;
      }) ?? String(value)
    );
  } catch {
    return String(value);
  }
}
