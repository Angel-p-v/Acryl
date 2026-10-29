import { Poster } from './Poster';
import { useLanguage } from '../contexts/LanguageContext';

import type { Media } from '../types/media';
import type { JSX } from 'react';

type MediaListProps = {
  readonly media: readonly Media[];
  readonly onMediaSelect?: (mediaId: number) => void;
};

export function MediaList({ media, onMediaSelect }: MediaListProps): JSX.Element {
  const { t } = useLanguage();

  if (media.length === 0) {
    return (
      <section className="media-list empty">
        <p>{t('search.noResults')}</p>
      </section>
    );
  }

  return (
    <section className="media-list grid">
      {media.map((m) => (
        <article
          key={m.id}
          className="media-card"
          onClick={() => {
            void onMediaSelect?.(m.id);
          }}
          role={onMediaSelect ? 'button' : undefined}
          tabIndex={onMediaSelect ? 0 : undefined}
          onKeyDown={(e) => {
            if ((e.key === 'Enter' || e.key === ' ') && onMediaSelect) {
              e.preventDefault();
              void onMediaSelect(m.id);
            }
          }}
        >
          <Poster media={m} size={180} />
          <h3 className="media-card-title">{m.title}</h3>
        </article>
      ))}
    </section>
  );
}
