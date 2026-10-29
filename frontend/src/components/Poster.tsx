import type { JSX } from 'react';
import type { Media } from '../types/media';

interface PosterProps {
  media: Media;
  readonly size?: number;
}

export function Poster({ media, size = 160 }: PosterProps): JSX.Element {
  // El reproductor integrado aun no esta disponible (Hito 3), asi que el
  // poster se renderiza como una caja con el tipo de medio y una miniatura
  // placeholder. En el Hito 2 se sustituira por la imagen real de la BD.
  const initial = media.title.charAt(0).toUpperCase();
  const hue = (media.title.charCodeAt(0) * 37) % 360;

  return (
    <figure
      className="poster"
      style={{ width: size, height: Math.round(size * 1.5) }}
    >
      <div
        className="poster-placeholder"
        style={{
          background: `hsl(${hue}, 60%, 25%)`,
          width: size,
          height: Math.round(size * 1.5),
        }}
      >
        <span className="poster-initial">{initial}</span>
        <span className="poster-type">{media.type}</span>
      </div>
      <figcaption className="poster-caption">{media.title}</figcaption>
    </figure>
  );
}
