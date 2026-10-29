import { useCallback, useState, useEffect } from 'react';

import { MediaList } from './MediaList';
import { MediaDetails } from './MediaDetails';
import { listMedia, searchMedia } from '../services/tauri';

import type { Media } from '../types/media';
import type { JSX } from 'react';

type SearchBarProps = {
  readonly dbPath: string;
};

export function SearchBar({ dbPath }: SearchBarProps): JSX.Element {
  const [query, setQuery] = useState('');
  const [media, setMedia] = useState<readonly Media[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedMediaId, setSelectedMediaId] = useState<number | null>(null);

  const load = useCallback(
    async (q: string) => {
      try {
        const results = q.trim() ? await searchMedia(dbPath, q) : (await listMedia(dbPath)).media;
        setMedia(results);
        setSelectedMediaId(null);
      } catch (error) {
        console.error('Error cargando medios:', error);
        setMedia([]);
        setSelectedMediaId(null);
      } finally {
        setLoading(false);
      }
    },
    [dbPath]
  );

  // Carga inicial: lista completa sin filtro. El setState ocurre tras el await,
  // no de forma síncrona en el cuerpo del efecto (patrón de data-fetching de
  // React). Se ignora la regla aquí por ser el uso legítimo de useEffect.
  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    void load('');
  }, [dbPath, load]);

  const handleMediaSelect = (mediaId: number) => {
    setSelectedMediaId(mediaId);
  };

  const handleBack = () => {
    setSelectedMediaId(null);
  };

  return (
    <section className="search-bar">
      <label htmlFor="media-search">
        <span className="visually-hidden">Buscar medios</span>
        <input
          id="media-search"
          type="search"
          placeholder="Buscar por título…"
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setSelectedMediaId(null);
          }}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              void load(query);
            }
          }}
          disabled={loading}
        />
      </label>
      {loading && <p className="loading">Buscando…</p>}
      {selectedMediaId !== null ? (
        <MediaDetails mediaId={selectedMediaId} onBack={handleBack} />
      ) : (
        <MediaList media={media} onMediaSelect={handleMediaSelect} />
      )}
    </section>
  );
}
