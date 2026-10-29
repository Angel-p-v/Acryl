import { describe, expect, it } from 'vitest';

import { backupTimestamp } from '../../../src/core/db/backup.js';

describe('backupTimestamp', () => {
  it('genera una marca compacta en UTC con milisegundos', () => {
    expect(backupTimestamp(new Date('2026-10-23T10:15:00.000Z'))).toBe('20261023T101500000Z');
    expect(backupTimestamp(new Date('2026-10-23T10:15:00.123Z'))).toBe('20261023T101500123Z');
  });

  it('ordena alfabéticamente en el orden temporal', () => {
    const first = backupTimestamp(new Date('2026-10-23T10:15:00.100Z'));
    const second = backupTimestamp(new Date('2026-10-23T10:15:00.900Z'));

    expect(first).toBe('20261023T101500100Z');
    expect(second).toBe('20261023T101500900Z');
    expect(second > first).toBe(true);
  });
});
