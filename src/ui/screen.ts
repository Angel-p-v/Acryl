const ENTER_ALTERNATE_SCREEN = '\u001b[?1049h';
const LEAVE_ALTERNATE_SCREEN = '\u001b[?1049l';

/** Lo mínimo que necesita `screen.ts` de un stream para poder testearlo. */
export type ScreenStream = {
  readonly isTTY?: boolean;
  write(chunk: string): boolean;
};

/** Entra en la pantalla alternativa (la TUI ocupa toda la terminal). */
export function enterAlternateScreen(stream: ScreenStream = process.stdout): void {
  if (stream.isTTY !== true) return;
  stream.write(ENTER_ALTERNATE_SCREEN);
}

/** Restaura la pantalla normal y el cursor. */
export function leaveAlternateScreen(stream: ScreenStream = process.stdout): void {
  if (stream.isTTY !== true) return;
  stream.write(LEAVE_ALTERNATE_SCREEN);
}
