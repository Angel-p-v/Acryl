import type { JSX } from 'react';
import { SearchBar } from './components/SearchBar';
import './index.css';

const DEFAULT_DB_PATH = 'media-manager.db';

export function App(): JSX.Element {
  return (
    <div className="app">
      <header className="app-header">
        <h1>media-manager</h1>
        <p>Biblioteca multimedia de escritorio — Linux</p>
      </header>
      <main className="app-main">
        <SearchBar dbPath={DEFAULT_DB_PATH} />
      </main>
    </div>
  );
}

export default App;
