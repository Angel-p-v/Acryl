# Rendimiento

Este documento fija el presupuesto de rendimiento de `media-manager` y explica cómo
medirlo. Los benchmarks no son una promesa de velocidad absoluta: sirven para detectar
regresiones de orden de magnitud en las rutas críticas.

## Cómo ejecutar

```bash
npm run bench    # escaneo, indexado completo y consultas
npm test         # incluye el test de estrés de 100 000 entradas
```

`npm run bench` ejecuta la variante `unit (bench)` que Vitest crea a partir de
`benchmark.include` y muestra una tabla por benchmark con `hz`, media, percentiles y
error relativo. Los benchmarks no bloquean `npm run verify`; el test de estrés sí forma
parte de la suite de integración.

## Escenarios medidos

| Benchmark                | Escenario                                                                         |
| ------------------------ | --------------------------------------------------------------------------------- |
| Escaneo                  | 10 000 ficheros `.mkv` vacíos en 100 carpetas de series                           |
| Indexado completo        | Los mismos 10 000 ficheros sobre una base SQLite en memoria nueva en cada muestra |
| Búsqueda FTS             | `search("99999")` sobre 100 000 títulos                                           |
| Listado                  | Primera página de 50 títulos ordenada por título sobre 100 000 títulos            |
| Estrés (gate automático) | Siembra de 100 000 títulos + búsqueda + listado en `npm test`                     |

El escaneo y el indexado incluyen trabajo de E/S real. La búsqueda y el listado usan una
base en memoria para aislar el coste de SQLite y FTS5.

## Presupuesto

Medición de referencia en un Intel Core i5-12400F (12 hilos), Linux y Node.js 26; una
ejecución concreta puede variar por caché de disco y carga de la máquina.

| Escenario                   | Referencia (media) | Umbral de regresión | Cómo se controla            |
| --------------------------- | -----------------: | ------------------: | --------------------------- |
| Escaneo de 10 000 ficheros  |              36 ms |      media ≤ 250 ms | `npm run bench` (revisión)  |
| Indexado de 10 000 ficheros |             136 ms |      media ≤ 750 ms | `npm run bench` (revisión)  |
| Búsqueda FTS sobre 100 000  |            0,04 ms |         p99 ≤ 25 ms | `npm run bench` (revisión)  |
| Listado de 50 sobre 100 000 |            0,04 ms |         p99 ≤ 25 ms | `npm run bench` (revisión)  |
| Siembra de 100 000 títulos  |              < 2 s |              ≤ 15 s | Test de estrés (automático) |
| Búsqueda/listado de estrés  |             < 2 ms |            ≤ 500 ms | Test de estrés (automático) |

Los umbrales son deliberadamente holgados respecto a la referencia: deben cazar una
regresión real (por ejemplo, perder un índice o ejecutar una consulta N+1), no el ruido
normal de una máquina compartida. Si un benchmark supera su umbral, se investiga antes de
mergear el cambio.

El test de estrés vive en
[`tests/integration/performance/stress.integration.test.ts`](https://github.com/Angel-p-v/media-manager/blob/main/tests/integration/performance/stress.integration.test.ts)
y falla la suite si la siembra supera 15 s o cualquiera de las consultas supera 500 ms.

## Metodología

- Cada benchmark se ejecuta con presupuesto de tiempo y mínimo de iteraciones; el setup
  (crear 10 000 ficheros o sembrar 100 000 filas) queda fuera de la medición.
- La base de datos del indexado se crea nueva en cada muestra y se cierra al terminar,
  para que un índice en caché no falsee el resultado.
- La siembra de búsqueda se hace en una única transacción; los triggers de FTS5 se
  ejecutan igual que en una indexación real.
- Los benchmarks se ejecutan en un único worker (`maxWorkers: 1`) para evitar que varios
  escenarios compitan por CPU y disco.
- Antes de comparar dos ramas, usa la misma máquina, el mismo Node y evita procesos
  pesados en segundo plano.

## Mantenimiento de la base de datos

La aplicación ejecuta `PRAGMA optimize` al cerrar la conexión. Para compactar de verdad
un índice que ha sufrido muchos borrados:

```bash
mm db optimize --vacuum
```

`VACUUM` reescribe el fichero completo y requiere espacio temporal; no se lanza de forma
automática. La decisión está registrada en el
[ADR 0007](../adr/0007-logging-errores-y-mantenimiento.md).

## Qué revisar en una regresión

1. Repite `npm run bench` con la máquina en reposo.
2. Comprueba que la migración v2 está aplicada (`mm db stats` y las migraciones de
   `src/core/db/migrations/index.ts`).
3. Revisa el plan de consulta si cambió `mediaRepo` (`EXPLAIN QUERY PLAN` sobre la
   consulta afectada).
4. Si el cambio es en el escáner, descarta un aumento de `stat()` o de ficheros
   recorridos antes de tocar los índices.
