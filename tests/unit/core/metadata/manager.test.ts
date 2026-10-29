import { describe, it, expect } from 'vitest';

import { DefaultMetadataProvider } from '../../../../src/core/metadata/default-provider.js';
import { createMetadataManager } from '../../../../src/core/metadata/manager.js';

describe('DefaultMetadataProvider', () => {
  it('está siempre configurado (plug & play)', () => {
    const provider = new DefaultMetadataProvider();
    expect(provider.isConfigured).toBe(true);
    expect(provider.name).toBe('default');
  });

  it('search devuelve al menos un resultado básico sin fallar', async () => {
    const provider = new DefaultMetadataProvider();
    const result = await provider.search('breaking bad');

    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.data.length).toBeGreaterThan(0);
      expect(result.data[0]!.title).toBe('breaking bad');
    }
  });

  it('search con consulta vacía devuelve error', async () => {
    const provider = new DefaultMetadataProvider();
    const result = await provider.search('');

    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error.code).toBe('empty_query');
    }
  });

  it('getDetails devuelve resultado básico sin fallar', async () => {
    const provider = new DefaultMetadataProvider();
    const result = await provider.getDetails(123, 'movie');

    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.data.externalId).toBe(123);
    }
  });
});

describe('createMetadataManager', () => {
  it('usa el proveedor por defecto cuando no hay externos configurados', () => {
    const manager = createMetadataManager({
      providers: [],
      defaultProvider: new DefaultMetadataProvider(),
    });

    expect(manager.hasExternalProvider()).toBe(false);
    const resolution = manager.resolveProvider();
    expect(resolution.provider.name).toBe('default');
    expect(resolution.isDefault).toBe(true);
  });

  it('search devuelve metadatos de inmediato sin API keys', async () => {
    const manager = createMetadataManager({
      providers: [],
      defaultProvider: new DefaultMetadataProvider(),
    });

    const result = await manager.search('seinfeld', { limit: 5 });

    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.data.length).toBeGreaterThan(0);
      expect(result.data[0]!.title).toContain('seinfeld');
    }
  });

  it('searchAndEnrich devuelve enriquecimiento con proveedor default', async () => {
    const manager = createMetadataManager({
      providers: [],
      defaultProvider: new DefaultMetadataProvider(),
    });

    const result = await manager.searchAndEnrich('friends', 'tv');

    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.data.provider).toBe('default');
      expect(result.data.title).toBe('friends');
    }
  });
});
