import { Box, Text, useApp, useInput } from 'ink';
import React, { useCallback, useEffect, useMemo, useState } from 'react';

import { Header } from './components/Header.js';
import { HelpOverlay } from './components/HelpOverlay.js';
import { KeybindingsView } from './components/KeybindingsView.js';
import { StatusBar } from './components/StatusBar.js';
import { useConfig } from './hooks/useConfig.js';
import { matchesKey, useKeymap } from './hooks/useKeymap.js';
import { useLibrary } from './hooks/useLibrary.js';
import { useMediaDetails } from './hooks/useMediaDetails.js';
import { theme, setTheme } from './theme.js';
import { LibraryView } from './views/Library.js';
import { PlayDetails } from './views/PlayDetails.js';
import { SearchView } from './views/Search.js';
import { SettingsView } from './views/Settings.js';
import { saveConfig, loadConfig } from '../core/config/loader.js';
import { defaultConfigPath } from '../core/config/paths.js';
import { createLaunchPlan, launchPlayer as spawnPlayer } from '../core/player/launcher.js';
import { selectMediaFile } from '../core/player/selection.js';
import { formatError } from '../shared/errors.js';

import type { Counts, Section, ViewName } from './types.js';
import type { AppConfig } from '../core/config/schema.js';
import type { MediaFileRow } from '../core/db/types.js';
import type { LibraryService } from '../core/library/service.js';
import type { LaunchPlayerFn } from '../core/player/launcher.js';

export type AppProps = {
  config: AppConfig;
  library: LibraryService;
  /** Ruta del archivo de configuración (para guardar cambios desde la TUI). */
  configPath?: string;
  /** Solo para tests: sustituye al cierre real de Ink. */
  onQuit?: () => void;
  /** Solo para tests: permite observar o simular el lanzamiento del reproductor. */
  launchPlayer?: LaunchPlayerFn;
  /** Indexa la biblioteca (se llama tras guardar cambios en ajustes). Recibe la config actualizada. */
  onIndex?: (newConfig: AppConfig) => Promise<void>;
};

const SECTION_ORDER: readonly Section[] = [
  'all',
  'in-progress',
  'favorites',
  'series',
  'anime',
  'movie',
];

const VIEW_TITLE: Record<ViewName, string> = {
  library: 'Biblioteca',
  search: 'Búsqueda',
  settings: 'Ajustes',
  'play-details': 'Reproducir',
};

/**
 * Componente raíz de la TUI.
 *
 * Mantiene el estado de navegación local (vista, sección, selección y búsqueda) y
 * delega todas las consultas en `LibraryService`. No escribe SQL ni conoce SQLite.
 */
