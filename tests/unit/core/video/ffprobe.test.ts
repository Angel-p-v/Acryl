import { execFile } from 'node:child_process';

import { describe, expect, it, vi } from 'vitest';

import { readVideoMediaInfo } from '../../../../src/core/video/ffprobe.js';

vi.mock('node:child_process', () => ({
  execFile: vi.fn(),
}));

const mockedExecFile = vi.mocked(execFile);

describe('readVideoMediaInfo', () => {
  it('extrae pistas de audio y subtítulos de la salida JSON de ffprobe', async () => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockedExecFile as any).mockImplementation(
      (
        _cmd: string,
        _args: unknown[],
        _options: { timeout?: number },
        callback: (err: Error | null, stdout: string, stderr: string) => void,
      ) => {
        callback(
          null,
          JSON.stringify({
            streams: [
              { codec_type: 'video', codec_name: 'h264' },
              {
                codec_type: 'audio',
                index: 0,
                language: 'spa',
                codec_name: 'aac',
                channels: 2,
                sample_rate: '48000',
                tags: { language: 'spa', title: 'Español Castellano' },
              },
              {
                codec_type: 'audio',
                index: 1,
                language: 'eng',
                codec_name: 'mp3',
                channels: 2,
                sample_rate: '44100',
                tags: { language: 'eng', title: 'Inglés Stereo' },
              },
              {
                codec_type: 'subtitle',
                index: 2,
                language: 'spa',
                codec_name: 'subrip',
                tags: { language: 'spa', title: 'Subtítulos en español' },
              },
            ],
            format: { duration: '4200.5' },
          }),
          '',
        );
      },
    );

    const result = await readVideoMediaInfo('/video/serie/S01E01.mkv');

    expect(result.durationSeconds).toBe(4200.5);
    expect(result.audioTracks).toHaveLength(2);
    expect(result.audioTracks[0]).toEqual({
      index: 0,
      language: 'spa',
      codec: 'aac',
      channels: 2,
      sampleRate: '48000',
      title: 'Español Castellano',
    });
    expect(result.audioTracks[1]).toEqual({
      index: 1,
      language: 'eng',
      codec: 'mp3',
      channels: 2,
      sampleRate: '44100',
      title: 'Inglés Stereo',
    });
    expect(result.subtitleTracks).toHaveLength(1);
    expect(result.subtitleTracks[0]).toEqual({
      index: 0,
      language: 'spa',
      codec: 'subrip',
      title: 'Subtítulos en español',
    });
  });

  it('devuelve arrays vacíos cuando ffprobe falla', async () => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockedExecFile as any).mockImplementation(
      (
        _cmd: string,
        _args: unknown[],
        _options: { timeout?: number },
        callback: (err: Error | null, stdout: string, stderr: string) => void,
      ) => {
        callback(new Error('ffprobe no encontrado'), '', '');
      },
    );

    const result = await readVideoMediaInfo('/video/inexistente.mkv');

    expect(result.durationSeconds).toBeNull();
    expect(result.audioTracks).toEqual([]);
    expect(result.subtitleTracks).toEqual([]);
  });

  it('devuelve arrays vacíos cuando ffprobe devuelve JSON inválido', async () => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockedExecFile as any).mockImplementation(
      (
        _cmd: string,
        _args: unknown[],
        _options: { timeout?: number },
        callback: (err: Error | null, stdout: string, stderr: string) => void,
      ) => {
        callback(null, 'no es json', '');
      },
    );

    const result = await readVideoMediaInfo('/video/corrupto.mkv');

    expect(result.durationSeconds).toBeNull();
    expect(result.audioTracks).toEqual([]);
    expect(result.subtitleTracks).toEqual([]);
  });

  it('usa valores por defecto cuando no hay idioma en los tags', async () => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockedExecFile as any).mockImplementation(
      (
        _cmd: string,
        _args: unknown[],
        _options: { timeout?: number },
        callback: (err: Error | null, stdout: string, stderr: string) => void,
      ) => {
        callback(
          null,
          JSON.stringify({
            streams: [
              {
                codec_type: 'audio',
                index: 0,
                codec_name: 'aac',
                channels: 2,
                sample_rate: '48000',
                tags: {},
              },
            ],
            format: {},
          }),
          '',
        );
      },
    );

    const result = await readVideoMediaInfo('/video/sin-idioma.mkv');

    expect(result.audioTracks).toHaveLength(1);
    expect(result.audioTracks[0]!.language).toBe('audio 1');
    expect(result.audioTracks[0]!.codec).toBe('aac');
  });

  it('respeta el timeout configurado', async () => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockedExecFile as any).mockImplementation(
      (
        _cmd: string,
        _args: unknown[],
        options: { timeout?: number },
        callback: (err: Error | null, stdout: string, stderr: string) => void,
      ) => {
        callback(null, JSON.stringify({ streams: [], format: {} }), '');
      },
    );

    await readVideoMediaInfo('/video/test.mkv', { timeoutMs: 5000 });

    expect(mockedExecFile).toHaveBeenCalledWith(
      'ffprobe',
      ['-v', 'quiet', '-print_format', 'json', '-show_streams', '-show_format', '/video/test.mkv'],
      { timeout: 5000, encoding: 'utf-8' },
      expect.any(Function),
    );
  });
});
