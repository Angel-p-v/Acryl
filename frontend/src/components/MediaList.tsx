import { Poster } from './Poster';

import type { Media } from '../types/media';
import type { JSX } from 'react';

type MediaListProps = {
  readonly media: readonly Media[];
};

export function MediaList({ media }: MediaListProps): JSX.Element {
  if (media.length === 0) {
    return (
      <section className="media-list empty">
        <p>No se encontraron medios.</p>
      </section>
    );
  }

  return (
    <section className="media-list grid">
      {media.map((m) => (
        <article key={m.id} className="media-card">
          <Poster media={m} size={180} />
        </article>
      ))}
    </section>
  );
}
