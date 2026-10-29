import { Writable } from 'node:stream';

import { describe, expect, it } from 'vitest';

import { reportFatalError } from '../../../src/commands/errorHandler.js';
import { NotFoundError } from '../../../src/shared/errors.js';

function createCollector(): { stream: Writable; output: () => string } {
  let buffer = '';
  const stream = new Writable({
    write(chunk: unknown, _encoding, callback) {
      buffer += String(chunk);
      callback();
    },
  });
  return { stream, output: () => buffer };
}

describe('reportFatalError', () => {
  it('escribe el código y el mensaje en stderr', () => {
    const collector = createCollector();

    reportFatalError(new NotFoundError('No existe «x»'), { stream: collector.stream });

    expect(collector.output()).toContain('[NOT_FOUND]');
    expect(collector.output()).toContain('No existe «x»');
  });

  it('normaliza errores inesperados a APP_ERROR', () => {
    const collector = createCollector();

    reportFatalError(new Error('boom'), { stream: collector.stream });

    expect(collector.output()).toContain('[APP_ERROR]');
    expect(collector.output()).toContain('boom');
  });

  it('con --verbose añade la pila del error', () => {
    const collector = createCollector();
    const error = new Error('boom');

    reportFatalError(error, { stream: collector.stream, verbose: true });

    expect(collector.output()).toContain(error.stack ?? '');
  });
});
