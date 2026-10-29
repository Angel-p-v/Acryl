/* eslint-disable @typescript-eslint/unbound-method */

import { describe, expect, it, vi } from 'vitest';

import {
  createDatabaseMaintenance,
  optimizeDatabase,
  vacuumDatabase,
} from '../../../../src/core/db/maintenance.js';
import { DbError } from '../../../../src/shared/errors.js';

import type { Db } from '../../../../src/core/db/connection.js';

function createDb(overrides: Partial<Pick<Db, 'pragma' | 'exec'>> = {}): Db {
  return {
    pragma: vi.fn(),
    exec: vi.fn(),
    ...overrides,
  } as Db;
}

function throwingPragma(): Db['pragma'] {
  return vi.fn(() => {
    throw new Error('disco lleno');
  });
}

function throwingExec(): Db['exec'] {
  return vi.fn(() => {
    throw new Error('sin espacio');
  });
}

function throwingPragmaCheckpoint(): Db['pragma'] {
  return vi.fn(() => {
    throw new Error('bloqueo');
  });
}

describe('mantenimiento de base de datos (unidad)', () => {
  describe('createDatabaseMaintenance', () => {
    it('devuelve objeto con optimize y vacuum', () => {
      const db = createDb();
      const maintenance = createDatabaseMaintenance(db);

      expect(maintenance).toHaveProperty('optimize');
      expect(maintenance).toHaveProperty('vacuum');
      expect(typeof maintenance.optimize).toBe('function');
      expect(typeof maintenance.vacuum).toBe('function');
    });

    it('optimize llama a optimizeDatabase', () => {
      const db = createDb();
      const maintenance = createDatabaseMaintenance(db);

      maintenance.optimize();

      expect(db.pragma).toHaveBeenCalledWith('optimize');
    });

    it('vacuum llama a vacuumDatabase', () => {
      const db = createDb();
      const maintenance = createDatabaseMaintenance(db);

      maintenance.vacuum();

      expect(db.pragma).toHaveBeenCalledWith('wal_checkpoint(TRUNCATE)');
      expect(db.exec).toHaveBeenCalledWith('VACUUM');
    });
  });

  describe('optimizeDatabase', () => {
    it('ejecuta PRAGMA optimize sin errores', () => {
      const db = createDb();

      expect(() => optimizeDatabase(db)).not.toThrow();
      expect(db.pragma).toHaveBeenCalledWith('optimize');
    });

    it('lanza DbError cuando PRAGMA optimize falla', () => {
      const db = createDb({ pragma: throwingPragma() });

      expect(() => optimizeDatabase(db)).toThrow(DbError);
      expect(db.pragma).toHaveBeenCalledWith('optimize');
    });
  });

  describe('vacuumDatabase', () => {
    it('ejecuta wal_checkpoint y VACUUM sin errores', () => {
      const db = createDb();

      expect(() => vacuumDatabase(db)).not.toThrow();
      expect(db.pragma).toHaveBeenCalledWith('wal_checkpoint(TRUNCATE)');
      expect(db.exec).toHaveBeenCalledWith('VACUUM');
    });

    it('lanza DbError cuando wal_checkpoint falla', () => {
      const db = createDb({ pragma: throwingPragmaCheckpoint() });

      expect(() => vacuumDatabase(db)).toThrow(DbError);
      expect(db.pragma).toHaveBeenCalledWith('wal_checkpoint(TRUNCATE)');
    });

    it('lanza DbError cuando VACUUM falla', () => {
      const db = createDb({ exec: throwingExec() });

      expect(() => vacuumDatabase(db)).toThrow(DbError);
      expect(db.exec).toHaveBeenCalledWith('VACUUM');
    });
  });
});
