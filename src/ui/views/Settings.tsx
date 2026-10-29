import { Box, Text, useInput } from 'ink';
import React, { useCallback, useEffect, useState } from 'react';

import { saveConfig, resolveConfigPath } from '../../core/config/loader.js';
import { shortenPath, theme } from '../theme.js';

import type { AppConfig } from '../../core/config/schema.js';

/** Tipo de campo editable. */
type FieldKind = 'text' | 'select';

/** Opción de un campo select. */
type SelectOption = {
  /** Valor que se guardará en la configuración. */
  value: string;
  /** Etiqueta mostrada al usuario. */
  label: string;
};

/** Ítem editable de la configuración. */
type SettingsItem = {
  /** Identificador único del ítem. */
  id: string;
  /** Etiqueta mostrada a la izquierda. */
  label: string;
  /** Tipo de campo: texto libre o selección. */
  kind: FieldKind;
  /** Valor actual. */
  value: string;
  /** Valor original antes de editar (para restaurar al cancelar). */
  originalValue: string;
  /** Si el ítem está en modo edición. */
  editing: boolean;
  /** Opciones disponibles cuando kind === 'select'. */
  options?: SelectOption[];
  /** Índice de la opción seleccionada cuando kind === 'select'. */
  selectedOptionIndex?: number;
  /** Tipo por defecto para una nueva raíz de biblioteca (solo library-root-add). */
  defaultType?: string;
};

/** Construye la lista de ítems editables a partir de la configuración. */
function buildItems(config: AppConfig): SettingsItem[] {
  const items: SettingsItem[] = [];

  // Raíces de la biblioteca (texto).
  config.library.roots.forEach((root, index) => {
    items.push({
      id: `library-root-${index}`,
      label: `${root.type.padEnd(7)}`,
      kind: 'text',
      value: root.path,
      originalValue: root.path,
      editing: false,
    });
  });

  // Opción para añadir una nueva raíz de biblioteca.
  // El tipo por defecto es el tipo de la primera raíz ya configurada (si la hay),
  // de modo que no se asuma siempre «movie». Si no hay raíces, se usa 'movie'.
  const defaultRootType = config.library.roots.length > 0 ? config.library.roots[0]!.type : 'movie';

  items.push({
    id: 'library-root-add',
    label: '+ Añadir raíz',
    kind: 'text',
    value: '',
    originalValue: '',
    editing: false,
    /** Tipo por defecto para la nueva raíz (uno de los ya configurados). */
    defaultType: defaultRootType,
  });

  // Ruta de la base de datos (texto).
  items.push({
    id: 'database-path',
    label: 'Base de datos',
    kind: 'text',
    value: config.database.path,
    originalValue: config.database.path,
    editing: false,
  });

  // Reproductor predeterminado (select).
  const playerOptions: SelectOption[] = Object.keys(config.player.players).map((name) => ({
    value: name,
    label: name,
  }));
  const defaultPlayerIndex = playerOptions.findIndex((opt) => opt.value === config.player.default);
  items.push({
    id: 'player-default',
    label: 'Reproductor predeterminado',
    kind: 'select',
    value: config.player.default,
    originalValue: config.player.default,
    editing: false,
    options: playerOptions,
    selectedOptionIndex: defaultPlayerIndex >= 0 ? defaultPlayerIndex : 0,
  });

  // Modo de lanzamiento (select).
  items.push({
    id: 'player-on-launch',
    label: 'Lanzamiento',
    kind: 'select',
    value: config.player.onLaunch,
    originalValue: config.player.onLaunch,
    editing: false,
    options: [
      { value: 'detach', label: 'Desacoplado' },
      { value: 'foreground', label: 'Primero plano' },
    ],
    selectedOptionIndex: config.player.onLaunch === 'detach' ? 0 : 1,
  });

  // Tema de la UI (select).
  items.push({
    id: 'ui-theme',
    label: 'Tema',
    kind: 'select',
    value: typeof config.ui.theme === 'string' ? config.ui.theme : JSON.stringify(config.ui.theme),
    originalValue:
      typeof config.ui.theme === 'string' ? config.ui.theme : JSON.stringify(config.ui.theme),
    editing: false,
    options: [
      { value: 'default', label: 'Por defecto' },
      { value: 'dark', label: 'Oscuro' },
      { value: 'light', label: 'Claro' },
    ],
    selectedOptionIndex: ['default', 'dark', 'light'].indexOf(
      typeof config.ui.theme === 'string' ? config.ui.theme : 'default',
    ),
  });

  // Diseño de la UI (select).
  items.push({
    id: 'ui-layout',
    label: 'Diseño',
    kind: 'select',
    value: config.ui.layout,
    originalValue: config.ui.layout,
    editing: false,
    options: [
      { value: 'split', label: 'Dividido' },
      { value: 'list', label: 'Lista' },
    ],
    selectedOptionIndex: config.ui.layout === 'split' ? 0 : 1,
  });

  // Nivel de logging (select).
  items.push({
    id: 'logging-level',
    label: 'Logging',
    kind: 'select',
    value: config.logging.level,
    originalValue: config.logging.level,
    editing: false,
    options: [
      { value: 'silent', label: 'Silencioso' },
      { value: 'error', label: 'Error' },
      { value: 'warn', label: 'Aviso' },
      { value: 'info', label: 'Info' },
      { value: 'debug', label: 'Depurar' },
    ],
    selectedOptionIndex: ['silent', 'error', 'warn', 'info', 'debug'].indexOf(config.logging.level),
  });

  return items;
}

