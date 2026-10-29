/** Genera scripts de autocompletado de shell para mm. */

import { ValidationError } from '../shared/errors.js';

export type CompletionShell = 'bash' | 'zsh' | 'fish';

/** Genera el script de autocompletado para Bash. */
function generateBashCompletion(): string {
  return [
    '# media-manager shell completion for Bash',
    '',
    '_mm_completion() {',
    '  local cur prev words cword',
    '  _init_completion || return',
    '',
    '  local mm_cmds="index list search show play mark favorite tag doctor db watch config metadata completion"',
    '',
    '  case "${prev}" in',
    '    mm)',
    '      COMPREPLY=( $(compgen -W "${mm_cmds}" -- "${cur}") )',
    '      return 0',
    '      ;;',
    '    index|search|show|play|mark|favorite|tag|doctor|db|watch|config|metadata|completion)',
    '      # No hay subcomandos adicionales por ahora',
    '      COMPREPLY=()',
    '      return 0',
    '      ;;',
    '    *)',
    '      COMPREPLY=()',
    '      ;;',
    '  esac',
    '}',
    '',
    'complete -F _mm_completion mm',
    '',
  ].join('\n');
}

/** Genera el script de autocompletado para Zsh. */
function generateZshCompletion(): string {
  return [
    '#compdef mm',
    '',
    '# media-manager shell completion for Zsh',
    '',
    '_mm() {',
    '  local -a commands',
    '  commands=(',
    "    'index:indexar la biblioteca'",
    "    'list:enumar los títulos de la biblioteca'",
    "    'search:buscar títulos'",
    "    'show:mostrar detalle de un título'",
    "    'play:reproducir un título'",
    "    'mark:marcar estado visto/no visto'",
    "    'favorite:marcar como favorito'",
    "    'tag:gestionar etiquetas'",
    "    'doctor:diagnóstico de la biblioteca'",
    "    'db:operaciones de base de datos'",
    "    'watch:vigilar cambios en la biblioteca'",
    "    'config:mostrar configuración'",
    "    'metadata:gestionar metadatos externos'",
    "    'self-update:auto-actualizar'",
    "    'completion:generar script de autocompletado'",
    "    'help:mostrar ayuda'",
    '  )',
    '',
    '  _arguments -s \\',
    "    '(-h --help)'{-h,--help}[muestra la ayuda] \\",
    "    '(-V --version)'{-V,--version}[muestra la versión] \\",
    "    '*: :_mm_commands'",
    '}',
    '',
    '# Definir comandos para completion',
    '_mm_commands() {',
    "  _values 'comando' \\",
    "    'index[indexar la biblioteca]' \\",
    "    'list[enumar los títulos de la biblioteca]' \\",
    "    'search[buscar títulos]' \\",
    "    'show[mostrar detalle de un título]' \\",
    "    'play[reproducir un título]' \\",
    "    'mark[marcar estado visto/no visto]' \\",
    "    'favorite[marcar como favorito]' \\",
    "    'tag[gestionar etiquetas]' \\",
    "    'doctor[diagnóstico de la biblioteca]' \\",
    "    'db[operaciones de base de datos]' \\",
    "    'watch[vigilar cambios en la biblioteca]' \\",
    "    'config[mostrar configuración]' \\",
    "    'metadata[gestionar metadatos externos]' \\",
    "    'self-update[auto-actualizar]' \\",
    "    'completion[generar script de autocompletado]' \\",
    "    'help[mostrar ayuda]'",
    '}',
    '',
  ].join('\n');
}

/** Genera el script de autocompletado para Fish. */
function generateFishCompletion(): string {
  return [
    '# media-manager shell completion for Fish',
    '',
    'function __fish_mm_using_command',
    '  set -l cmd (commandline -opc)',
    '  test (count $cmd) -eq 1',
    'end',
    '',
    'function __fish_mm_using_command',
    '  set -l cmd (commandline -opc)',
    '  test (count $cmd) -eq 1',
    'end',
    '',
    "complete -c mm -f -a 'index' -d 'indexar la biblioteca'",
    "complete -c mm -f -a 'list' -d 'enumar los títulos de la biblioteca'",
    "complete -c mm -f -a 'search' -d 'buscar títulos'",
    "complete -c mm -f -a 'show' -d 'mostrar detalle de un título'",
    "complete -c mm -f -a 'play' -d 'reproducir un título'",
    "complete -c mm -f -a 'mark' -d 'marcar estado visto/no visto'",
    "complete -c mm -f -a 'favorite' -d 'marcar como favorito'",
    "complete -c mm -f -a 'tag' -d 'gestionar etiquetas'",
    "complete -c mm -f -a 'doctor' -d 'diagnóstico de la biblioteca'",
    "complete -c mm -f -a 'db' -d 'operaciones de base de datos'",
    "complete -c mm -f -a 'watch' -d 'vigilar cambios en la biblioteca'",
    "complete -c mm -f -a 'config' -d 'mostrar configuración'",
    "complete -c mm -f -a 'metadata' -d 'gestionar metadatos externos'",
    "complete -c mm -f -a 'self-update' -d 'auto-actualizar'",
    "complete -c mm -f -a 'completion' -d 'generar script de autocompletado'",
    "complete -c mm -f -a 'help' -d 'mostrar ayuda'",
    '',
  ].join('\n');
}

/**
 * Genera el script de autocompletado para la shell especificada.
 *
 * @param shell - nombre de la shell ('bash', 'zsh', 'fish')
 * @returns el script de autocompletado como cadena
 * @throws Error si la shell no es soportada
 */
export function generateCompletion(shell: string): string {
  switch (shell) {
    case 'bash':
      return generateBashCompletion();
    case 'zsh':
      return generateZshCompletion();
    case 'fish':
      return generateFishCompletion();
    default:
      throw new ValidationError(
        'Shell no soportada: ' + String(shell) + '. Usa "bash", "zsh" o "fish".',
      );
  }
}

/** Lista de shells soportadas. */
export const supportedShells: CompletionShell[] = ['bash', 'zsh', 'fish'];
