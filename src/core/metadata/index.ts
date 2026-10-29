/**
 * Módulo de metadatos: obtención automática y transparente al estilo
 * Sonarr / Radarr.
 *
 * Exporta el contrato de proveedores, el gestor orquestador, los errores
 * tipados y el proveedor interno por defecto.
 */

export type {
  MediaKind,
  MetadataSearchResult,
  MetadataResult,
  MetadataEnrichment,
  MetadataProvider,
} from './provider.js';
export type { MetadataManager, MetadataManagerConfig } from './manager.js';
export { createMetadataManager } from './manager.js';
export {
  MetadataProviderError,
  MetadataNotFoundError,
  MetadataRateLimitError,
  MetadataNetworkError,
  MetadataConfigurationError,
} from './errors.js';
export { DefaultMetadataProvider } from './default-provider.js';
