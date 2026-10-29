/**
 * Utilidad HTTP con timeout.
 *
 * Envuelve `fetch` con un tiempo de espera para evitar que una petición
 * cuelgue indefinidamente si un proveedor externo no responde.
 */

export type FetchOptions = {
  /** Tiempo máximo de espera en milisegundos. Por defecto 10 000 ms. */
  timeoutMs?: number;
  /** Cabeceras HTTP adicionales. */
  headers?: Record<string, string>;
};

/** Ejecuta fetch con timeout. Lanza si se excede el tiempo o hay error de red. */
export async function fetchWithTimeout(
  url: string | URL,
  options: RequestInit & FetchOptions = {},
): Promise<Response> {
  const { timeoutMs = 10_000, headers, ...fetchOptions } = options;

  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeoutMs);

  try {
    const response = await fetch(url, {
      ...fetchOptions,
      signal: controller.signal,
      headers: {
        Accept: 'application/json',
        ...headers,
      },
    });
    return response;
  } finally {
    clearTimeout(timeoutId);
  }
}
