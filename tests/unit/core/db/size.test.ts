import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { databaseSizeBytes } from '../../../../src/core/db/size.js';

describe('databaseSizeBytes', () => {
  let workDir: string;
  let dbPath: string;

  beforeEach(async () => {
    workDir = await mkdtemp(path.join(tmpdir(), 'mm-size-'));
    dbPath = path.join(workDir, 'library.db');
  });

  afterEach(async () => {
    await rm(workDir, { recursive: true, force: true });
  });

  it('devuelve 0 para base de datos en memoria', () => {
    expect(databaseSizeBytes(':memory:')).toBe(0);
  });

  it('devuelve 0 para una ruta que no existe', () => {
    expect(databaseSizeBytes('/tmp/archivo-que-no-existe.db')).toBe(0);
  });

  it('devuelve el tamaño de un archivo existente', async () => {
    await writeFile(dbPath, 'contenido de prueba');
    const size = databaseSizeBytes(dbPath);
    expect(size).toBeGreaterThan(0);
  });

  it('suma el tamaño de archivos auxiliares -wal y -shm si existen', async () => {
    await writeFile(dbPath, 'contenido de prueba');
    await writeFile(`${dbPath}-wal`, 'wal');
    await writeFile(`${dbPath}-shm`, 'shm');
    const size = databaseSizeBytes(dbPath);
    expect(size).toBeGreaterThan(0);
    // El tamaño debe incluir al menos los 3 archivos
    expect(size).toBeGreaterThanOrEqual(
      Buffer.byteLength('contenido de prueba') +
        Buffer.byteLength('wal') +
        Buffer.byteLength('shm'),
    );
  });
});
