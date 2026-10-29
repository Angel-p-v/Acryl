import { mkdtemp, rm, writeFile, utimes, stat } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
  cachePoster,
  clearPosterCache,
  getCachedPoster,
  hasValidMagicBytes,
  isAllowedContentType,
  sanitizeFileName,
} from '../../../../src/core/metadata/posterCache.js';

import type { Logger } from '../../../../src/shared/logger.js';

const logger = {
  debug: vi.fn(),
  warn: vi.fn(),
  error: vi.fn(),
  info: vi.fn(),
  level: 'debug' as const,
  format: 'text' as const,
  child: vi.fn(),
};

describe('posterCache', () => {
  let workDir: string;

  beforeEach(async () => {
    workDir = await mkdtemp(path.join(tmpdir(), 'mm-poster-'));
  });

  afterEach(async () => {
    await rm(workDir, { recursive: true, force: true });
  });

  it('cachePoster devuelve null para posterUrl nulo', async () => {
    const result = await cachePoster(null, 'black-torch', 'tv', logger);
    expect(result).toBe(null);
  });

  it('sanitizeFileName elimina etiquetas, temporada/episodio y calidad', () => {
    expect(sanitizeFileName('Black Torch [Judas] S01E01 1080p', 'tv')).toBe('black-torch-tv');
    expect(sanitizeFileName('Spider-Man: Into the Spider-Verse', 'movie')).toBe(
      'spider-man-into-the-spider-verse-movie',
    );
    expect(sanitizeFileName('Cowboy Bebop', 'anime')).toBe('cowboy-bebop-anime');
    expect(sanitizeFileName('  ', 'tv')).toBe('tv');
  });

  it('getCachedPoster devuelve null cuando no hay caché', async () => {
    const result = await getCachedPoster('black-torch', 'tv', workDir);
    expect(result).toBe(null);
  });

  it('getCachedPoster devuelve la ruta del póster almacenado', async () => {
    await writeFile(path.join(workDir, 'black-torch-tv.jpg'), 'fake-image-data');

    const result = await getCachedPoster('black-torch', 'tv', workDir);

    expect(result).toBe(path.join(workDir, 'black-torch-tv.jpg'));
  });

  it('getCachedPoster ignora archivos de otros títulos', async () => {
    await writeFile(path.join(workDir, 'black-torch-tv.jpg'), 'fake-image-data');
    await writeFile(
      path.join(workDir, 'spider-man-into-the-spider-verse-movie.jpg'),
      'other-image',
    );

    const result = await getCachedPoster('black-torch', 'tv', workDir);

    expect(result).toBe(path.join(workDir, 'black-torch-tv.jpg'));
  });

  it('clearPosterCache devuelve 0 cuando no hay archivos', async () => {
    const result = await clearPosterCache(workDir, 30);

    expect(result.deleted).toBe(0);
    expect(result.errors).toEqual([]);
  });

  it('clearPosterCache borra archivos más viejos que maxAgeDays', async () => {
    const oldFile = path.join(workDir, 'black-torch-tv.jpg');
    await writeFile(oldFile, 'old-image-data');

    // Establecer el mtime a 40 días atrás (más viejo que maxAgeDays=30)
    const fortyDaysAgo = Date.now() - 40 * 24 * 60 * 60 * 1000;
    await utimes(oldFile, fortyDaysAgo / 1000, fortyDaysAgo / 1000);

    const result = await clearPosterCache(workDir, 30);

    expect(result.deleted).toBe(1);
    expect(result.errors).toEqual([]);

    // Verificar que el archivo ya no existe
    await expect(stat(oldFile)).rejects.toMatchObject({ code: 'ENOENT' });
  });

  it('clearPosterCache no borra archivos más recientes que maxAgeDays', async () => {
    const newFile = path.join(workDir, 'cowboy-bebop-anime.jpg');
    await writeFile(newFile, 'new-image-data');

    // mtime ahora (más reciente que maxAgeDays=30)
    const now = Date.now() / 1000;
    await utimes(newFile, now, now);

    const result = await clearPosterCache(workDir, 30);

    expect(result.deleted).toBe(0);
    expect(result.errors).toEqual([]);

    // Verificar que el archivo todavía existe
    await expect(stat(newFile)).resolves.toBeDefined();
  });

  it('clearPosterCache con maxAgeDays 0 borra todos los archivos', async () => {
    await writeFile(path.join(workDir, 'a-tv.jpg'), 'a');
    await writeFile(path.join(workDir, 'b-movie.jpg'), 'b');

    const result = await clearPosterCache(workDir, 0);

    expect(result.deleted).toBe(2);
    expect(result.errors).toEqual([]);
  });

  describe('isAllowedContentType', () => {
    it('devuelve true para Content-Type de imagen válido', () => {
      expect(isAllowedContentType('image/jpeg')).toBe(true);
      expect(isAllowedContentType('image/png')).toBe(true);
      expect(isAllowedContentType('image/webp')).toBe(true);
      expect(isAllowedContentType('image/gif')).toBe(true);
      expect(isAllowedContentType('image/jpg; charset=utf-8')).toBe(true);
    });

    it('devuelve false para Content-Type no imagen o nulo', () => {
      expect(isAllowedContentType(null)).toBe(false);
      expect(isAllowedContentType('text/html')).toBe(false);
      expect(isAllowedContentType('application/json')).toBe(false);
      expect(isAllowedContentType('')).toBe(false);
    });
  });

  describe('hasValidMagicBytes', () => {
    it('devuelve true para JPEG con magic bytes válidos', () => {
      const jpeg = Buffer.from([0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10]);
      expect(hasValidMagicBytes(jpeg, '.jpg')).toBe(true);
      expect(hasValidMagicBytes(jpeg, '.jpeg')).toBe(true);
    });

    it('devuelve true para PNG con magic bytes válidos', () => {
      const png = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a]);
      expect(hasValidMagicBytes(png, '.png')).toBe(true);
    });

    it('devuelve true para WEBP con magic bytes y marca válidos', () => {
      const webp = Buffer.from([
        0x52,
        0x49,
        0x46,
        0x46, // RIFF
        0x00,
        0x00,
        0x00,
        0x00, // size
        0x57,
        0x45,
        0x42,
        0x50, // WEBP
      ]);
      expect(hasValidMagicBytes(webp, '.webp')).toBe(true);
    });

    it('devuelve false para WEBP sin marca WEBP', () => {
      const notWebp = Buffer.from([
        0x52,
        0x49,
        0x46,
        0x46, // RIFF
        0x00,
        0x00,
        0x00,
        0x00, // size
        0x41,
        0x56,
        0x49,
        0x20, // AVI  (no WEBP)
      ]);
      expect(hasValidMagicBytes(notWebp, '.webp')).toBe(false);
    });

    it('devuelve false para GIF con magic bytes válidos', () => {
      const gif = Buffer.from([0x47, 0x49, 0x46, 0x38, 0x39, 0x61]);
      expect(hasValidMagicBytes(gif, '.gif')).toBe(true);
    });

    it('devuelve false para buffer vacío o demasiado corto', () => {
      expect(hasValidMagicBytes(Buffer.from([0xff]), '.jpg')).toBe(false);
      expect(hasValidMagicBytes(Buffer.alloc(0), '.jpg')).toBe(false);
    });

    it('devuelve false para extensión desconocida', () => {
      expect(hasValidMagicBytes(Buffer.from([0xff, 0xd8, 0xff]), '.bmp')).toBe(false);
    });

    it('devuelve false para contenido que no es imagen', () => {
      const html = Buffer.from('<html><body>fake</body></html>');
      expect(hasValidMagicBytes(html, '.jpg')).toBe(false);
    });
  });

  describe('cachePoster rechaza descargas inválidas', () => {
    const logger = {
      debug: vi.fn(),
      warn: vi.fn(),
      error: vi.fn(),
      info: vi.fn(),
      level: 'debug',
      format: 'text',
      child: vi.fn(),
    } satisfies Logger;

    beforeEach(function () {
      const originalFetch = globalThis.fetch;
      vi.stubGlobal(
        'fetch',
        vi.fn(async (url: string) => {
          if (String(url).includes('invalid-type')) {
            return new Response('<html>error</html>', {
              headers: { 'content-type': 'text/html' },
            });
          }
          if (String(url).includes('tiny-file')) {
            return new Response('x'.repeat(50), {
              headers: { 'content-type': 'image/jpeg' },
            });
          }
          return originalFetch(url);
        }),
      );
    });

    afterEach(function () {
      vi.unstubAllGlobals();
    });

    it('cachePoster rechaza Content-Type inválido', async () => {
      const result = await cachePoster(
        'https://example.com/invalid-type.jpg',
        'test-title',
        'tv',
        logger,
        workDir,
      );
      expect(result).toBe(null);
    });

    it('cachePoster rechaza archivo demasiado pequeño (< 100 bytes)', async () => {
      const result = await cachePoster(
        'https://example.com/tiny-file.jpg',
        'test-title',
        'tv',
        logger,
        workDir,
      );
      expect(result).toBe(null);
    });
  });
});
