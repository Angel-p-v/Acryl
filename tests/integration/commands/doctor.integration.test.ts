import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import Database from 'better-sqlite3';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { isDatabaseMigrated, runDoctorCommand } from '../../../src/commands/doctor.js';
import { runMigrations } from '../../../src/core/db/connection.js';
import { migrations } from '../../../src/core/db/migrations/index.js';

const FIXTURES = fileURLToPath(new URL('../../fixtures/library', import.meta.url));

describe('mm doctor (integración)', () => {
  let workDir: string;
  let configPath: string;

  beforeEach(async () => {
    workDir = await mkdtemp(path.join(tmpdir(), 'mm-doctor-'));
    configPath = path.join(workDir, 'config.yaml');
    const quote = (value: string): string => `'${value.replaceAll("'", "''")}'`;

    await writeFile(
      configPath,
      [
        'library:',
        '  roots:',
        `    - path: ${quote(path.join(FIXTURES, 'series'))}`,
        '      type: series',
        '',
        'database:',
        `  path: ${quote(path.join(workDir, 'library.db'))}`,
        '',
      ].join('\n'),
      'utf8',
    );
  });

  afterEach(async () => {
    await rm(workDir, { recursive: true, force: true });
  });

  async function captureDoctor(json = true): Promise<string> {
    const lines: string[] = [];
    const spy = vi.spyOn(console, 'log').mockImplementation((...args: unknown[]) => {
      lines.push(args.map((arg) => String(arg)).join(' '));
    });
    try {
      await runDoctorCommand({ config: configPath, json });
    } finally {
      spy.mockRestore();
    }
    return lines.join('\n');
  }

  it('reporta migrated: true cuando la BD tiene todas las migraciones aplicadas', async () => {
    const output = await captureDoctor();
    const parsed = JSON.parse(output);

    expect(parsed.database.migrated).toBe(true);
    expect(parsed.database.integrity).toBe('ok');
    expect(parsed.database.foreignKeys).toBe('ok');
    expect(parsed.status).toBe('ok');
  });

  it('reporta migrated: false cuando la BD tiene migraciones pendientes', async () => {
    const dbPath = path.join(workDir, 'pending.db');
    const db = new Database(dbPath);
    try {
      // Solo aplicar la migración v1 (init), dejando el resto pendientes
      const v1 = migrations.find((m) => m.version === 1);
      expect(v1).toBeDefined();
      runMigrations(db, [v1!], { dbPath });

      expect(isDatabaseMigrated(db)).toBe(false);
    } finally {
      db.close();
      await rm(dbPath, { force: true, recursive: true });
    }
  });

  it('produce salida legible en modo texto (no JSON)', async () => {
    const output = await captureDoctor(false);

    expect(output).toContain('Biblioteca sana');
  });
});
