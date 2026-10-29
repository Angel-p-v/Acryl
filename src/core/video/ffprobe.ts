import { execFile } from 'node:child_process';

/** Pista de audio real extraída de los metadatos del fichero de vídeo. */
export type RealAudioTrack = {
  index: number;
  language: string;
  codec: string;
  channels: number | null;
  sampleRate: string | null;
  title: string | null;
};

/** Pista de subtítulo real extraída de los metadatos del fichero de vídeo. */
export type RealSubtitleTrack = {
  index: number;
  language: string;
  codec: string;
  title: string | null;
};

/** Resultado del parseo de metadatos de un fichero de vídeo. */
export type VideoMediaInfo = {
  durationSeconds: number | null;
  videoCodec: string | null;
  videoWidth: number | null;
  videoHeight: number | null;
  audioTracks: readonly RealAudioTrack[];
  subtitleTracks: readonly RealSubtitleTrack[];
};

/** Opciones para ejecutar ffprobe. */
export type FfprobeOptions = {
  /** Ruta al ejecutable ffprobe. Por defecto 'ffprobe'. */
  ffprobePath?: string;
  /** Timeout en milisegundos. Por defecto 10 000 ms. */
  timeoutMs?: number;
};

/**
 * Ejecuta ffprobe sobre un fichero de vídeo y extrae las pistas de audio y
 * subtítulos con sus idiomas, códecs y demás metadatos reales, además del
 * códec y resolución del flujo de vídeo.
 *
 * Requiere que ffprobe esté instalado en el sistema (paquete ffmpeg).
 * Si ffprobe no está disponible o el fichero no tiene pistas reconocibles,
 * se devuelve un resultado con arrays vacíos (no lanza error para no bloquear
 * la TUI).
 */
export async function readVideoMediaInfo(
  filePath: string,
  options: FfprobeOptions = {},
): Promise<VideoMediaInfo> {
  const { ffprobePath = 'ffprobe', timeoutMs = 10_000 } = options;

  return new Promise((resolve) => {
    execFile(
      ffprobePath,
      ['-v', 'quiet', '-print_format', 'json', '-show_streams', '-show_format', filePath],
      { timeout: timeoutMs, encoding: 'utf-8' },
      (error, stdout, _stderr) => {
        if (error !== null) {
          // ffprobe no está instalado o el fichero no es legible: no bloquear la TUI.
          resolve({
            durationSeconds: null,
            videoCodec: null,
            videoWidth: null,
            videoHeight: null,
            audioTracks: [],
            subtitleTracks: [],
          });
          return;
        }

        let parsed: { streams?: unknown[]; format?: { duration?: string } };
        try {
          parsed = JSON.parse(stdout) as { streams?: unknown[]; format?: { duration?: string } };
        } catch {
          resolve({
            durationSeconds: null,
            videoCodec: null,
            videoWidth: null,
            videoHeight: null,
            audioTracks: [],
            subtitleTracks: [],
          });
          return;
        }

        const streams = parsed.streams ?? [];
        const formatDuration = parsed.format?.duration;

        const durationSeconds =
          formatDuration !== undefined ? Number.parseFloat(formatDuration) : null;

        const audioTracks: RealAudioTrack[] = [];
        const subtitleTracks: RealSubtitleTrack[] = [];
        let videoCodec: string | null = null;
        let videoWidth: number | null = null;
        let videoHeight: number | null = null;

        let audioIndex = 0;
        let subtitleIndex = 0;

        for (const stream of streams) {
          if (stream === null || typeof stream !== 'object') continue;

          const codecType = (stream as { codec_type?: string }).codec_type;

          if (codecType === 'video' && videoCodec === null) {
            const s = stream as {
              codec_name?: string;
              width?: number;
              height?: number;
            };
            videoCodec = s.codec_name ?? null;
            videoWidth = s.width ?? null;
            videoHeight = s.height ?? null;
          } else if (codecType === 'audio') {
            const s = stream as {
              index?: number;
              language?: string;
              codec_name?: string;
              channels?: number;
              sample_rate?: string;
              tags?: { language?: string; title?: string };
            };
            const tags = s.tags ?? {};
            const language = tags.language ?? s.language ?? `audio ${audioIndex + 1}`;
            audioTracks.push({
              index: audioIndex,
              language,
              codec: s.codec_name ?? 'desconocido',
              channels: s.channels ?? null,
              sampleRate: s.sample_rate ?? null,
              title: tags.title ?? null,
            });
            audioIndex++;
          } else if (codecType === 'subtitle') {
            const s = stream as {
              index?: number;
              language?: string;
              codec_name?: string;
              tags?: { language?: string; title?: string };
            };
            const tags = s.tags ?? {};
            const language = tags.language ?? s.language ?? `subtítulo ${subtitleIndex + 1}`;
            subtitleTracks.push({
              index: subtitleIndex,
              language,
              codec: s.codec_name ?? 'desconocido',
              title: tags.title ?? null,
            });
            subtitleIndex++;
          }
        }

        resolve({
          durationSeconds,
          videoCodec,
          videoWidth,
          videoHeight,
          audioTracks,
          subtitleTracks,
        });
      },
    );
  });
}
