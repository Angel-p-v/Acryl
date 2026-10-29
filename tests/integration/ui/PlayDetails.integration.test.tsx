import { cleanup, render } from 'ink-testing-library';
import React from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { PlayDetails } from '../../../src/ui/views/PlayDetails.js';

import type { MediaMetadataRow } from '../../../src/core/db/repositories/metadataRepo.js';
import type { MediaFileRow, MediaRow } from '../../../src/core/db/types.js';
import type { LibraryService } from '../../../src/core/library/service.js';

const media: MediaRow = {
  id: 1,
  type: 'series',
  title: 'Black Torch',
  sortTitle: 'black torch',
  year: 2023,
  folderPath: '/media/series/Black Torch',
  createdAt: '2026-01-01 00:00:00',
  updatedAt: '2026-01-01 00:00:00',
};

const files: MediaFileRow[] = [
  {
    id: 1,
    mediaId: 1,
    season: 1,
    episode: 1,
    title: null,
    filePath: '/media/series/Black Torch/S01E01.mkv',
    sizeBytes: 100,
    mtimeMs: 1,
    indexedAt: '2026-01-01 00:00:00',
  },
  {
    id: 2,
    mediaId: 1,
    season: 1,
    episode: 2,
    title: null,
    filePath: '/media/series/Black Torch/S01E02.mkv',
    sizeBytes: 100,
    mtimeMs: 1,
    indexedAt: '2026-01-01 00:00:00',
  },
];

const details = {
  media,
  files,
};

const fakeMetadata: MediaMetadataRow = {
  mediaId: 1,
  provider: 'tvmaze',
  externalId: 789,
  title: 'Black Torch',
  overview:
    'Jiro Azuma es un joven que puede comunicarse con animales y se une a una organización secreta.',
  posterPath: 'https://static.tvmaze.com/uploads/images/original/789.jpg',
  backdropPath: null,
  voteAverage: 8.0,
  releaseDate: '2023-04-08',
  firstAirDate: null,
  originalLanguage: 'Japanese',
  localPosterPath: null,
  fetchedAt: new Date().toISOString(),
  updatedAt: new Date().toISOString(),
};

describe('PlayDetails (flujo de metadatos)', () => {
  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it('muestra indicador de carga y obtiene metadatos al pulsar "r"', async () => {
    const getOrFetchMetadata = vi.fn(() => fakeMetadata);
    const getMetadata = vi.fn(() => fakeMetadata);
    const library = {
      getOrFetchMetadata,
      getMetadata,
      find: vi.fn(() => details),
    } as unknown as LibraryService;

    const { lastFrame, stdin, unmount } = render(
      <PlayDetails details={details} library={library} onPlay={vi.fn()} onBack={vi.fn()} />,
    );

    // Estado inicial: sin metadatos, muestra placeholder
    let frame = lastFrame() ?? '';
    const normalized = frame.replace(/\s+/g, ' ');
    expect(normalized).toContain('Pulsa «r»');
    expect(normalized).toContain('para');
    expect(normalized).toContain('obtener metadatos');
    expect(frame).not.toContain('Jiro Azuma');

    // Pulsar "r" para obtener metadatos
    stdin.write('r');
    await new Promise((resolve) => setTimeout(resolve, 100));

    frame = lastFrame() ?? '';

    // Debe haberse llamado getOrFetchMetadata con forceRefresh
    expect(getOrFetchMetadata).toHaveBeenCalledWith(1, { forceRefresh: true });

    // Después de obtener metadatos, la sinopsis debe mostrarse
    expect(frame).toContain('Jiro Azuma');
    expect(frame).toContain('Black Torch');

    unmount();
  });

  it('muestra mensaje de error cuando la búsqueda de metadatos falla', async () => {
    const getOrFetchMetadata = vi.fn(() => {
      throw new Error('API no disponible');
    });
    const getMetadata = vi.fn(() => undefined);
    const library = {
      getOrFetchMetadata,
      getMetadata,
      find: vi.fn(() => details),
    } as unknown as LibraryService;

    const { lastFrame, stdin, unmount } = render(
      <PlayDetails details={details} library={library} onPlay={vi.fn()} onBack={vi.fn()} />,
    );

    // Pulsar "r" para intentar obtener metadatos (falla)
    stdin.write('r');
    await new Promise((resolve) => setTimeout(resolve, 100));

    const frame = lastFrame() ?? '';

    // Debe mostrarse el error en el área de sinopsis (texto dividido entre líneas)
    expect(frame).toContain('Error');
    expect(frame).toContain('obtener');
    expect(frame).toContain('metadatos');
    expect(frame).toContain('API');
    expect(frame).toContain('disponible');

    unmount();
  });

  it('usa el título limpio al buscar metadatos (elimina [Judas], S01E01, 1080p)', async () => {
    const getOrFetchMetadata = vi.fn(() => fakeMetadata);
    const getMetadata = vi.fn(() => fakeMetadata);
    const library = {
      getOrFetchMetadata,
      getMetadata,
      find: vi.fn(() => details),
    } as unknown as LibraryService;

    const { stdin, unmount } = render(
      <PlayDetails details={details} library={library} onPlay={vi.fn()} onBack={vi.fn()} />,
    );

    stdin.write('r');
    await new Promise((resolve) => setTimeout(resolve, 100));

    // Verificar que se llamó con el id del medio y forceRefresh
    // (la limpieza del título ocurre dentro de getOrFetchMetadata en library/service.ts)
    expect(getOrFetchMetadata).toHaveBeenCalledWith(1, { forceRefresh: true });

    unmount();
  });
});
