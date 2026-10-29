import { useCallback, useEffect, useRef, useState } from 'react';

import {
  playerOpen,
  playerPause,
  playerPlay,
  playerSeek,
  playerStop,
  playerStatus,
  playerVolume,
  recordPlayback,
} from '../services/tauri';
import { useLanguage } from '../contexts/LanguageContext';

import type { PlayerStatus } from '../types/media';
import type { JSX } from 'react';

const UPDATE_INTERVAL_MS = 1000;

export function VideoPlayer(): JSX.Element {
  const { t } = useLanguage();
  const containerRef = useRef<HTMLDivElement>(null);
  const [filePath, setFilePath] = useState<string>('');
  const [status, setStatus] = useState<PlayerStatus | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [inputPath, setInputPath] = useState<string>('');

  const updateStatus = useCallback(async (): Promise<void> => {
    try {
      const s = await playerStatus();
      setStatus(s);
    } catch {
      // Si el reproductor no está activo, ignorar silenciosamente.
    }
  }, []);

  useEffect(() => {
    if (!status?.paused && status?.duration && status.duration > 0) {
      const timer = setInterval(() => {
        void updateStatus();
      }, UPDATE_INTERVAL_MS);
      return () => clearInterval(timer);
    }
  }, [status, updateStatus]);

  const handlePlay = useCallback(async (): Promise<void> => {
    setError(null);
    try {
      await playerPlay();
      await updateStatus();
    } catch (e) {
      setError(e instanceof Error ? e.message : t('player.errorPlay'));
    }
  }, [updateStatus, t]);

  const handlePause = useCallback(async (): Promise<void> => {
    setError(null);
    try {
      await playerPause();
      await updateStatus();
    } catch (e) {
      setError(e instanceof Error ? e.message : t('player.errorPause'));
    }
  }, [updateStatus, t]);

  const handleStop = useCallback(async (): Promise<void> => {
    setError(null);
    try {
      await playerStop();
      await updateStatus();
    } catch (e) {
      setError(e instanceof Error ? e.message : t('player.errorStop'));
    }
  }, [updateStatus, t]);

  const handleSeek = useCallback(
    async (seconds: number): Promise<void> => {
      setError(null);
      try {
        await playerSeek(seconds);
        await updateStatus();
      } catch (e) {
        setError(e instanceof Error ? e.message : t('player.errorSeek'));
      }
    },
    [updateStatus, t]
  );

  const handleVolume = useCallback(
    async (volume: number): Promise<void> => {
      setError(null);
      try {
        await playerVolume(volume);
        await updateStatus();
      } catch (e) {
        setError(e instanceof Error ? e.message : t('player.errorVolume'));
      }
    },
    [updateStatus, t]
  );

  const handleOpenFile = useCallback(
    async (path: string): Promise<void> => {
      setIsLoading(true);
      setError(null);
      try {
        await playerOpen(path);
        setFilePath(path);
        await updateStatus();
        // Registrar reproducción en el historial (posición 0 al abrir;
        // la duración real se actualiza al cambiar el estado del reproductor)
        await recordPlayback('acryl.db', 0, 0, 0).catch(() => {
          // No fallar si no se puede registrar el historial
        });
      } catch (e) {
        setError(e instanceof Error ? e.message : t('player.errorOpen'));
      } finally {
        setIsLoading(false);
      }
    },
    [updateStatus, t]
  );

  const handleOpenInput = useCallback(async (): Promise<void> => {
    await handleOpenFile(inputPath);
    setInputPath('');
  }, [handleOpenFile, inputPath]);

  const onStop = useCallback((): void => {
    void handleStop();
  }, [handleStop]);

  const onPlayPause = useCallback((): void => {
    if (status?.paused) {
      void handlePlay();
    } else {
      void handlePause();
    }
  }, [status?.paused, handlePlay, handlePause]);

  const onOpenInput = useCallback((): void => {
    void handleOpenInput();
  }, [handleOpenInput]);

  const onOpenInputKeyPress = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>): void => {
      if (e.key === 'Enter') {
        void handleOpenInput();
      }
    },
    [handleOpenInput]
  );

  const duration = status?.duration ?? 0;
  const progress = duration > 0 ? ((status?.timePosition ?? 0) / duration) * 100 : 0;

  void progress;

  return (
    <div className="video-player">
      <div className="video-open-row">
        <input
          type="text"
          placeholder={t('player.openFileLabel')}
          value={inputPath}
          onChange={(e) => setInputPath(e.target.value)}
          onKeyPress={onOpenInputKeyPress}
          disabled={isLoading}
        />
        <button
          className="control-btn"
          onClick={onOpenInput}
          disabled={isLoading || !inputPath}
          title={t('player.openButtonTitle')}
        >
          {t('player.openButton')}
        </button>
      </div>

      <div className="video-container" ref={containerRef}>
        {filePath ? (
          <>
            {error && <div className="video-error">{error}</div>}
            {!error && status && (
              <div className="video-info">
                <span className="time-current">{formatTime(status.timePosition)}</span>
                <span className="time-separator"> / </span>
                <span className="time-duration">{formatTime(status.duration)}</span>
              </div>
            )}
            {!isLoading && !error && status && !status.paused && (
              <div className="video-playing-indicator">▶</div>
            )}
          </>
        ) : (
          <div className="video-placeholder">
            <p>{t('player.placeholder')}</p>
          </div>
        )}
      </div>

      {filePath && (
        <div className="video-controls">
          <div className="control-row">
            <button
              className="control-btn"
              onClick={onStop}
              disabled={isLoading}
              title={t('player.stop')}
            >
              ⏹
            </button>
            <button
              className="control-btn"
              onClick={onPlayPause}
              disabled={isLoading}
              title={status?.paused ? t('player.play') : t('player.pause')}
            >
              {status?.paused ? '▶' : '⏸'}
            </button>
          </div>

          <div className="control-row control-seek">
            <input
              type="range"
              min="0"
              max={status?.duration ?? 100}
              step="1"
              value={status?.timePosition ?? 0}
              onChange={(e) => {
                void handleSeek(Number(e.target.value));
              }}
              disabled={isLoading || !status?.duration}
            />
          </div>

          <div className="control-row control-volume">
            <span className="control-label">{t('player.volume')}</span>
            <input
              type="range"
              min="0"
              max="100"
              step="1"
              value={status?.volume ?? 70}
              onChange={(e) => {
                void handleVolume(Number(e.target.value));
              }}
              disabled={isLoading}
            />
          </div>
        </div>
      )}
    </div>
  );
}

function formatTime(seconds: number): string {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins}:${secs.toString().padStart(2, '0')}`;
}
