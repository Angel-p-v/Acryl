import { Box, Text } from 'ink';
import React, { useEffect, useState } from 'react';

import { shortenPath, theme } from '../theme.js';
import { PosterImage } from './PosterImage.js';

import type { MediaMetadataRow } from '../../core/db/repositories/metadataRepo.js';
import type { LibraryService, MediaDetails as Details } from '../../core/library/service.js';

export type MediaDetailsProps = {
  details: Details | null;
  library: LibraryService;
  /** Clave de refresco de metadatos: al cambiarla, se vuelven a leer los metadatos de la BD. */
  metadataRefreshKey: number;
};

const MAX_FILES = 8;

/** Panel con la información del título seleccionado y sus ficheros. */
export function MediaDetails({
  details,
  library,
  metadataRefreshKey,
}: MediaDetailsProps): React.JSX.Element {
  // Los Hooks van ANTES de cualquier return condicional para respetar las
  // Reglas de Hooks: el orden y la cantidad de hooks deben ser idénticos en
  // cada render, incluso cuando `details` sea null.
  const mediaId: number | null = details === null ? null : details.media.id;
  const [metadata, setMetadata] = useState<MediaMetadataRow | null>(null);

  // Cargar metadatos existentes al montar o al refrescar metadatos externos.
  useEffect(() => {
    if (mediaId === null) return;
    const saved = library.getMetadata(mediaId);
    setMetadata(saved ?? null);
  }, [library, mediaId, metadataRefreshKey]);

  if (details === null) {
    return (
      <Box paddingX={1} marginTop={1}>
        <Text>{theme.muted('Selecciona un título para ver sus detalles.')}</Text>
      </Box>
    );
  }

  const { media, files } = details;
  const year = media.year === null ? '' : ` (${media.year})`;

  return (
    <Box
      flexDirection="column"
      borderStyle="round"
      borderColor={theme.border}
      paddingX={1}
      marginTop={1}
    >
      <Box flexDirection="row" flexGrow={1} justifyContent="space-between">
        {/* ==================== Derecha: información del título ==================== */}
        <Box flexDirection="column" flexGrow={1} flexShrink={1}>
          <Text>
            {theme.highlight(`${media.title}${year}`)} {theme.muted(`#${media.id}`)}
          </Text>
          <Text>{theme.muted(`  ${shortenPath(media.folderPath, 72)}`)}</Text>
          <Text>
            {theme.label('Ficheros')} {theme.muted(`(${files.length})`)}
          </Text>
          {files.length === 0 ? (
            <Text>{theme.muted('  sin ficheros asociados')}</Text>
          ) : (
            files.slice(0, MAX_FILES).map((file) => {
              const season = file.season === null ? '' : `T${String(file.season).padStart(2, '0')}`;
              const episode =
                file.episode === null ? '' : `E${String(file.episode).padStart(2, '0')}`;
              const label = `${season}${episode}` || '—';
              return (
                <Text key={file.id}>
                  {'  '}
                  {theme.muted(`#${file.id}`)} {theme.primary(label.padEnd(6))}{' '}
                  {shortenPath(file.filePath, 35)}
                </Text>
              );
            })
          )}
          {files.length > MAX_FILES ? (
            <Text>{theme.muted(`  … y ${files.length - MAX_FILES} más`)}</Text>
          ) : null}
          <Text> </Text>

          {/* Metadatos externos (sinopsis, calificación, etc.) */}
          {metadata ? (
            <>
              <Text>
                {theme.label('Metadatos')} {theme.muted(`(${metadata.provider})`)}
              </Text>
              {metadata.title && <Text>{`  Título: ${metadata.title}`}</Text>}
              {metadata.overview && (
                <Box maxWidth={120} paddingX={1} marginBottom={1}>
                  <Text wrap="wrap">{`  Sinopsis: ${metadata.overview}`}</Text>
                </Box>
              )}
              {metadata.voteAverage !== null && (
                <Text>{`  Calificación: ${metadata.voteAverage}/10`}</Text>
              )}
              {metadata.releaseDate && <Text>{`  Estreno: ${metadata.releaseDate}`}</Text>}
              {metadata.originalLanguage && <Text>{`  Idioma: ${metadata.originalLanguage}`}</Text>}
            </>
          ) : (
            <>
              <Text>
                {theme.label('Metadatos')} {theme.muted('(sin datos)')}
              </Text>
              <Text>{theme.muted('  Pulsa «r» para obtener metadatos externos.')}</Text>
            </>
          )}
        </Box>

        {/* ==================== Izquierda: póster (abajo a la derecha) ==================== */}
        <Box
          flexDirection="column"
          flexGrow={1}
          flexShrink={0}
          marginLeft={2}
          justifyContent="flex-end"
        >
          <PosterImage imagePath={metadata?.localPosterPath ?? null} />
        </Box>
      </Box>
    </Box>
  );
}
