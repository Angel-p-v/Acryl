import { Box, Text, useApp, useInput } from 'ink';
import React, { useCallback, useEffect, useState } from 'react';

import { readVideoMediaInfo } from '../../core/video/ffprobe.js';
import { PosterImage } from '../components/PosterImage.js';
import { theme } from '../theme.js';

import type { MediaMetadataRow } from '../../core/db/repositories/metadataRepo.js';
import type { MediaFileRow } from '../../core/db/types.js';
import type { MediaDetails as Details, LibraryService } from '../../core/library/service.js';

/** Número máximo de episodios visibles por página en la lista. */
const EPISODES_PER_PAGE = 12;

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

/** Props de la vista de detalles de reproducción. */
export type PlayDetailsProps = {
  /** Título seleccionado en la biblioteca. */
  details: Details | null;
  /** Servicio de biblioteca (para obtener metadatos y pósters cacheados). */
  library: LibraryService;
  /** Reproduce el fichero seleccionado con las pistas de audio/subtítulo elegidas. */
  onPlay: (input: {
    fileId: number;
    audioTrackIndex: number | null;
    subtitleTrackIndex: number | null;
  }) => void;
  /** Vuelve a la vista de biblioteca. */
  onBack: () => void;
};

/**
 * Vista de detalles de reproducción tipo Jellyfin.
 *
 * Diseño (3 columnas flexibles + barra inferior):
 * - Izquierda: póster grande.
 * - Centro: título, año, códec, sinopsis y listado de episodios con paginación.
 * - Derecha: selección de pista de audio (←→) y subtítulos (Mayús+←→).
 * - Abajo (ancho completo): barra de teclas de acción en filas separadas.
 *
 * Los idiomas de audio y subtítulos se extraen de los metadatos reales de cada
 * fichero de vídeo mediante ffprobe, no se infieren del nombre de archivo.
 *
 * Navegación:
 * - ↑ / ↓ : navega por los episodios/ficheros (con paginación de 12)
 * - ← / → : navega por las pistas de audio detectadas
 * - Mayús+← / Mayús+→ : navega por las pistas de subtítulos detectadas
 * - p : reproduce el fichero seleccionado con las pistas elegidas
 * - q / Esc : vuelve a la biblioteca
 */
