import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { closeDatabase, openDatabase } from '../../../src/core/db/connection.js';
import { createDatabaseMaintenance } from '../../../src/core/db/maintenance.js';
import { createMediaRepository } from '../../../src/core/db/repositories/mediaRepo.js';
import { databaseSizeBytes } from '../../../src/core/db/size.js';

describe('mantenimiento de SQLite (integración)', () => {
  let workDir: string;
  let databasePath: string;

  beforeEach(async () => {
    workDir = await mkdtemp(path.join(tmpdir(), 'mm-maintenance-'));
    databasePath = path.join(workDir, 'library.db');
  });

  afterEach(async () => {
    await rm(workDir, { recursive: true, force: true });
  });

  it('ejecuta PRAGMA optimize y VACUUM conservando los datos', () => {
    const db = openDatabase(databasePath);
    const mediaRepo = createMediaRepository(db);
    const maintenance = createDatabaseMaintenance(db);

    try {
      for (let index = 0; index < 200; index += 1) {
        mediaRepo.upsert({
          type: 'movie',
          title: `Temporary ${index}`,
          year: 2020,
          folderPath: `/library/temporary-${index}`,
        });
      }

      for (let index = 0; index < 150; index += 1) {
        mediaRepo.remove(index + 1);
      }

      maintenance.optimize();
      maintenance.vacuum();

      expect(mediaRepo.count()).toBe(50);
      expect(databaseSizeBytes(databasePath)).toBeGreaterThan(0);
    } finally {
      closeDatabase(db);
    }
  });
});
