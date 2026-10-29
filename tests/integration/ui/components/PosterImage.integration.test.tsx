import { cleanup, render } from 'ink-testing-library';
import React from 'react';
import { afterEach, describe, expect, it } from 'vitest';

import { PosterImage } from '../../../../src/ui/components/PosterImage.js';

import type { PosterImageProps } from '../../../../src/ui/components/PosterImage.js';

function renderPoster(props: Partial<PosterImageProps> = {}) {
  const { imagePath = null, width = 40, height = 12 } = props;
  return render(<PosterImage imagePath={imagePath} width={width} height={height} />);
}

/** Normaliza el frame eliminando saltos de línea y colapsando espacios. */
function normalizeFrame(frame: string): string {
  return frame.replace(/\s+/g, ' ').trim();
}

describe('PosterImage', () => {
  afterEach(() => {
    cleanup();
  });

  it('muestra "[ PÓSTER ] (sin póster)" cuando no hay ruta de imagen', () => {
    const { lastFrame } = renderPoster({ imagePath: null });

    const frame = normalizeFrame(lastFrame() ?? '');
    expect(frame).toContain('[ PÓSTER ]');
    expect(frame).toContain('(sin póster)');
  });

  it('no muestra contenido de póster cuando la ruta es null', () => {
    const { lastFrame } = renderPoster({ imagePath: null });

    const frame = normalizeFrame(lastFrame() ?? '');
    expect(frame).toContain('(sin póster)');
  });
});
