import { describe, expect, it } from 'vitest';

import { metrics } from '../../../src/core/metrics.js';

describe('Metrics', () => {
  it('inicia con todos los contadores en cero', () => {
    metrics.reset();
    const s = metrics.snapshot();
    expect(s.indexations).toBe(0);
    expect(s.metadataErrors).toBe(0);
    expect(s.totalQueryTimeMs).toBe(0);
    expect(s.queryCount).toBe(0);
    expect(s.averageQueryTimeMs).toBe(0);
  });

  it('registra indexaciones', () => {
    metrics.reset();
    metrics.recordIndexation();
    metrics.recordIndexation();
    expect(metrics.snapshot().indexations).toBe(2);
  });

  it('registra errores de metadatos', () => {
    metrics.reset();
    metrics.recordMetadataError();
    metrics.recordMetadataError();
    metrics.recordMetadataError();
    expect(metrics.snapshot().metadataErrors).toBe(3);
  });

  it('registra tiempos de consulta y calcula promedio', () => {
    metrics.reset();
    metrics.recordQueryTime(100);
    metrics.recordQueryTime(200);
    metrics.recordQueryTime(300);
    const s = metrics.snapshot();
    expect(s.totalQueryTimeMs).toBe(600);
    expect(s.queryCount).toBe(3);
    expect(s.averageQueryTimeMs).toBe(200);
  });

  it('ignora tiempos de consulta negativos', () => {
    metrics.reset();
    metrics.recordQueryTime(-10);
    metrics.recordQueryTime(100);
    const s = metrics.snapshot();
    expect(s.totalQueryTimeMs).toBe(100);
    expect(s.queryCount).toBe(1);
    expect(s.averageQueryTimeMs).toBe(100);
  });

  it('promedio es cero cuando no hay consultas', () => {
    metrics.reset();
    expect(metrics.snapshot().averageQueryTimeMs).toBe(0);
  });

  it('reset deja todos los contadores en cero', () => {
    metrics.recordIndexation();
    metrics.recordMetadataError();
    metrics.recordQueryTime(50);
    metrics.reset();
    const s = metrics.snapshot();
    expect(s.indexations).toBe(0);
    expect(s.metadataErrors).toBe(0);
    expect(s.totalQueryTimeMs).toBe(0);
    expect(s.queryCount).toBe(0);
    expect(s.averageQueryTimeMs).toBe(0);
  });
});
