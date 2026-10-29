import { SearchBar } from './components/SearchBar';
import { VideoPlayer } from './components/VideoPlayer';
import { LanguageProvider, useLanguage } from './contexts/LanguageContext';
import { indexFolder } from './services/tauri';
import './index.css';

import { useCallback, useState, type JSX } from 'react';

const DEFAULT_DB_PATH = 'acryl.db';

function LanguageSelector(): JSX.Element {
  const { locale, setLocale, t } = useLanguage() as ReturnType<typeof useLanguage>;

  return (
    <div className="language-selector">
      <label htmlFor="language-select">{t('language.label')}</label>
      <select
        id="language-select"
        value={locale}
        onChange={(e) => {
          void setLocale(e.target.value as 'es' | 'en');
        }}
      >
        <option value="es">{t('language.spanish')}</option>
        <option value="en">{t('language.english')}</option>
      </select>
    </div>
  );
}

function FolderIndexer(): JSX.Element {
  const { t } = useLanguage();
  const [folderPath, setFolderPath] = useState('');
  const [indexing, setIndexing] = useState(false);
  const [result, setResult] = useState<{ media_added: number; files_added: number; folders_processed: number } | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleIndex = useCallback(async (): Promise<void> => {
    setError(null);
    setResult(null);
    setIndexing(true);
    try {
      const r = await indexFolder(DEFAULT_DB_PATH, folderPath);
      setResult(r);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Error indexando carpeta');
    } finally {
      setIndexing(false);
    }
  }, [folderPath, t]);

  return (
    <section className="folder-indexer">
      <h3>{t('index.folder.title')}</h3>
      <div className="indexer-row">
        <input
          id="folder-path"
          type="text"
          placeholder={t('index.folder.placeholder')}
          value={folderPath}
          onChange={(e) => setFolderPath(e.target.value)}
          disabled={indexing}
        />
        <button
          className="control-btn"
          onClick={handleIndex}
          disabled={indexing || !folderPath}
          title={t('index.folder.button')}
        >
          {t('index.folder.button')}
        </button>
      </div>
      {indexing && <p className="loading">{t('index.folder.indexing')}</p>}
      {result && (
        <div className="index-result">
          <p>
            {t('index.folder.result', {
              media: result.media_added,
              files: result.files_added,
              folders: result.folders_processed,
            })}
          </p>
        </div>
      )}
      {error && <p className="error">{error}</p>}
    </section>
  );
}

export function App(): JSX.Element {
  return (
    <LanguageProvider>
      <div className="app">
        <header className="app-header">
          <h1>{useLanguage()?.t('app.title') ?? 'Acryl'}</h1>
          <p>{useLanguage()?.t('app.subtitle') ?? 'Biblioteca multimedia de escritorio — Linux'}</p>
          <LanguageSelector />
        </header>
        <main className="app-main">
          <SearchBar dbPath={DEFAULT_DB_PATH} />
          <FolderIndexer />
          <VideoPlayer />
        </main>
      </div>
    </LanguageProvider>
  );
}

export default App;
