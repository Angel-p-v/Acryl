import { SearchBar } from './components/SearchBar';
import { VideoPlayer } from './components/VideoPlayer';
import { LanguageProvider, useLanguage } from './contexts/LanguageContext';
import './index.css';

import type { JSX } from 'react';

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
          <VideoPlayer />
        </main>
      </div>
    </LanguageProvider>
  );
}

export default App;
