import { Box, Text, useInput, type Key } from 'ink';
import React, { useCallback, useMemo, useState } from 'react';

import { defaultConfig } from '../../core/config/defaults.js';

import type { AppConfig } from '../../core/config/schema.js';

export type KeybindingsViewProps = {
  config: AppConfig;
  /** Guarda la configuración actualizada en el archivo YAML. */
  onSave: (config: AppConfig) => Promise<void>;
  /** Vuelve a la vista anterior. */
  onBack: () => void;
};

/** Vista para reasignar atajos de teclado desde la TUI (T2 del plan-9). */
export function KeybindingsView({
  config,
  onSave,
  onBack,
}: KeybindingsViewProps): React.JSX.Element {
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [editingAction, setEditingAction] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const actions = useMemo<{ action: string; key: string }[]>(
    () =>
      Object.keys(defaultConfig.keybindings).map((action) => ({
        action,
        key: config.keybindings[action] ?? defaultConfig.keybindings[action] ?? '',
      })),
    [config],
  );

  const handleKey = useCallback(
    (input: string, key: Key) => {
      // Ctrl+C siempre sale de la vista de edición sin guardar.
      if (key.ctrl && input === 'c') {
        setEditingAction(null);
        setStatus(null);
        return;
      }

      // Si estamos editando una acción, capturar la nueva tecla.
      if (editingAction !== null) {
        // Solo se reasignan teclas imprimibles de un solo carácter.
        const newKey = input.length === 1 ? input : undefined;
        if (!newKey) return;

        const nextKeybindings: Record<string, string> = {
          ...config.keybindings,
          [editingAction]: newKey,
        };
        const nextConfig: AppConfig = { ...config, keybindings: nextKeybindings };

        setEditingAction(null);
        setSaving(true);
        onSave(nextConfig)
          .then(() => {
            setSaving(false);
            setStatus(`Atajo de «${editingAction}» cambiado a «${newKey}».`);
          })
          .catch(() => {
            setSaving(false);
            setStatus('No se pudo guardar la configuración.');
          });
        return;
      }

      // Navegación normal.
      if (key.escape || input === 'q') {
        onBack();
        return;
      }

      if (key.upArrow) {
        setSelectedIndex((prev) => (prev > 0 ? prev - 1 : actions.length - 1));
        return;
      }
      if (key.downArrow) {
        setSelectedIndex((prev) => (prev < actions.length - 1 ? prev + 1 : 0));
        return;
      }

      if ((key.return || input === ' ') && editingAction === null) {
        const selected = actions[selectedIndex];
        if (selected) setEditingAction(selected.action);
        return;
      }
    },
    [editingAction, actions, selectedIndex, config, onSave, onBack],
  );

  useInput(handleKey);

  return (
    <Box flexDirection="column" paddingX={1} paddingY={1}>
      <Text>{saving ? 'Guardando…' : 'Atajos de teclado'}</Text>
      <Text> </Text>
      <Text>
        {editingAction
          ? 'Pulsa una tecla para reasignar · Escape para cancelar'
          : '↑↓ navegan · Enter edita · q vuelve'}
      </Text>
      <Text> </Text>
      {actions.map(({ action, key }, index) => {
        const isSelected = index === selectedIndex;
        const isEditing = editingAction === action;
        return (
          <Text key={action}>
            {isEditing ? '▶ ' : isSelected ? '▸ ' : '  '}
            {action}: {key}
            {isEditing ? ' (escucha nueva tecla…)' : ''}
          </Text>
        );
      })}
      <Text> </Text>
      {status && <Text>{status}</Text>}
      <Text>{saving ? 'Esperando a que se guarde…' : ''}</Text>
    </Box>
  );
}
