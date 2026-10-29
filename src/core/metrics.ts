/**
 * Métricas simples de la aplicación: contadores de indexaciones, errores de
 * metadatos y tiempo medio de consulta. Se exponen para que la CLI/TUI puedan
 * mostrarlas sin depender de un sistema de observabilidad externo.
 */
export type MetricsSnapshot = {
  /** Número de indexaciones realizadas. */
  indexations: number;
  /** Número de errores al obtener metadatos externos. */
  metadataErrors: number;
  /** Tiempo acumulado de consultas en milisegundos. */
  totalQueryTimeMs: number;
  /** Número de consultas registradas. */
  queryCount: number;
  /** Tiempo medio de consulta en milisegundos. */
  averageQueryTimeMs: number;
};

/** Acumulador de métricas en memoria. No persiste entre reinicios. */
export class Metrics {
  private indexations = 0;
  private metadataErrors = 0;
  private totalQueryTimeMs = 0;
  private queryCount = 0;

  /** Registra una indexación completada. */
  recordIndexation(): void {
    this.indexations += 1;
  }

  /** Registra un error al obtener metadatos externos (TMDB/TVDB/OMDB). */
  recordMetadataError(): void {
    this.metadataErrors += 1;
  }

  /** Registra el tiempo de una consulta a la base de datos o API externa. */
  recordQueryTime(ms: number): void {
    if (ms < 0) return;
    this.totalQueryTimeMs += ms;
    this.queryCount += 1;
  }

  /** Devuelve una instantánea de las métricas actuales. */
  snapshot(): MetricsSnapshot {
    return {
      indexations: this.indexations,
      metadataErrors: this.metadataErrors,
      totalQueryTimeMs: this.totalQueryTimeMs,
      queryCount: this.queryCount,
      averageQueryTimeMs: this.queryCount > 0 ? this.totalQueryTimeMs / this.queryCount : 0,
    };
  }

  /** Reinicia todos los contadores a cero. */
  reset(): void {
    this.indexations = 0;
    this.metadataErrors = 0;
    this.totalQueryTimeMs = 0;
    this.queryCount = 0;
  }
}

/** Instancia compartida de métricas para uso en toda la aplicación. */
export const metrics = new Metrics();
