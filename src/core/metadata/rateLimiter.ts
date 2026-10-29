/**
 * Rate limiter simple por proveedor de metadatos.
 *
 * Registra los timestamps de las llamadas realizadas y asegura que no se
 * exceda el número máximo de peticiones dentro de una ventana de tiempo
 * deslizante. TVMaze no tiene límite oficial publicado pero usamos un
 * límite conservador; Jikan permite 30 peticiones por segundo para IP no
 * autenticada (usamos 2 por segundo para ser conservadores).
 *
 * Antes de cada llamada a la API, el llamante invoca `waitForSlot()` que
 * espera (si es necesario) hasta que haya cupo disponible.
 */
export class RateLimiter {
  private readonly maxRequests: number;
  private readonly windowMs: number;
  private timestamps: number[] = [];

  constructor(maxRequests: number, windowMs: number) {
    this.maxRequests = maxRequests;
    this.windowMs = windowMs;
  }

  /**
   * Espera si es necesario para respetar el límite de tasa, luego registra
   * la petición y resuelve. Si ya hay cupo, resuelve inmediatamente.
   */
  async waitForSlot(): Promise<void> {
    const now = Date.now();

    // Eliminar timestamps que ya salieron de la ventana deslizante
    this.timestamps = this.timestamps.filter((t) => now - t < this.windowMs);

    if (this.timestamps.length < this.maxRequests) {
      this.timestamps.push(now);
      return;
    }

    // Esperar hasta que el registro más antiguo salga de la ventana
    const oldest = this.timestamps[0];
    if (oldest === undefined) return;
    const waitTime = Math.max(0, this.windowMs - (now - oldest));
    await new Promise<void>((resolve) => setTimeout(resolve, waitTime));
    return this.waitForSlot();
  }
}

/** Límites de tasa por proveedor público (peticiones por ventana de tiempo). */
export const rateLimiters: Record<'tvmaze' | 'jikan', RateLimiter> = {
  // TVMaze: no hay límite oficial publicado, usamos 5 por segundo como conservador.
  tvmaze: new RateLimiter(5, 1_000),
  // Jikan: 30 req/sec para IP no autenticada, usamos 2 por segundo para ser conservadores.
  jikan: new RateLimiter(2, 1_000),
};
