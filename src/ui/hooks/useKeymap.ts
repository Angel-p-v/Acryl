import { useInput, type Key } from 'ink';

import type { Keybindings } from './useConfig.js';

export type KeyHandler = () => void;
export type KeyHandlers = Record<string, KeyHandler | undefined>;

/**
 * Escucha el teclado y despacha la acción cuyo atajo coincide.
 *
 * Los handlers deciden si la acción aplica a la vista actual; así una tecla
 * escrita dentro del buscador no dispara "salir" ni "ayuda".
 */
export function useKeymap(bindings: Keybindings, handlers: KeyHandlers, enabled = true): void {
  useInput((input, key) => {
    if (!enabled) return;

    // Ctrl+C siempre cierra, incluso si la vista está en modo búsqueda.
    if (key.ctrl && input === 'c') {
      (handlers.forceQuit ?? handlers.quit)?.();
      return;
    }

    for (const [action, handler] of Object.entries(handlers)) {
      if (handler === undefined) continue;
      const binding = bindings[action];
      if (binding !== undefined && binding.length > 0 && matchesKey(binding, input, key)) {
        handler();
        return;
      }
    }
  });
}

/** Comprueba si una pulsación corresponde a un atajo configurado. */
export function matchesKey(binding: string, input: string, key: Key): boolean {
  switch (binding) {
    case 'up':
      return key.upArrow;
    case 'down':
      return key.downArrow;
    case 'left':
      return key.leftArrow;
    case 'right':
      return key.rightArrow;
    case 'return':
    case 'enter':
      return key.return;
    case 'escape':
    case 'esc':
      return key.escape;
    case 'tab':
      return key.tab;
    case 'backspace':
      return key.backspace;
    case 'delete':
      return key.delete;
    case 'space':
      return input === ' ';
    default:
      return binding.length === 1 && input === binding;
  }
}
