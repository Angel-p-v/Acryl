import process from 'node:process';

import pc from 'picocolors';

import { describeError } from '../shared/errors.js';

export type FatalErrorOptions = {
  /** Incluye la pila del error, sin mezclarla con `stdout`. */
  verbose?: boolean;
  stream?: NodeJS.WritableStream;
};

/**
 * Punto único de salida para errores no controlados de la CLI.
 *
 * Siempre escribe en `stderr`, mantiene el código estable del error delante y no
 * cambia el contrato de códigos de salida (1 para errores de ejecución). La pila
 * solo aparece con `--verbose`.
 */
export function reportFatalError(error: unknown, options: FatalErrorOptions = {}): void {
  const stream = options.stream ?? process.stderr;
  const info = describeError(error);

  stream.write(`${pc.red('✖')} ${pc.bold(`[${info.code}]`)} ${info.message}\n`);

  if (options.verbose === true && error instanceof Error && error.stack !== undefined) {
    stream.write(`${pc.dim(error.stack)}\n`);
  }
}
