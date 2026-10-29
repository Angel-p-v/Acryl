import { useCallback, useEffect, useState } from 'react';

import {
  getMediaDetails,
  syncMetadata,
  toggleFavorite,
  toggleWatched,
  playerOpen,
} from '../services/tauri';
import { useLanguage } from '../contexts/LanguageContext';
import type { MediaDetails } from '../types/media';
import type { JSX } from 'react';

export function MediaDetails({
  mediaId,
  onBack,
}: {
  readonly mediaId: number;
  readonly onBack: () => void;
}): JSX.Element {
  const { t } = useLanguage();
  const [details, setDetails] = useState<MediaDetails | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [syncing, setSyncing] = useState(false);

  const loadDetails = useCallback(async (): Promise<void> => {
    try {
      const d = await getMediaDetails('acryl.db', mediaId);
      setDetails(d);
    } catch (e) {
      setError(e instanceof Error ? e.message : t('details.error'));
    } finally {
      setLoading(false);
    }
  }, [mediaId, t]);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    void loadDetails();
  }, [loadDetails]);

  const handleToggleFavorite = useCallback(async (): Promise<void> => {
    if (!details) return;
    try {
      const next = await toggleFavorite('acryl.db', mediaId);
      setDetails((d) => (d ? { ...d, media: { ...d.media, is_favorite: next } } : d));
    } catch (e) {
      setError(e instanceof Error ? e.message : t('details.favorite.error'));
    }
  }, [details, mediaId, t]);

  const handleToggleWatched = useCallback(async (): Promise<void> => {
    if (!details) return;
    try {
      const next = await toggleWatched('acryl.db', mediaId);
      setDetails((d) => (d ? { ...d, media: { ...d.media, is_watched: next } } : d));
    } catch (e) {
      setError(e instanceof Error ? e.message : t('details.watched.error'));
    }
  }, [details, mediaId, t]);

  const handlePlayFile = useCallback(
    async (filePath: string): Promise<void> => {
      try {
        await playerOpen(filePath);
      } catch (e) {
        setError(e instanceof Error ? e.message : t('details.openFile'));
      }
    },
    [t]
  );

  const handleSyncMetadata = useCallback(async (): Promise<void> => {
    setSyncing(true);
    setError(null);
    try {
      const result = await syncMetadata('acryl.db', mediaId);
      setDetails((d) =>
        d
          ? {
              ...d,
              media: {
                ...d.media,
                poster_path: result.poster_path ?? d.media.poster_path,
                synopsis: result.synopsis ?? d.media.synopsis,
                metadata_fetched_at: new Date().toISOString(),
              },
            }
          : d
      );
    } catch (e) {
      setError(e instanceof Error ? e.message : t('details.error'));
    } finally {
      setSyncing(false);
    }
  }, [mediaId, t]);

  if (loading) {
    return (
      <div className="details-page">
        <p className="loading">{t('details.loading')}</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="details-page">
        <p className="error">{error}</p>
        <button className="back-btn" onClick={onBack}>
          {t('details.back')}
        </button>
      </div>
    );
  }

  if (!details) {
    return (
      <div className="details-page">
        <p className="empty">{t('details.notFound')}</p>
        <button className="back-btn" onClick={onBack}>
          {t('details.back')}
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
          {t('details.back')}
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
            title={media.is_favorite ? t('details.favorite.remove') : t('details.favorite.add')}
          >
            {media.is_favorite ? '★' : '☆'}
          </button>
          <button
            className={`status-btn watched ${media.is_watched ? 'active' : ''}`}
            onClick={handleToggleWatched}
            title={media.is_watched ? t('details.watched.remove') : t('details.watched.add')}
          >
            {media.is_watched ? '✓' : '○'}
          </button>
          {!syncing && media.metadata_fetched_at ? (
            <button
              className="status-btn sync"
              onClick={handleSyncMetadata}
              title={t('details.syncMetadata')}
              disabled={syncing}
            >
              🔄
            </button>
          ) : (
            <button
              className="status-btn sync"
              onClick={handleSyncMetadata}
              title={t('details.syncMetadata')}
              disabled={syncing}
            >
              {syncing ? '⏳' : '🔄'}
            </button>
          )}
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
          <h2 className="details-files-title">{t('details.files', { count: files.length })}</h2>
          <ul className="details-files-list">
            {files.map((file) => (
              <li key={file.id} className="details-file-item">
                <span className="details-file-episode">
                  {file.season !== null && file.episode !== null
                    ? t('details.episode', {
                        season: file.season,
                        episode: file.episode,
                      })
                    : '—'}
                </span>
                <span className="details-file-title">{file.title ?? t('details.noTitle')}</span>
                <button
                  className="details-file-play"
                  onClick={() => {
                    void handlePlayFile(file.file_path);
                  }}
                  title={t('details.play')}
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