export function PlayDetails({
  details,
  library,
  onPlay,
  onBack,
}: PlayDetailsProps): React.JSX.Element {
  const { exit } = useApp();

  // Datos del título (pueden ser undefined cuando details es null).
  const media = details?.media ?? null;
  const files: readonly MediaFileRow[] = details?.files ?? [];

  // -------------------------------------------------------------------------
  // REGLAS DE HOOKS: todos los hooks van ANTES de cualquier return
  // condicional, para que el orden y la cantidad sean idénticos en cada
  // render, incluso cuando `details` sea null.
  // -------------------------------------------------------------------------

  const [selectedFileIndex, setSelectedFileIndex] = useState(0);
  const [visibleStart, setVisibleStart] = useState(0);
  const [selectedAudioIndex, setSelectedAudioIndex] = useState<number | null>(null);
  const [selectedSubtitleIndex, setSelectedSubtitleIndex] = useState<number | null>(null);
  const [audioTracks, setAudioTracks] = useState<readonly RealAudioTrack[]>([]);
  const [subtitleTracks, setSubtitleTracks] = useState<readonly RealSubtitleTrack[]>([]);
  const [videoCodec, setVideoCodec] = useState<string | null>(null);
  const [videoWidth, setVideoWidth] = useState<number | null>(null);
  const [videoHeight, setVideoHeight] = useState<number | null>(null);
  const [loadingMetadata, setLoadingMetadata] = useState(false);
  const [metadataError, setMetadataError] = useState<string | null>(null);
  /** Indica si se está obteniendo metadatos externos de la API pública. */
  const [fetchingMetadata, setFetchingMetadata] = useState(false);
  /** Error al obtener metadatos externos de la API pública (se muestra en el área de sinopsis). */
  const [externalMetadataError, setExternalMetadataError] = useState<string | null>(null);
  /** Ruta local al póster cacheado del título (si existe). */
  const [posterPath, setPosterPath] = useState<string | null>(null);
  /** Metadatos externos (TMDB/TVDB/AniList) del título. */
  const [metadata, setMetadata] = useState<MediaMetadataRow | null>(null);

  if (details === null || media === null) {
    return (
      <Box flexGrow={1} flexDirection="column" paddingX={1}>
        <Text>{theme.error('No se cargaron los detalles del título.')}</Text>
        <Text>{theme.muted('Vuelve a la biblioteca y selecciona un título.')}</Text>
      </Box>
    );
  }

  const year = media.year === null ? '' : ` (${media.year})`;
  const typeLabel =
    media.type === 'series' ? 'serie' : media.type === 'anime' ? 'anime' : 'película';

  const selectedFile = files[selectedFileIndex];
  const selectedAudio = audioTracks[selectedAudioIndex ?? -1];
  const selectedSubtitle = subtitleTracks[selectedSubtitleIndex ?? -1];

  const handlePlaySelected = useCallback((): void => {
    if (selectedFile === undefined) return;
    onPlay({
      fileId: selectedFile.id,
      audioTrackIndex: selectedAudioIndex,
      subtitleTrackIndex: selectedSubtitleIndex,
    });
  }, [onPlay, selectedFile, selectedAudioIndex, selectedSubtitleIndex]);

  const handleBack = useCallback((): void => {
    onBack();
  }, [onBack]);

  /** Obtiene los metadatos externos (sinopsis, carátula, etc.) de la API pública. */
  const handleFetchMetadata = useCallback(async (): Promise<void> => {
    if (media === null) return;

    setFetchingMetadata(true);
    setExternalMetadataError(null);
    try {
      // Forzar refresco: el usuario pulsó "r" explícitamente, ignorar TTL.
      await library.getOrFetchMetadata(media.id, { forceRefresh: true });

      // Refrescar estado local desde el repositorio tras la obtención.
      const saved = library.getMetadata(media.id);
      setMetadata(saved ?? null);
      setPosterPath(saved?.localPosterPath ?? null);
    } catch (error) {
      setExternalMetadataError(
        error instanceof Error ? error.message : 'No se pudieron obtener los metadatos.',
      );
    } finally {
      setFetchingMetadata(false);
    }
  }, [media, library]);

  /** Carga los metadatos reales (audio/subtítulos/vídeo) del fichero seleccionado con ffprobe. */
  const loadFileMetadata = useCallback(async (file: MediaFileRow | undefined): Promise<void> => {
    if (file === undefined) {
      setAudioTracks([]);
      setSubtitleTracks([]);
      setVideoCodec(null);
      setVideoWidth(null);
      setVideoHeight(null);
      setSelectedAudioIndex(null);
      setSelectedSubtitleIndex(null);
      return;
    }

    setLoadingMetadata(true);
    setMetadataError(null);
    try {
      const info = await readVideoMediaInfo(file.filePath);
      setAudioTracks(info.audioTracks);
      setSubtitleTracks(info.subtitleTracks);
      setVideoCodec(info.videoCodec);
      setVideoWidth(info.videoWidth);
      setVideoHeight(info.videoHeight);
      setSelectedAudioIndex(info.audioTracks.length > 0 ? 0 : null);
      setSelectedSubtitleIndex(null);
    } catch (error) {
      setMetadataError(
        error instanceof Error ? error.message : 'No se pudieron leer los metadatos.',
      );
      setAudioTracks([]);
      setSubtitleTracks([]);
      setVideoCodec(null);
      setVideoWidth(null);
      setVideoHeight(null);
      setSelectedAudioIndex(null);
      setSelectedSubtitleIndex(null);
    } finally {
      setLoadingMetadata(false);
    }
  }, []);

  // Cargar metadatos del fichero seleccionado cuando cambia.
  useEffect(() => {
    void loadFileMetadata(selectedFile);
  }, [selectedFile, loadFileMetadata]);

  // Obtener el póster cacheado del título cuando cambian los detalles.
  useEffect(() => {
    if (media === null) {
      setPosterPath(null);
      return;
    }
    const saved = library.getMetadata(media.id);
    setPosterPath(saved?.localPosterPath ?? null);
  }, [media, library]);

  // Obtener la sinopsis de los metadatos externos (TMDB/TVDB/AniList).
  useEffect(() => {
    if (media === null) return;
    const saved = library.getMetadata(media.id);
    setMetadata(saved ?? null);
  }, [media, library]);

  /** Ajusta la ventana visible para que el índice seleccionado siempre esté visible. */
  const ensureVisible = useCallback(
    (nextIndex: number): void => {
      setSelectedFileIndex(nextIndex);
      const pageEnd = visibleStart + EPISODES_PER_PAGE;
      if (nextIndex < visibleStart) {
        setVisibleStart(Math.max(0, nextIndex));
      } else if (nextIndex >= pageEnd) {
        setVisibleStart(nextIndex - EPISODES_PER_PAGE + 1);
      }
    },
    [visibleStart],
  );

  // Navegación por teclado dentro de esta vista.
  // Se usa useInput en lugar de useKeymap para poder distinguir
  // Mayús+flecha (subtítulos) de flecha simple (audio).
  useInput(
    (
      input: string,
      key: {
        escape?: boolean;
        ctrl?: boolean;
        shift?: boolean;
        leftArrow?: boolean;
        rightArrow?: boolean;
        upArrow?: boolean;
        downArrow?: boolean;
      },
    ): void => {
      if (key.ctrl && input === 'c') {
        exit();
        return;
      }

      if (key.escape || input === 'q') {
        handleBack();
        return;
      }

      if (input === 'p') {
        handlePlaySelected();
        return;
      }

      if (input === 'r') {
        void handleFetchMetadata();
        return;
      }

      // Navegación de episodios con flechas arriba/abajo.
      if (key.upArrow || input === '\u001b[A') {
        ensureVisible(Math.max(0, selectedFileIndex - 1));
        return;
      }
      if (key.downArrow || input === '\u001b[B') {
        ensureVisible(Math.min(files.length - 1, selectedFileIndex + 1));
        return;
      }

      // Navegación de pistas de audio con flechas izquierda/derecha (sin shift).
      if (!key.shift && (key.leftArrow || input === '\u001b[D')) {
        setSelectedAudioIndex((prev) =>
          prev === null || prev <= 0 ? null : Math.max(0, prev - 1),
        );
        return;
      }
      if (!key.shift && (key.rightArrow || input === '\u001b[C')) {
        setSelectedAudioIndex((prev) =>
          prev === null
            ? audioTracks.length > 0
              ? 0
              : null
            : Math.min(audioTracks.length - 1, prev + 1),
        );
        return;
      }

      // Navegación de pistas de subtítulos con Mayús+flechas izquierda/derecha.
      if (key.shift && (key.leftArrow || input === '\u001b[D')) {
        setSelectedSubtitleIndex((prev) =>
          prev === null || prev <= 0 ? null : Math.max(0, prev - 1),
        );
        return;
      }
      if (key.shift && (key.rightArrow || input === '\u001b[C')) {
        setSelectedSubtitleIndex((prev) =>
          prev === null
            ? subtitleTracks.length > 0
              ? 0
              : null
            : Math.min(subtitleTracks.length - 1, prev + 1),
        );
        return;
      }
    },
  );

  // Nombre corto del fichero (sin ruta absoluta).
  const fileName = selectedFile ? (selectedFile.filePath.split('/').pop() ?? '—') : '—';

  // Ventana visible de episodios (paginación de 12).
  const visibleFiles = files.slice(visibleStart, visibleStart + EPISODES_PER_PAGE);

  return (
    <Box flexGrow={1} flexDirection="column">
      {/* ==================== Cuerpo principal (3 columnas flexibles) ==================== */}
      <Box flexGrow={1} flexDirection="row">
        {/* Izquierda: póster grande */}
        <Box flexDirection="column" paddingX={0} paddingY={1} flexShrink={0}>
          <PosterImage imagePath={posterPath} />
        </Box>

        {/* Centro: metadatos y episodios */}
        <Box flexDirection="column" flexGrow={1} flexShrink={1} paddingX={1} paddingY={1}>
          <Text>
            {theme.value(`${media.title}${year}`)} {theme.muted(`#${media.id} · ${typeLabel}`)}
          </Text>
          <Text> </Text>

          {media.year !== null && (
            <>
              <Text>
                {theme.label('Año')} {theme.value(String(media.year))}
              </Text>
              <Text> </Text>
            </>
          )}

          {videoCodec !== null && (
            <>
              <Text>
                {theme.label('Códec vídeo')}{' '}
                {theme.value(
                  videoCodec +
                    (videoWidth !== null && videoHeight !== null
                      ? ` ${videoWidth}x${videoHeight}`
                      : ''),
                )}
              </Text>
              <Text> </Text>
            </>
          )}

          {/* Sinopsis real de los metadatos externos (TMDB/Jikan) */}
          <Text>
            {theme.label('Sinopsis')}{' '}
            {fetchingMetadata
              ? theme.muted('  [Buscando metadatos…]')
              : externalMetadataError !== null
                ? theme.error(`  [Error al obtener metadatos: ${externalMetadataError}]`)
                : metadata?.overview
                  ? theme.value(metadata.overview)
                  : theme.muted(
                      files.length > 0
                        ? `${files.length} fichero(s). Pulsa «r» para obtener metadatos.`
                        : 'Sin sinopsis. Pulsa «r».',
                    )}
          </Text>
          <Text> </Text>

          <Text>
            {theme.label('Episodios / ficheros')} {theme.muted(`(↑↓ navegan)`)}
          </Text>
          {files.length === 0 ? (
            <Text>{theme.muted('  sin ficheros indexados')}</Text>
          ) : (
            visibleFiles.map((file, relativeIndex) => {
              const absoluteIndex = visibleStart + relativeIndex;
              const active = absoluteIndex === selectedFileIndex;
              const season = file.season === null ? '' : `T${String(file.season).padStart(2, '0')}`;
              const episode =
                file.episode === null ? '' : `E${String(file.episode).padStart(2, '0')}`;
              const label = `${season}${episode}` || '—';
              const fileName = file.filePath.split('/').pop() ?? '';
              return (
                <Text key={file.id} bold={active} {...(active ? { color: 'cyan' as const } : {})}>
                  {`${active ? '▸ ' : '   '}${label.padEnd(8)} ${fileName}`}
                </Text>
              );
            })
          )}
          <Text> </Text>
        </Box>

        {/* Derecha: selección de pistas */}
        <Box flexDirection="column" flexShrink={0} paddingX={1} paddingY={1}>
          {loadingMetadata ? (
            <>
              <Text>
                {theme.label('Pista de audio')} {theme.muted('(←→)')}
              </Text>
              <Text>{theme.muted('  cargando metadatos…')}</Text>
              <Text> </Text>
              <Text>
                {theme.label('Pista de subtítulos')} {theme.muted('(Mayús+←→)')}
              </Text>
              <Text>{theme.muted('  cargando metadatos…')}</Text>
              <Text> </Text>
            </>
          ) : metadataError !== null ? (
            <>
              <Text>
                {theme.label('Pista de audio')} {theme.muted('(←→)')}
              </Text>
              <Text>{theme.error(`  error: ${metadataError}`)}</Text>
              <Text> </Text>
              <Text>
                {theme.label('Pista de subtítulos')} {theme.muted('(Mayús+←→)')}
              </Text>
              <Text>{theme.error(`  error: ${metadataError}`)}</Text>
              <Text> </Text>
            </>
          ) : (
            <>
              <Text>
                {theme.label('Pista de audio')} {theme.muted('(←→)')}
              </Text>
              {audioTracks.length === 0 ? (
                <Text>{theme.muted('  sin pistas de audio detectadas')}</Text>
              ) : (
                audioTracks.map((track) => {
                  const active = track.index === selectedAudioIndex;
                  return (
                    <Text
                      key={`audio-${track.index}`}
                      bold={active}
                      {...(active ? { color: 'cyan' as const } : {})}
                    >
                      {`${active ? '▸ ' : '   '}${track.language} (${track.codec}${
                        track.channels !== null ? ` ${track.channels}ch` : ''
                      }${track.sampleRate ? ` ${track.sampleRate}Hz` : ''}${
                        track.title ? ` - ${track.title}` : ''
                      }`}
                    </Text>
                  );
                })
              )}
              <Text> </Text>

              <Text>
                {theme.label('Pista de subtítulos')} {theme.muted('(Mayús+←→)')}
              </Text>
              {subtitleTracks.length === 0 ? (
                <Text>{theme.muted('  sin pistas de subtítulos detectadas')}</Text>
              ) : (
                subtitleTracks.map((track) => {
                  const active = track.index === selectedSubtitleIndex;
                  return (
                    <Text
                      key={`subtitle-${track.index}`}
                      bold={active}
                      {...(active ? { color: 'cyan' as const } : {})}
                    >
                      {`${active ? '▸ ' : '   '}${track.language} (${track.codec}${
                        track.title ? ` - ${track.title}` : ''
                      }`}
                    </Text>
                  );
                })
              )}
              <Text> </Text>
            </>
          )}
        </Box>
      </Box>

      {/* ==================== Barra inferior: teclas de acción (ancho completo, filas separadas) ==================== */}
      <Box paddingX={1} paddingY={1}>
        <Text>
          {theme.muted(
            '  [p] Reproducir   [q/Esc] Volver   ↑↓ Episodios   ←→ Audio   Mayús+←→ Subtítulos   [r] Metadatos',
          )}
        </Text>
        <Text>
          {theme.muted(
            `  Fichero: ${fileName}   Audio: ${selectedAudio?.language ?? '—'}   Subtítulos: ${selectedSubtitle?.language ?? '—'}`,
          )}
        </Text>
      </Box>
    </Box>
  );
}
