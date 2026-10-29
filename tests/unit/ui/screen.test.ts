import { describe, expect, it, vi } from 'vitest';

import {
  enterAlternateScreen,
  leaveAlternateScreen,
  type ScreenStream,
} from '../../../src/ui/screen.js';

function streamWith(write: (chunk: string) => boolean, isTTY: boolean): ScreenStream {
  return { isTTY, write };
}

describe('pantalla alternativa', () => {
  it('entra con la secuencia ANSI correcta en una terminal', () => {
    const write = vi.fn(() => true);

    enterAlternateScreen(streamWith(write, true));

    expect(write).toHaveBeenCalledWith('\u001b[?1049h');
  });

  it('sale con la secuencia ANSI correcta en una terminal', () => {
    const write = vi.fn(() => true);

    leaveAlternateScreen(streamWith(write, true));

    expect(write).toHaveBeenCalledWith('\u001b[?1049l');
  });

  it('no escribe nada si la salida no es una terminal', () => {
    const write = vi.fn(() => true);
    const stdout = streamWith(write, false);

    enterAlternateScreen(stdout);
    leaveAlternateScreen(stdout);

    expect(write).not.toHaveBeenCalled();
  });
});
