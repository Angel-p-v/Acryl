import { useState, useEffect } from 'react';
import type { JSX } from 'react';
import type { Media } from '../types/media';
import { listMedia } from '../services/tauri';
import { MediaList } from './MediaList';

interface SearchBarProps {
  readonly dbPath: string;
}

export function SearchBar({ dbPath }: SearchBarProps): JSX.Element {
  const [query, setQuery] = useState('');
  const [media, setMedia] = useState<Media[]>([]);
  const [loading, setLoading] = useState(false);

  const search = async (q: string) => {
    setLoading(true);
    try {
      const results = await listMedia(dbPath, q || undefined);
      setMedia(results);
    } catch (error) {
      console.error('Error buscando medios:', error);
      setMedia([]);
    } finally {
      setLoading(false);
    }
  };

  // Carga inicial: lista completa sin filtro.
  useEffect(() => {
    void search('');
  }, [dbPath]);

  return (
    <section className="search-bar">
      <label htmlFor="media-search">
        <span className="visually-hidden">Buscar medios</span>
        <input
          id="media-search"
          type="search"
          placeholder="Buscar por título…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              void search(query);
            }
          }}
          disabled={loading}
        />
      </label>
      {loading && <p className="loading">Buscando…</p>}
      <MediaList media={media} />
    </section>
  );
}
