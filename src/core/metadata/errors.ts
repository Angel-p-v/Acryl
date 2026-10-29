/**
 * Errores tipados para el sistema de metadatos.
 *
 * El proyecto prohíbe lanzar `throw new Error(...)` genérico. Estas clases
 * permiten distinguir el tipo de fallo y aplicar degradación graciosa.
 */

export class MetadataProviderError extends Error {
  readonly provider: string;
  readonly code: string;
  readonly original?: unknown;

  constructor(message: string, provider: string, code: string, original?: unknown) {
    super(message);
    this.name = 'MetadataProviderError';
    this.provider = provider;
    this.code = code;
    this.original = original;
  }
}

export class MetadataNotFoundError extends MetadataProviderError {
  constructor(provider: string, query: string, original?: unknown) {
    super(
      `No se encontraron metadatos para "${query}" en ${provider}.`,
      provider,
      'not_found',
      original,
    );
  }
}

export class MetadataRateLimitError extends MetadataProviderError {
  constructor(provider: string, retryAfterSeconds?: number, original?: unknown) {
    super(
      `Límite de peticiones excedido en ${provider}` +
        (retryAfterSeconds !== undefined ? ` (reintenta en ${retryAfterSeconds}s)` : ''),
      provider,
      'rate_limit',
      original,
    );
  }
}

export class MetadataNetworkError extends MetadataProviderError {
  constructor(provider: string, original?: unknown) {
    super(`Error de red al consultar ${provider}.`, provider, 'network_error', original);
  }
}

export class MetadataConfigurationError extends MetadataProviderError {
  constructor(provider: string, reason: string) {
    super(
      `Proveedor ${provider} no configurado correctamente: ${reason}.`,
      provider,
      'configuration_error',
    );
  }
}