/** Vista de configuración editable desde la TUI. */
export function SettingsView({
  config,
  configPath,
  onSave,
  onBack,
}: {
  config: AppConfig;
  /** Ruta del archivo de configuración (para guardar cambios). */
  configPath: string | undefined;
  /** Callback opcional tras guardar cambios (puede ser async). */
  onSave?: () => void | Promise<void>;
  /** Callback para volver a la vista anterior (biblioteca). */
  onBack?: () => void;
}): React.JSX.Element {
  const [items, setItems] = useState<SettingsItem[]>(buildItems(config));
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [status, setStatus] = useState<string | null>(null);
  /** Valor que se está editando actualmente (construido caracter a caracter). */
  const [editingValue, setEditingValue] = useState('');

  // Sincronizamos ítems con la config entrante preservando el estado de edición.
  useEffect(() => {
    const next = buildItems(config);
    const merged = next.map((item) => {
      const existing = items.find((i) => i.id === item.id);
      return { ...item, editing: existing?.editing ?? false };
    });
    setItems(merged);
    if (selectedIndex >= merged.length) {
      setSelectedIndex(Math.max(0, merged.length - 1));
    }
  }, [config, selectedIndex]);

  const itemCount = items.length;
  const hasItems = itemCount > 0;
  const selectedItem = hasItems ? items[selectedIndex] : null;
  const isEditing = selectedItem?.editing ?? false;

  /**
   * Navega al ítem siguiente o anterior dentro de la lista de ajustes.
   */
  const moveSelection = useCallback(
    (delta: number): void => {
      if (!hasItems || isEditing) return;
      setSelectedIndex((prev) => {
        const next = prev + delta;
        if (next < 0) return itemCount - 1;
        if (next >= itemCount) return 0;
        return next;
      });
      setStatus(null);
    },
    [hasItems, itemCount, isEditing],
  );

  /** Entra en modo edición del ítem seleccionado. */
  const startEdit = useCallback((): void => {
    const item = selectedItem;
    if (!item) return;
    setEditingValue(item.value);
    setItems((prev) => prev.map((it) => (it.id === item.id ? { ...it, editing: true } : it)));
    setStatus(
      item.kind === 'select'
        ? '←→ cambia la opción. Enter para guardar, Escape para cancelar.'
        : 'Escribe el nuevo valor. Enter para guardar, Escape para cancelar.',
    );
  }, [selectedItem]);

  /** Guarda el valor editado y persiste la configuración. */
  const saveEdit = useCallback(
    (newValue: string): void => {
      const item = selectedItem;
      if (!item) {
        setStatus('No se puede guardar: no hay ítem seleccionado.');
        return;
      }
      // Si no se pasó configPath, usar la ruta por defecto.
      const effectiveConfigPath = configPath ?? resolveConfigPath();
      if (!effectiveConfigPath) {
        setStatus('No se puede guardar: no hay ruta de configuración.');
        return;
      }

      setItems((prev) =>
        prev.map((it) =>
          it.id === item.id
            ? { ...it, value: newValue, editing: false, originalValue: newValue }
            : it,
        ),
      );

      const nextConfig: AppConfig = { ...config };

      if (item.id === 'library-root-add') {
        if (!newValue.trim()) {
          setStatus('La ruta no puede estar vacía.');
          return;
        }
        const rootType = (item.defaultType ??
          'movie') as AppConfig['library']['roots'][number]['type'];
        nextConfig.library = {
          ...nextConfig.library,
          roots: [...nextConfig.library.roots, { path: newValue.trim(), type: rootType }],
        };
      } else if (item.id.startsWith('library-root-')) {
        const index = Number(item.id.split('-').pop());
        nextConfig.library = {
          ...nextConfig.library,
          roots: nextConfig.library.roots.map((root, i) =>
            i === index ? { ...root, path: newValue } : root,
          ),
        };
      } else if (item.id === 'database-path') {
        nextConfig.database = { ...nextConfig.database, path: newValue };
      } else if (item.id === 'player-default') {
        nextConfig.player = { ...nextConfig.player, default: newValue };
      } else if (item.id === 'player-on-launch') {
        nextConfig.player = {
          ...nextConfig.player,
          onLaunch: newValue as AppConfig['player']['onLaunch'],
        };
      } else if (item.id === 'ui-theme') {
        nextConfig.ui = { ...nextConfig.ui, theme: newValue as AppConfig['ui']['theme'] };
      } else if (item.id === 'ui-layout') {
        nextConfig.ui = { ...nextConfig.ui, layout: newValue as AppConfig['ui']['layout'] };
      } else if (item.id === 'logging-level') {
        nextConfig.logging = {
          ...nextConfig.logging,
          level: newValue as AppConfig['logging']['level'],
        };
      }

      void saveConfig(nextConfig, effectiveConfigPath)
        .then(() => {
          setStatus('Guardado.');
          setEditingValue('');
          void Promise.resolve(onSave?.());
        })
        .catch(() => {
          setStatus('No se pudo guardar la configuración.');
        });
    },
    [selectedItem, config, configPath, onSave],
  );

  /** Cambia la opción seleccionada en un campo select. */
  const changeOption = useCallback(
    (delta: number): void => {
      const item = selectedItem;
      if (!item) return;
      if (item.kind !== 'select' || !item.options) return;
      const current = item.selectedOptionIndex ?? 0;
      const next = (current + delta + item.options.length) % item.options.length;
      const newValue = item.options[next]!.value;
      setItems((prev) =>
        prev.map((it) =>
          it.id === item.id
            ? {
                ...it,
                value: newValue,
                selectedOptionIndex: next,
              }
            : it,
        ),
      );
      setEditingValue(newValue);
    },
    [selectedItem],
  );

  /** Cancela la edición y restaura el valor original. */
  const cancelEdit = useCallback((): void => {
    const item = selectedItem;
    if (!item) return;
    setEditingValue('');
    setItems((prev) =>
      prev.map((it) =>
        it.id === item.id ? { ...it, value: it.originalValue, editing: false } : it,
      ),
    );
    setStatus('Edición cancelada.');
  }, [selectedItem]);

  /**
   * Maneja el teclado dentro de la vista de ajustes.
   * Cuando se está editando, captura caracteres para construir el nuevo valor.
   * Return entra en modo edición (cuando no se edita) o guarda (cuando se edita).
   */
  useInput((input, key) => {
    // Ctrl+C siempre sale sin guardar.
    if (key.ctrl && input === 'c') {
      setItems((prev) => prev.map((item) => ({ ...item, editing: false })));
      setEditingValue('');
      setStatus(null);
      return;
    }

    // Escape: cancela edición o vuelve a la biblioteca.
    if (key.escape) {
      const item = selectedItem;
      if (isEditing && item) {
        cancelEdit();
      } else {
        setItems((prev) => prev.map((it) => ({ ...it, editing: false })));
        onBack?.();
      }
      return;
    }

    if (isEditing) {
      const item = selectedItem;
      if (!item) return;
      if (item.kind === 'select') {
        // En modo select: ←→ cambia opción, Return guarda.
        if (key.leftArrow) {
          changeOption(-1);
          return;
        }
        if (key.rightArrow) {
          changeOption(1);
          return;
        }
        if (key.return) {
          saveEdit(item.value);
          return;
        }
        return;
      }

      // En modo texto: Return guarda, cualquier otro caracter se añade al valor.
      if (key.return) {
        saveEdit(editingValue);
        return;
      }
      // Retroceso: borrar último caracter.
      if (key.backspace || key.delete) {
        setEditingValue((prev) => prev.slice(0, -1));
        return;
      }
      // Caracter imprimible o texto pegado: añadir al valor.
      // Si input tiene varios caracteres (pegado) y no es una tecla de
      // navegación especial (flechas, inicio, fin, etc.), se trata como
      // texto pegado y se inserta completo.
      const isNavigationKey =
        key.leftArrow ||
        key.rightArrow ||
        key.upArrow ||
        key.downArrow ||
        key.home ||
        key.end ||
        key.pageUp ||
        key.pageDown;
      if (!isNavigationKey && input.length > 0) {
        setEditingValue((prev) => prev + input);
        return;
      }
      return;
    }

    // No estamos editando: navegamos con flechas y Return entra en modo edición.
    if (key.upArrow) {
      moveSelection(-1);
      return;
    }
    if (key.downArrow) {
      moveSelection(1);
      return;
    }
    if (key.return || input === ' ') {
      startEdit();
      return;
    }
    // Tab vuelve a la biblioteca.
    if (key.tab) {
      onBack?.();
      return;
    }
  });

  return (
    <Box flexDirection="column" flexGrow={1} paddingX={1} paddingY={1}>
      <Text>{theme.highlight('Configuración efectiva')}</Text>
      <Text> </Text>
      <Text>
        {theme.muted(
          isEditing && selectedItem?.kind === 'select'
            ? '←→ cambia · Enter guarda · Escape cancela/volver · q sale'
            : '↑↓ navegan · Enter edita · Escape cancela/volver · q sale',
        )}
      </Text>
      <Text> </Text>

      {hasItems ? (
        items.map((item, index) => {
          const isSelected = index === selectedIndex;
          const isEditingThis = item.editing;
          return (
            <Box key={item.id} flexDirection="row" marginBottom={1}>
              <Text>{isSelected ? '▸ ' : '  '}</Text>
              <Text>{item.label}</Text>
              <Text> </Text>
              {isEditingThis ? (
                <Text>
                  {item.kind === 'select' && item.options ? (
                    <>
                      {item.options.map((opt, optIndex) => (
                        <Text key={opt.value}>
                          {optIndex === (item.selectedOptionIndex ?? 0)
                            ? theme.primary(`[${opt.label}]`)
                            : theme.muted(`(${opt.label})`)}
                        </Text>
                      ))}
                    </>
                  ) : (
                    theme.primary(editingValue)
                  )}
                  {theme.muted(' █')}
                </Text>
              ) : (
                <Text>
                  {isSelected
                    ? theme.primary(shortenPath(item.value, 50))
                    : theme.muted(shortenPath(item.value, 50))}
                </Text>
              )}
            </Box>
          );
        })
      ) : (
        <Text>{theme.muted('No hay elementos configurables.')}</Text>
      )}

      <Text> </Text>
      {status && <Text>{theme.muted(status)}</Text>}
      <Text>{theme.muted('Presiona «Escape» o «Tab» para volver a la biblioteca.')}</Text>
    </Box>
  );
}
