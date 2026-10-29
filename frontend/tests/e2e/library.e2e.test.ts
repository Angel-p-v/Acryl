import { describe, it, expect, beforeAll } from 'vitest';
import { invoke } from './mockInvoke';

import type { Media, MediaDetails } from '../../src/types/media';

/**
 * Pruebas E2E de la biblioteca de medios.
 *
 * Verifica:
 *   - Búsqueda de medios por título
 * *   - Visualización de detalles completos (medio + archivos)
 *   - Toggle de favorito
 *   - Toggle de visto
 */

const DEFAULT_DB_PATH = 'media-manager.db';

describe('Biblioteca de medios (E2E)', () => {
  let testMediaId: number | null = null;

  beforeAll(async () => {
    // Insertar un medio de prueba para las pruebas de detalles
    const media = await invoke<Media[]>('search_media_cmd', {
      db_path: DEFAULT_DB_PATH,
      query: 'Arrival',
      limit: 1,
    });

    if (media.length > 0) {
      testMediaId = media[0].id;
    }
  });

  it('debe buscar medios por título', async () => {
    const result = await invoke<Media[]>('search_media_cmd', {
      db_path: DEFAULT_DB_PATH,
      query: 'Arrival',
      limit: 5,
    });

    expect(result).toBeInstanceOf(Array);
    expect(result.length).toBeGreaterThan(0);
    expect(result[0].title).toContain('Arrival');
  });

  it('debe buscar medios por tipo', async () => {
    const movies = await invoke<Media[]>('search_media_cmd', {
      db_path: DEFAULT_DB_PATH,
      query: '',
      type: 'movie',
      limit: 10,
    });

    expect(movies).toBeInstanceOf(Array);
    movies.forEach((m) => {
      expect(m.type).toBe('movie');
    });
  });

  it('debe obtener detalles de un medio', async () => {
    if (!testMediaId) {
      // Si no hay medio de prueba, buscar uno
      const media = await invoke<Media[]>('search_media_cmd', {
        db_path: DEFAULT_DB_PATH,
        query: '',
        limit: 1,
      });
      if (media.length > 0) {
        testMediaId = media[0].id;
      }
    }

    if (testMediaId) {
      const details = await invoke<MediaDetails | null>('get_media_details_cmd', {
        db_path: DEFAULT_DB_PATH,
        media_id: testMediaId,
      });

      expect(details).not.toBeNull();
      if (details) {
        expect(details.media.id).toBe(testMediaId);
        expect(details.files).toBeInstanceOf(Array);
      }
    } else {
      // Si no hay medios, el comando debe devolver null o manejarse
      const details = await invoke<MediaDetails | null>('get_media_details_cmd', {
        db_path: DEFAULT_DB_PATH,
        media_id: 999999,
      });
      expect(details).toBeNull();
    }
  });

  it('debe alternar estado de favorito', async () => {
    if (!testMediaId) {
      const media = await invoke<Media[]>('search_media_cmd', {
        db_path: DEFAULT_DB_PATH,
        query: '',
        limit: 1,
      });
      if (media.length > 0) {
        testMediaId = media[0].id;
      }
    }

    if (testMediaId) {
      // Obtener estado actual
      const detailsBefore = await invoke<MediaDetails | null>('get_media_details_cmd', {
        db_path: DEFAULT_DB_PATH,
        media_id: testMediaId,
      });
      const initialFavorite = detailsBefore?.media.is_favorite ?? false;

      // Alternar favorito
      const afterToggle = await invoke<boolean>('toggle_favorite_cmd', {
        db_path: DEFAULT_DB_PATH,
        media_id: testMediaId,
      });
      expect(typeof afterToggle).toBe('boolean');
      expect(afterToggle).toBe(!initialFavorite);

      // Alternar de nuevo para restaurar
      const afterToggleBack = await invoke<boolean>('toggle_favorite_cmd', {
        db_path: DEFAULT_DB_PATH,
        media_id: testMediaId,
      });
      expect(afterToggleBack).toBe(initialFavorite);
    }
  });

  it('debe alternar estado de visto', async () => {
    if (!testMediaId) {
      const media = await invoke<Media[]>('search_media_cmd', {
        db_path: DEFAULT_DB_PATH,
        query: '',
        limit: 1,
      });
      if (media.length > 0) {
        testMediaId = media[0].id;
      }
    }

    if (testMediaId) {
      // Obtener estado actual
      const detailsBefore = await invoke<MediaDetails | null>('get_media_details_cmd', {
        db_path: DEFAULT_DB_PATH,
        media_id: testMediaId,
      });
      const initialWatched = detailsBefore?.media.is_watched ?? false;

      // Alternar visto
      const afterToggle = await invoke<boolean>('toggle_watched_cmd', {
        db_path: DEFAULT_DB_PATH,
        media_id: testMediaId,
      });
      expect(typeof afterToggle).toBe('boolean');
      expect(afterToggle).toBe(!initialWatched);

      // Alternar de nuevo para restaurar
      const afterToggleBack = await invoke<boolean>('toggle_watched_cmd', {
        db_path: DEFAULT_DB_PATH,
        media_id: testMediaId,
      });
      expect(afterToggleBack).toBe(initialWatched);
    }
  });
});
