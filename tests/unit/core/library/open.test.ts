import { describe, expect, it, vi } from 'vitest';

import { openLibrary } from '../../../../src/core/library/open.js';

vi.mock('../../../../src/core/db/connection.js', () => ({
  openDatabase: vi.fn(() => ({
    pragma: vi.fn(),
    close: vi.fn(),
    exec: vi.fn(),
    prepare: vi.fn(() => ({
      run: vi.fn(),
      all: vi.fn(() => []),
      get: vi.fn(() => undefined),
    })),
  })),
  closeDatabase: vi.fn(),
}));

vi.mock('../../../../src/core/db/repositories/mediaRepo.js', () => ({
  createMediaRepository: vi.fn(() => ({})),
}));

vi.mock('../../../../src/core/db/repositories/fileRepo.js', () => ({
  createFileRepository: vi.fn(() => ({})),
}));

vi.mock('../../../../src/core/db/repositories/indexRunRepo.js', () => ({
  createIndexRunRepository: vi.fn(() => ({})),
}));

vi.mock('../../../../src/core/db/repositories/playbackRepo.js', () => ({
  createPlaybackRepository: vi.fn(() => ({})),
}));

vi.mock('../../../../src/core/db/repositories/preferencesRepo.js', () => ({
  createPreferenceRepository: vi.fn(() => ({})),
}));

vi.mock('../../../../src/core/db/repositories/tagRepo.js', () => ({
  createTagRepository: vi.fn(() => ({})),
}));

vi.mock('../../../../src/core/db/repositories/metadataRepo.js', () => ({
  createMetadataRepository: vi.fn(() => ({})),
}));

vi.mock('../../../../src/core/db/maintenance.js', () => ({
  createDatabaseMaintenance: vi.fn(() => ({ optimize: vi.fn(), vacuum: vi.fn() })),
}));

vi.mock('../../../../src/core/library/service.js', () => ({
  createLibraryService: vi.fn(() => ({})),
}));

vi.mock('../../../../src/core/metadata/service.js', () => ({
  MetadataService: vi.fn(),
}));

describe('openLibrary', () => {
  it('abre la biblioteca sin config ni logger (rama else)', () => {
    const opened = openLibrary(':memory:');

    expect(opened).toBeDefined();
    expect(opened.repositories).toBeDefined();
    expect(opened.library).toBeDefined();
    expect(opened.metadataService).toBeDefined();
    expect(opened.metadataConfig).toEqual({ cacheTtlHours: 24, autoFetch: false });
    expect(typeof opened.close).toBe('function');

    opened.close();
  });

  it('abre la biblioteca con config y logger (rama if)', () => {
    const config = {
      metadata: { tmdbApiKey: 'test-key', cacheTtlHours: 48 },
    } as { metadata?: { tmdbApiKey?: string; tvdbApiKey?: string; cacheTtlHours?: number } };
    const logger = {
      debug: vi.fn(),
      warn: vi.fn(),
      error: vi.fn(),
      info: vi.fn(),
      level: 'debug' as const,
      format: 'text' as const,
      child: vi.fn(),
    };

    const opened = openLibrary(':memory:', config as never, logger);

    expect(opened.metadataConfig.cacheTtlHours).toBe(48);
    expect(opened.metadataService).toBeDefined();

    opened.close();
  });
});
