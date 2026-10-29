import { mkdtemp, writeFile, readdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { describe, expect, it } from 'vitest';

import {
  findSubtitleFile,
  findSubtitleFiles,
  isSubtitleFile,
} from '../../../src/core/player/subtitles.js';

async function createFiles(dir: string, names: string[]): Promise<void> {
  await Promise.all(names.map((name) => writeFile(path.join(dir, name), '', 'utf8')));
}

describe('isSubtitleFile', () => {
  it('reconoce archivos .srt', () => {
    expect(isSubtitleFile('/a/episodio.srt')).toBe(true);
  });

  it('reconoce archivos .sub', () => {
    expect(isSubtitleFile('/a/episodio.sub')).toBe(true);
  });

  it('reconoce archivos .vtt', () => {
    expect(isSubtitleFile('/a/episodio.vtt')).toBe(true);
  });

  it('reconoce archivos .ass y .ssa', () => {
    expect(isSubtitleFile('/a/episodio.ass')).toBe(true);
    expect(isSubtitleFile('/a/episodio.ssa')).toBe(true);
  });

  it('rechaza archivos de vídeo', () => {
    expect(isSubtitleFile('/a/episodio.mkv')).toBe(false);
    expect(isSubtitleFile('/a/episodio.mp4')).toBe(false);
    expect(isSubtitleFile('/a/episodio.avi')).toBe(false);
  });

  it('es insensible a mayúsculas en la extensión', () => {
    expect(isSubtitleFile('/a/episodio.SRT')).toBe(true);
    expect(isSubtitleFile('/a/episodio.Srt')).toBe(true);
  });
});

describe('findSubtitleFiles', () => {
  it('devuelve [] cuando no hay subtítulos asociados', async () => {
    const dir = await mkdtemp(path.join(tmpdir(), 'mm-subs-'));
    await createFiles(dir, ['episodio.mkv', 'otro.avi']);

    const result = findSubtitleFiles(path.join(dir, 'episodio.mkv'));
    expect(result).toEqual([]);
  });

  it('encuentra .srt con el mismo nombre base', async () => {
    const dir = await mkdtemp(path.join(tmpdir(), 'mm-subs-'));
    await createFiles(dir, ['episodio.mkv', 'episodio.srt']);

    const result = findSubtitleFiles(path.join(dir, 'episodio.mkv'));
    expect(result).toEqual([path.join(dir, 'episodio.srt')]);
  });

  it('encuentra varios subtítulos (mismo nombre base con sufijos)', async () => {
    const dir = await mkdtemp(path.join(tmpdir(), 'mm-subs-'));
    await createFiles(dir, ['episodio.mkv', 'episodio.srt', 'episodio.en.srt', 'episodio.es.srt']);

    const result = findSubtitleFiles(path.join(dir, 'episodio.mkv'));
    expect(result).toEqual([
      path.join(dir, 'episodio.en.srt'),
      path.join(dir, 'episodio.es.srt'),
      path.join(dir, 'episodio.srt'),
    ]);
  });

  it('no devuelve subtítulos de otros vídeos', async () => {
    const dir = await mkdtemp(path.join(tmpdir(), 'mm-subs-'));
    await createFiles(dir, ['episodio1.mkv', 'episodio1.srt', 'episodio2.mkv']);

    const result = findSubtitleFiles(path.join(dir, 'episodio2.mkv'));
    expect(result).toEqual([]);
  });

  it('ignora archivos que no son de subtítulo aunque coincidan el nombre base', async () => {
    const dir = await mkdtemp(path.join(tmpdir(), 'mm-subs-'));
    await createFiles(dir, ['episodio.mkv', 'episodio.srt', 'episodio.mkv.bak']);

    const result = findSubtitleFiles(path.join(dir, 'episodio.mkv'));
    expect(result).toEqual([path.join(dir, 'episodio.srt')]);
  });

  it('no falla si el directorio del vídeo no existe', () => {
    const result = findSubtitleFiles('/tmp/no-existe-esta-ruta/episodio.mkv');
    expect(result).toEqual([]);
  });
});

describe('findSubtitleFile', () => {
  it('devuelve null cuando no hay subtítulos', async () => {
    const dir = await mkdtemp(path.join(tmpdir(), 'mm-subs-'));
    await createFiles(dir, ['episodio.mkv']);

    const result = findSubtitleFile(path.join(dir, 'episodio.mkv'));
    expect(result).toBeNull();
  });

  it('devuelve el primer subtítulo alfabéticamente', async () => {
    const dir = await mkdtemp(path.join(tmpdir(), 'mm-subs-'));
    await createFiles(dir, ['episodio.mkv', 'episodio.z.srt', 'episodio.a.srt']);

    const result = findSubtitleFile(path.join(dir, 'episodio.mkv'));
    expect(result).toBe(path.join(dir, 'episodio.a.srt'));
  });

  it('lee el contenido del directorio para confirmar', async () => {
    const dir = await mkdtemp(path.join(tmpdir(), 'mm-subs-'));
    await createFiles(dir, ['episodio.mkv', 'episodio.srt']);

    const result = findSubtitleFile(path.join(dir, 'episodio.mkv'));
    expect(result).toBe(path.join(dir, 'episodio.srt'));

    // Verificar que el archivo existe realmente
    const entries = await readdir(dir);
    expect(entries).toContain('episodio.srt');
  });
});
