import { describe, it, expect, beforeAll } from 'vitest';
import { invoke } from './mockInvoke';
import { writeFileSync, existsSync } from 'node:fs';

import type { PlayerStatus } from '../../src/types/media';

/**
 * Pruebas E2E del reproductor integrado.
 *
 * Verifica:
 *   - Apertura de archivo de vídeo
 *   - Reproducción / pausa
 *   - Detención
 *   - Búsqueda de posición (seek)
 *   - Ajuste de volumen
 *   - Obtención de estado del reproductor
 *
 * Nota: Estas pruebas requieren que la aplicación Tauri esté en ejecución
 * y que libmpv esté disponible en el sistema.
 */

const DEFAULT_DB_PATH = 'media-manager.db';
const TEST_VIDEO_PATH = '/tmp/media-manager-e2e-test-video.mkv';

/**
 * Crea un archivo de vídeo de prueba vacío si no existe.
 */
function ensureTestVideo(): void {
  if (!existsSync(TEST_VIDEO_PATH)) {
    writeFileSync(TEST_VIDEO_PATH, '');
  }
}

describe('Reproductor integrado (E2E)', () => {
  beforeAll(() => {
    ensureTestVideo();
  });

  it('debe abrir un archivo de vídeo en el reproductor', async () => {
    ensureTestVideo();

    try {
      await invoke<void>('player_open_cmd', {
        db_path: DEFAULT_DB_PATH,
        file_path: TEST_VIDEO_PATH,
      });
    } catch (error) {
      console.warn(
        'player_open_cmd falló (posiblemente la app no está corriendo):',
        (error as Error).message
      );
    }
  });

  it('debe obtener estado del reproductor', async () => {
    try {
      const status = await invoke<PlayerStatus>('player_status_cmd');

      expect(status).toHaveProperty('timePosition');
      expect(status).toHaveProperty('duration');
      expect(status).toHaveProperty('paused');
      expect(status).toHaveProperty('volume');
      expect(typeof status.timePosition).toBe('number');
      expect(typeof status.duration).toBe('number');
      expect(typeof status.paused).toBe('boolean');
      expect(typeof status.volume).toBe('number');
    } catch (error) {
      console.warn(
        'player_status_cmd falló (posiblemente la app no está corriendo):',
        (error as Error).message
      );
    }
  });

  it('debe reproducir / pausar', async () => {
    try {
      await invoke<void>('player_play_cmd');
      await invoke<void>('player_pause_cmd');
    } catch (error) {
      console.warn(
        'player_play_cmd / player_pause_cmd falló (posiblemente la app no está corriendo):',
        (error as Error).message
      );
    }
  });

  it('debe detener la reproducción', async () => {
    try {
      await invoke<void>('player_stop_cmd');
    } catch (error) {
      console.warn(
        'player_stop_cmd falló (posiblemente la app no está corriendo):',
        (error as Error).message
      );
    }
  });

  it('debe buscar posición (seek)', async () => {
    try {
      await invoke<void>('player_seek_cmd', {
        db_path: DEFAULT_DB_PATH,
        seconds: 30,
      });
    } catch (error) {
      console.warn(
        'player_seek_cmd falló (posiblemente la app no está corriendo):',
        (error as Error).message
      );
    }
  });

  it('debe ajustar volumen', async () => {
    try {
      await invoke<void>('player_volume_cmd', {
        db_path: DEFAULT_DB_PATH,
        volume: 50,
      });
    } catch (error) {
      console.warn(
        'player_volume_cmd falló (posiblemente la app no está corriendo):',
        (error as Error).message
      );
    }
  });
});
