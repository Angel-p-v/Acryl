import type { MediaType } from '../core/config/schema.js';

/** Sección activa del navegador. */
export type Section = 'all' | MediaType | 'in-progress' | 'favorites';

/** Vistas de la interfaz. */
export type ViewName = 'library' | 'search' | 'settings' | 'play-details';

/** Número de títulos por sección, para la barra lateral. */
export type Counts = Record<Section, number>;
