import type { JSX } from 'react';

export function App(): JSX.Element {
  return (
    <div className="app">
      <header className="app-header">
        <h1>media-manager</h1>
        <p>Gestor de contenido multimedia — app de escritorio (Linux)</p>
      </header>
      <main className="app-main">
        <section className="placeholder">
          <p>Esqueleto del frontend (React + TypeScript + Tauri).</p>
          <p>Hito 1: prototipo mínimo en curso.</p>
        </section>
      </main>
    </div>
  );
}