function AppImpl({
  config: initialConfig,
  library,
  configPath,
  onQuit,
  launchPlayer = spawnPlayer,
  onIndex,
}: AppProps): React.JSX.Element {
  const [config, setConfig] = useState<AppConfig>(initialConfig);
  const { keybindings } = useConfig(config);
  setTheme(config.ui.theme);
  // Siempre hay una ruta de configuración válida: si no se pasa una explícita
  // (primera ejecución sin archivo YAML), se usa la ruta por defecto. Así
  // saveEdit / saveConfig pueden crear el archivo al guardar los ajustes.
  const resolvedConfigPath = configPath ?? defaultConfigPath();
  const { exit } = useApp();
  const [view, setView] = useState<ViewName>('library');
  const [section, setSection] = useState<Section>('all');
  const [selected, setSelected] = useState(0);
  const [query, setQuery] = useState('');
  const [helpVisible, setHelpVisible] = useState(false);
  const [status, setStatus] = useState<string | null>(null);
  const [refreshingMetadata, setRefreshingMetadata] = useState(false);
  const [detailsVisible, setDetailsVisible] = useState(config.ui.showDetails);
  /** IDs de títulos seleccionados (selección múltiple, D3). */
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set());
  /** Modo compacto: más filas por pantalla (D2). */
  const [compactMode, setCompactMode] = useState(config.ui.compact ?? false);
  /** Filtro adicional: 'watched' | 'unwatched' | undefined (D1). */
  const [watchFilter, setWatchFilter] = useState<'watched' | 'unwatched' | undefined>(undefined);
  /** Vista de reasignación de atajos abierta (T2). */
  const [keybindingsViewVisible, setKeybindingsViewVisible] = useState(false);
  /** ID del título abierto en la vista de detalles de reproducción (play-details). */
  const [playDetailsMediaId, setPlayDetailsMediaId] = useState<number | null>(null);
  /** Clave de refresco de metadatos: se incrementa tras obtener metadatos externos para
   *  forzar a MediaDetails a volver a leer de la BD y mostrar el póster recién descargado. */
  const [metadataRefreshKey, setMetadataRefreshKey] = useState(0);
  /** Versión de datos: se incrementa tras indexar para forzar la re-lectura de la BD. */
  const [dataVersion, setDataVersion] = useState(0);

  const type =
    section === 'all' || section === 'in-progress' || section === 'favorites' ? undefined : section;
  const { items, total, error } = useLibrary(library, {
    limit: view === 'search' ? 100 : 200,
    ...(type === undefined ? {} : { type }),
    ...(view === 'search' ? { query } : {}),
    section:
      section === 'in-progress' ? 'in-progress' : section === 'favorites' ? 'favorites' : undefined,
    ...(watchFilter === 'watched' ? { watched: true } : {}),
    ...(watchFilter === 'unwatched' ? { unwatched: true } : {}),
    dataVersion,
  });
  const safeSelected = items.length === 0 ? 0 : Math.min(selected, items.length - 1);
  const selectedMedia = items[safeSelected];
  const details = useMediaDetails(library, selectedMedia?.id);

  const counts = useMemo<Counts>(() => {
    try {
      const stats = library.stats();
      const favoritesCount = library.countFavorites();
      return {
        all: stats.media.total,
        'in-progress': stats.inProgress,
        favorites: favoritesCount,
        series: stats.media.series,
        anime: stats.media.anime,
        movie: stats.media.movie,
        unknown: stats.media.unknown,
      };
    } catch {
      return { all: 0, 'in-progress': 0, favorites: 0, series: 0, anime: 0, movie: 0, unknown: 0 };
    }
  }, [library]);

  useEffect(() => {
    setSelected(0);
  }, [view, section, query]);

  // Sincronización automática de metadatos expirados al montar la TUI.
  // Si hay metadatos expirados y un proveedor configurado, se refrescan
  // automáticamente al abrir la biblioteca (M19.2).
  useEffect(() => {
    if (view !== 'library') return;
    const expiredCount = library.getExpiredMetadataCount();
    if (expiredCount > 0 && library.hasMetadataProviderConfigured()) {
      setStatus(`Refrescando ${expiredCount} metadatos expirados…`);
      void library
        .refreshExpiredMetadata()
        .then(({ refreshed, failed }) => {
          if (refreshed === 0 && failed === 0) {
            setStatus('No hay metadatos expirados para actualizar.');
          } else if (failed === 0) {
            setStatus(`Metadatos actualizados: ${refreshed}.`);
          } else {
            setStatus(`Metadatos actualizados: ${refreshed} correctos, ${failed} fallidos.`);
          }
        })
        .catch(() => {
          setStatus('No se pudieron refrescar los metadatos expirados.');
        });
    }
  }, [library, config.metadata?.cacheTtlHours, view]);

  // Obtención automática de metadatos para títulos sin metadatos (tipo Sonarr/Radarr).
  // Solo se activa si metadata.autoFetch está habilitado y hay un proveedor configurado.
  useEffect(() => {
    if (view !== 'library') return;
    if (!config.metadata?.autoFetch) return;
    if (!library.hasMetadataProviderConfigured()) return;

    setRefreshingMetadata(true);
    setStatus('Buscando metadatos para títulos sin ellos…');

    void library
      .fetchMissingMetadata()
      .then(({ fetched, failed }) => {
        if (fetched === 0 && failed === 0) {
          setStatus('Todos los títulos ya tienen metadatos.');
        } else if (failed === 0) {
          setStatus(`Metadatos obtenidos: ${fetched}.`);
        } else {
          setStatus(`Metadatos obtenidos: ${fetched} correctos, ${failed} fallidos.`);
        }
      })
      .catch(() => {
        setStatus('No se pudieron obtener los metadatos automáticos.');
      })
      .finally(() => {
        setRefreshingMetadata(false);
      });
  }, [library, config.metadata?.autoFetch, view]);

  const exitApp = (): void => {
    if (onQuit !== undefined) {
      onQuit();
      return;
    }
    exit();
  };

  const handleQuit = (): void => {
    // Dentro del buscador, «q» es texto; solo Ctrl+C cierra.
    if (view !== 'search') exitApp();
  };

  const handleSearch = (): void => {
    if (view === 'search') return;
    setView('search');
    setStatus(null);
  };

  const handleSettings = (): void => {
    if (view === 'search') return;
    setView('settings');
    setStatus(null);
  };

  const handleHelp = (): void => {
    if (view === 'search') return;
    setHelpVisible((visible) => !visible);
  };

  const handlePlay = (): void => {
    if (view !== 'library') return;

    if (details === null) {
      setStatus('Selecciona un título para reproducirlo.');
      return;
    }

    try {
      const file = selectMediaFile(details.files);
      // La TUI nunca espera al reproductor: el hijo se desacopla y sigue vivo
      // aunque el usuario cierre Ink justo después.
      const plan = createLaunchPlan({
        playerConfig: config.player,
        filePath: file.filePath,
        mode: 'detach',
      });
      const label = playbackLabel(details.media.title, file);
      setStatus(`Abriendo ${label} en ${plan.player}…`);

      void launchPlayer(plan)
        .then(() => {
          setStatus(`▶ ${label} · ${plan.player}`);
        })
        .catch((error: unknown) => {
          setStatus(formatError(error));
        });
    } catch (error) {
      setStatus(formatError(error));
    }
  };

  const handleNextSection = (): void => {
    // Desde la vista de ajustes, Tab vuelve a la biblioteca principal.
    if (view === 'settings') {
      setView('library');
      setStatus(null);
      return;
    }
    if (view !== 'library') return;
    const current = SECTION_ORDER.indexOf(section);
    const next = SECTION_ORDER[(current + 1) % SECTION_ORDER.length] ?? 'all';
    setSection(next);
    setStatus(null);
  };

  const handleRefreshMetadata = (): void => {
    if (view !== 'library' || selectedMedia === undefined) return;
    setRefreshingMetadata(true);
    setStatus('Buscando metadatos…');
    void library.getOrFetchMetadata(selectedMedia.id).then(
      () => {
        setStatus('Metadatos actualizados.');
        setRefreshingMetadata(false);
        setMetadataRefreshKey((k) => k + 1);
      },
      () => {
        setStatus('No se pudieron obtener metadatos.');
        setRefreshingMetadata(false);
      },
    );
  };

  const handleGotoTaggedMedia = (): void => {
    if (view !== 'library') return;
    const start = (selected + 1) % items.length;
    for (let i = 0; i < items.length; i++) {
      const idx = (start + i) % items.length;
      const media = items[idx];
      if (media === undefined) continue;
      const meta = library.getMetadata(media.id);
      if (meta) {
        setSelected(idx);
        setStatus(`Metadatos disponibles para "${media.title}".`);
        return;
      }
    }
    setStatus('No hay títulos con metadatos en esta sección.');
  };

  const handleRefreshAllMetadata = (): void => {
    if (view !== 'library') return;
    setRefreshingMetadata(true);
    setStatus('Refrescando metadatos expirados…');
    void library
      .refreshExpiredMetadata()
      .then(
        ({ refreshed, failed }) => {
          if (refreshed === 0 && failed === 0) {
            setStatus('No hay metadatos expirados para actualizar.');
          } else {
            setStatus(`Metadatos actualizados: ${refreshed} actualizados, ${failed} fallidos.`);
          }
        },
        () => {
          setStatus('No se pudieron refrescar los metadatos.');
        },
      )
      .finally(() => {
        setRefreshingMetadata(false);
      });
  };

  const handleToggleSelection = (): void => {
    if (view !== 'library') return;
    const media = items[safeSelected];
    if (media === undefined) return;
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(media.id)) next.delete(media.id);
      else next.add(media.id);
      return next;
    });
  };

  const handleToggleCompact = (): void => {
    setCompactMode((prev) => {
      const next = !prev;
      setStatus(next ? 'Vista compacta activada' : 'Vista normal activada');
      return next;
    });
  };

  /** Abre la vista de reasignación de atajos de teclado (T2). */
  const handleOpenKeybindings = useCallback((): void => {
    if (view !== 'library') return;
    setKeybindingsViewVisible(true);
  }, [view]);

  /** Guarda los atajos reasignados en el archivo de configuración. */
  const handleSaveKeybindings = useCallback(
    async (nextConfig: AppConfig): Promise<void> => {
      try {
        await saveConfig(nextConfig, resolvedConfigPath);
        setStatus('Atajos de teclado guardados.');
      } catch {
        setStatus('No se pudo guardar la configuración.');
      }
    },
    [resolvedConfigPath],
  );

  const handleFilterWatched = (): void => {
    if (view !== 'library') return;
    setWatchFilter((prev) => (prev === 'watched' ? undefined : 'watched'));
  };

  const handleFilterUnwatched = (): void => {
    if (view !== 'library') return;
    setWatchFilter((prev) => (prev === 'unwatched' ? undefined : 'unwatched'));
  };

  const handleClearSelection = (): void => {
    setSelectedIds(new Set());
    setStatus('Selección limpiada.');
  };

  const handleGoToDetails = (): void => {
    if (view !== 'library') return;
    setDetailsVisible((visible) => !visible);
    if (!detailsVisible && selectedMedia === undefined) {
      setStatus('Selecciona un título para ver sus detalles.');
    }
  };

  /** Abre la vista de detalles de reproducción (play-details) para el título seleccionado. */
  const handleOpenPlayDetails = (): void => {
    if (view !== 'library') return;
    if (selectedMedia === undefined) {
      setStatus('Selecciona un título para ver sus detalles de reproducción.');
      return;
    }
    setPlayDetailsMediaId(selectedMedia.id);
    setView('play-details');
    setStatus(null);
  };

  /** Reproduce desde la vista de detalles de reproducción usando el fichero seleccionado allí. */
  const handlePlayFromDetails = (input: {
    fileId: number;
    audioTrackIndex: number | null;
    subtitleTrackIndex: number | null;
  }): void => {
    if (view !== 'play-details') return;
    const media = library.find(String(playDetailsMediaId ?? 0));
    const file = media.files.find((f) => f.id === input.fileId);
    if (file === undefined) {
      setStatus('No se pudo encontrar el fichero seleccionado.');
      return;
    }
    try {
      const plan = createLaunchPlan({
        playerConfig: config.player,
        filePath: file.filePath,
        mode: 'detach',
        audioTrackIndex: input.audioTrackIndex,
        subtitleTrackIndex: input.subtitleTrackIndex,
      });
      const label = playbackLabel(media.media.title, file);
      setStatus(`Abriendo ${label} en ${plan.player}…`);
      void launchPlayer(plan)
        .then(() => {
          setStatus(`▶ ${label} · ${plan.player}`);
        })
        .catch((error: unknown) => {
          setStatus(formatError(error));
        });
    } catch (error) {
      setStatus(formatError(error));
    }
  };

  const handleEscape = (): void => {
    if (view === 'play-details') {
      setView('library');
      setPlayDetailsMediaId(null);
      setStatus(null);
      return;
    }
    if (view !== 'library') {
      setView('library');
      setStatus(null);
      return;
    }
    setStatus(null);
  };

  useKeymap(
    keybindings,
    {
      forceQuit: exitApp,
      quit: handleQuit,
      help: handleHelp,
      search: handleSearch,
      settings: handleSettings,
      play: handlePlay,
      enter: handleOpenPlayDetails,
      nextSection: handleNextSection,
      refresh: handleRefreshMetadata,
      refreshAll: handleRefreshAllMetadata,
      goToDetails: handleGoToDetails,
      gotoTaggedMedia: handleGotoTaggedMedia,
      up: () => {
        if (view !== 'settings') setSelected((index) => Math.max(0, index - 1));
      },
      down: () => {
        if (view === 'settings') return;
        setSelected((index) => Math.min(Math.max(0, items.length - 1), index + 1));
      },
      escape: handleEscape,
      /** Alternar selección del título actual (D3 - selección múltiple). */
      toggleSelection: handleToggleSelection,
      /** Activar/desactivar vista compacta (D2). */
      toggleCompact: handleToggleCompact,
      /** Filtrar títulos vistos (D1). */
      filterWatched: handleFilterWatched,
      /** Filtrar títulos no vistos (D1). */
      filterUnwatched: handleFilterUnwatched,
      /** Limpiar selección múltiple y filtros (D3). */
      clearSelection: handleClearSelection,
      /** Abrir vista de reasignación de atajos (T2). */
      configureKeybindings: handleOpenKeybindings,
    },
    !helpVisible && !keybindingsViewVisible && view !== 'settings' && view !== 'play-details',
  );

  // Habilitar modo ratón para soporte de clic (D4).
  // Los clics en el área de la lista alternan la selección del título bajo el cursor.
  useEffect(() => {
    process.stdout.write('\x1b[?1000h');
    return () => {
      process.stdout.write('\x1b[?1000l');
    };
  }, []);

  useInput((input, key) => {
    const mouseKey = key as { pointer?: string };
    if (mouseKey.pointer === 'left' && view === 'library' && !helpVisible) {
      handleToggleSelection();
    }
  });

  // La ayuda es modal: cualquier tecla de cierre evita que se dispare otra acción.
  useInput((input, key) => {
    if (!helpVisible) return;

    if (key.ctrl && input === 'c') {
      exitApp();
      return;
    }

    if (
      key.escape ||
      matchesKey(keybindings.help ?? '?', input, key) ||
      matchesKey(keybindings.quit ?? 'q', input, key)
    ) {
      setHelpVisible(false);
    }
  });

  // Teclas globales que funcionan desde la vista de detalles de reproducción
  // (play-details), donde el useKeymap principal está desactivado para no
  // entrar en conflicto con la navegación propia de esa vista.
  useInput((input, key) => {
    if (view !== 'play-details') return;
    if (helpVisible) return;

    if (key.ctrl && input === 'c') {
      exitApp();
      return;
    }

    if (key.escape || input === 'q') {
      // Volver a la biblioteca (PlayDetails también lo gestiona; esta es la
      // delegación desde App por si acaso llega hasta aquí).
      setView('library');
      setPlayDetailsMediaId(null);
      setStatus(null);
      return;
    }

    if (matchesKey(keybindings.search ?? '/', input, key)) {
      handleSearch();
      return;
    }

    if (matchesKey(keybindings.help ?? '?', input, key)) {
      handleHelp();
      return;
    }

    if (matchesKey(keybindings.settings ?? 's', input, key)) {
      handleSettings();
      return;
    }
  });

  return (
    <Box flexDirection="column" height={process.stdout.rows ?? 24}>
      <Header
        title={VIEW_TITLE[view]}
        {...(view === 'library' && section !== 'all' ? { subtitle: section } : {})}
        count={total}
      />
      <Box flexGrow={1} flexDirection="column">
        {helpVisible ? <HelpOverlay bindings={keybindings} /> : null}
        {!helpVisible && view === 'library' ? (
          <LibraryView
            items={items}
            selected={safeSelected}
            section={section}
            counts={counts}
            details={details}
            showDetails={detailsVisible}
            library={library}
            compact={compactMode}
            metadataRefreshKey={metadataRefreshKey}
            selectedIds={selectedIds}
            _onToggleSelection={(mediaId) => {
              setSelectedIds((prev) => {
                const next = new Set(prev);
                if (next.has(mediaId)) next.delete(mediaId);
                else next.add(mediaId);
                return next;
              });
            }}
          />
        ) : null}
        {!helpVisible && view === 'search' ? (
          <SearchView
            query={query}
            onQueryChange={(value) => {
              setQuery(value);
              setStatus(null);
            }}
            items={items}
            selected={safeSelected}
            details={details}
            showDetails={config.ui.showDetails}
            library={library}
            metadataRefreshKey={metadataRefreshKey}
          />
        ) : null}
        {!helpVisible && view === 'settings' ? (
          <SettingsView
            config={config}
            configPath={resolvedConfigPath}
            onSave={async () => {
              setStatus('Configuración guardada.');
              try {
                const updated = await loadConfig({
                  configPath: resolvedConfigPath,
                });
                setConfig(updated);
                if (onIndex) {
                  setStatus('Indexando biblioteca…');
                  try {
                    await onIndex(updated);
                    setStatus('Indexación completada.');
                    setDataVersion((v) => v + 1);
                  } catch {
                    setStatus('No se pudo indexar la biblioteca.');
                  }
                }
              } catch {
                setStatus('No se pudo recargar la configuración.');
              }
            }}
            onBack={() => setView('library')}
          />
        ) : null}
        {!helpVisible && view === 'play-details' ? (
          <PlayDetails
            details={playDetailsMediaId === null ? null : library.find(String(playDetailsMediaId))}
            library={library}
            onPlay={handlePlayFromDetails}
            onBack={() => {
              setView('library');
              setPlayDetailsMediaId(null);
              setStatus(null);
            }}
          />
        ) : null}
        {keybindingsViewVisible ? (
          <KeybindingsView
            config={config}
            onSave={handleSaveKeybindings}
            onBack={() => setKeybindingsViewVisible(false)}
          />
        ) : null}
      </Box>
      {error === null ? null : (
        <Box paddingX={1}>
          <Text>{theme.error(error)}</Text>
        </Box>
      )}
      <StatusBar bindings={keybindings} view={view} message={status} />
      {refreshingMetadata && (
        <Box paddingX={1}>
          <Text>{theme.muted('  actualizando metadatos…')}</Text>
        </Box>
      )}
    </Box>
  );
}

/** Etiqueta corta del fichero para la barra de estado. */
function playbackLabel(title: string, file: MediaFileRow): string {
  if (file.episode === null) return title;

  const season = file.season === null ? '' : `T${String(file.season).padStart(2, '0')}`;
  const episode = `E${String(file.episode).padStart(2, '0')}`;
  return `${title} · ${season}${episode}`;
}

/** Exportación nombrada para compatibilidad con ESM y verbatimModuleSyntax. */
export { AppImpl as App };
