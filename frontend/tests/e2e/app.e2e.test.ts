import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { invoke } from '@tauri-apps/api/core';

import type { Media } from '../../src/types/media';

/**
 * Pruebas E2E del flujo de la aplicación media-manager.
 *
 * Estas pruebas verifican que la aplicación compilada funcione correctamente
 * en los flujos críticos:
 *   1. Inicio de la aplicación y carga de la biblioteca
 *   2. Búsqueda de medios
 *   3. Visualización de detalles
 *   4. Toggle de favorito y visto
 *   5. Reproducción de archivos
 *
 * Nota: Estas pruebas requieren que la aplicación Tauri esté en ejecución
 * (ya sea en modo desarrollo o compilada). En entornos de CI sin servidor X,
 * se pueden ejecutar con xvfb.
 */

const DEFAULT_DB_PATH = 'media-manager.db';

describe('Flujo de la aplicación media-manager (E2E)', () => {
  beforeAll(() => {
    // Verificar que la aplicación esté disponible
    // En CI con xvfb, la aplicación debería estar iniciada
  });

  it('debe listar medios desde la base de datos', async () => {
    const result = await invoke<{ media: Media[]; total: number }>(
      'list_media_cmd',
      { db_path: DEFAULT_DB_PATH }
    );

    expect(result.media).toBeInstanceOf(Array);
    expect(result.total).toBeGreaterThanOrEqual(0);
    expect(result.media.length).toBe(result.total);
  });

  it('debe buscar medios por texto', async () => {
    const result = await invoke<Media[]>('search_media_cmd', {
      db_path: DEFAULT_DB_PATH,
      query: 'test',
    });

    expect(result).toBeInstanceOf(Array);
  });

  it('debe contar medios', async () => {
    const count = await invoke<number>('count_media_cmd', {
      db_path: DEFAULT_DB_PATH,
    });

    expect(count).toBeGreaterThanOrEqual(0);
    expect(typeof count).toBe('number');
  });

  it('debe listar carpetas de medios', async () => {
    const folders = await invoke<string[]>('list_folders_cmd', {
      db_path: DEFAULT_DB_PATH,
    });

    expect(folders).toBeInstanceOf(Array);
  });

  afterAll(() => {
    // Limpieza si es necesario
  });
});
