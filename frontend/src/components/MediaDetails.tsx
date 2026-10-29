import { useCallback, useEffect, useState } from 'react';

import { getMediaDetails, toggleFavorite, toggleWatched, playerOpen } from '../services/tauri';
import type { MediaDetails } from '../types/media';
import type { JSX } from 'react';

/**
 * Página de detalles de un medio.
 *
 * Muestra el póster (o placeholder), metadatos (título, tipo, año,
 * sinopsis), estado de favorito/visto y la lista de archivos indexados.
 * Permite alternar favorito/visto y abrir un archivo para reproducir.
 */
export function MediaDetails({
  mediaId,
  onBack,
}: {
  readonly mediaId: number;
  readonly onBack: () => void;
}): JSX.Element {
  const [details, setDetails] = useState<MediaDetails | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadDetails = useCallback(async (): Promise<void> => {
    try {
      const d = await getMediaDetails('media-manager.db', mediaId);
      setDetails(d);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'no se pudieron cargar los detalles');
    } finally {
      setLoading(false);
    }
  }, [mediaId]);

  useEffect(() => {
    // Carga de datos al montar (o al cambiar mediaId). El setState ocurre tras
    // el await, no de forma síncrona en el cuerpo del efecto, por lo que este
    // patrón es el de data-fetching recomendado por React.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    void loadDetails();
  }, [loadDetails]);

  const handleToggleFavorite = useCallback(async (): Promise<void> => {
    if (!details) return;
    try {
      const next = await toggleFavorite('media-manager.db', mediaId);
      setDetails((d) => (d ? { ...d, media: { ...d.media, is_favorite: next } } : d));
    } catch (e) {
      setError(e instanceof Error ? e.message : 'no se pudo actualizar favorito');
    }
  }, [details, mediaId]);

  const handleToggleWatched = useCallback(async (): Promise<void> => {
    if (!details) return;
    try {
      const next = await toggleWatched('media-manager.db', mediaId);
      setDetails((d) => (d ? { ...d, media: { ...d.media, is_watched: next } } : d));
    } catch (e) {
      setError(e instanceof Error ? e.message : 'no se pudo actualizar visto');
    }
  }, [details, mediaId]);

  const handlePlayFile = useCallback(async (filePath: string): Promise<void> => {
    try {
      await playerOpen(filePath);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'no se pudo abrir el archivo');
    }
  }, []);

  if (loading) {
    return (
      <div className="details-page">
        <p className="loading">Cargando detalles…</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="details-page">
        <p className="error">{error}</p>
        <button className="back-btn" onClick={onBack}>
          ← Volver
        </button>
      </div>
    );
  }

  if (!details) {
    return (
      <div className="details-page">
        <p className="empty">No se encontró el medio.</p>
        <button className="back-btn" onClick={onBack}>
          ← Volver
        </button>
      </div>
    );
  }

  const { media, files } = details;
  const initial = media.title.charAt(0).toUpperCase();

  return (
    <div className="details-page">
      <header className="details-header">
        <button className="back-btn" onClick={onBack}>
          ← Volver
        </button>
        <h1 className="details-title">{media.title}</h1>
        <div className="details-meta">
          <span className="details-type">{media.type}</span>
          {media.year && <span className="details-separator">·</span>}
          {media.year && <span className="details-year">{media.year}</span>}
        </div>
        <div className="details-status">
          <button
            className={`status-btn favorite ${media.is_favorite ? 'active' : ''}`}
            onClick={handleToggleFavorite}
            title={media.is_favorite ? 'Quitar favorito' : 'Marcar favorito'}
          >
            {media.is_favorite ? '★' : '☆'}
          </button>
          <button
            className={`status-btn watched ${media.is_watched ? 'active' : ''}`}
            onClick={handleToggleWatched}
            title={media.is_watched ? 'Marcar como no visto' : 'Marcar visto'}
          >
            {media.is_watched ? '✓' : '○'}
          </button>
        </div>
      </header>

      <div className="details-content">
        <div className="details-poster">
          <div className="poster-placeholder">
            <span className="poster-initial">{initial}</span>
            <span className="poster-type">{media.type}</span>
          </div>
          {media.synopsis && <p className="details-synopsis">{media.synopsis}</p>}
        </div>

        <div className="details-files">
          <h2 className="details-files-title">Archivos ({files.length})</h2>
          <ul className="details-files-list">
            {files.map((file) => (
              <li key={file.id} className="details-file-item">
                <span className="details-file-episode">
                  {file.season !== null && file.episode !== null
                    ? `S${String(file.season).padStart(2, '0')}E${String(file.episode).padStart(2, '0')}`
                    : '—'}
                </span>
                <span className="details-file-title">{file.title ?? 'Sin título'}</span>
                <button
                  className="details-file-play"
                  onClick={() => {
                    void handlePlayFile(file.file_path);
                  }}
                  title="Reproducir"
                >
                  ▶
                </button>
              </li>
            ))}
          </ul>
        </div>
      </div>
    </div>
  );
}
