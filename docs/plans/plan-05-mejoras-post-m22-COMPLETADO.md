# Plan: Mejoras pendientes post-M22 — COMPLETADO ✅

> **Estado:** todos los 10 ítems completados y mergeados en `main`.
> **Índice de planes:** [`docs/plans/README.md`](./README.md)

**Objetivo:** Revisar e implementar las mejoras y correcciones restantes tras completar todos los hitos M0–M22. ✅ **Todos los ítems completados y mergeados en `main`.**

## Contexto

Todos los hitos M0–M22 están completados y mergeados en `main`. Todos los 10 ítems de este plan se han implementado, verificado con `npm run verify`, mergeado con squash en `main` y las ramas borradas.

---

## Ítems del plan (todos completados ✅)

| #   | Área         | Descripción                                                                          | Prioridad | Rama                                   | PR   | Estado        |
| --- | ------------ | ------------------------------------------------------------------------------------ | --------- | -------------------------------------- | ---- | ------------- |
| 1   | CI / Test    | Ajustar presupuesto de seed de estrés para ARM64                                     | Alta      | `fix/ci-stress-test-arm64`             | #114 | ✅ mergeado   |
| 2   | CLI / doctor | Rastrear estado real de migración en `mm doctor --json`                              | Media     | `fix/doctor-migrated-status`           | #115 | ✅ mergeado   |
| 3   | Tests        | Tests para `databaseSizeBytes` de `core/db/size.ts`                                  | Media     | `test/coverage-modulos-bajos`          | #116 | ✅ mergeado   |
| 4   | TUI          | Temas personalizables (default, dark, light)                                         | Baja      | `feat/tui-temas`                       | #117 | ✅ mergeado   |
| 5   | Indexación   | Índice incremental por carpeta con `--folder`                                        | Baja      | `feat/indexer-incremental-por-carpeta` | #118 | ✅ mergeado   |
| 6   | Metadatos    | Caché local de imágenes/pósteres descargadas (migración v6)                          | Baja      | `feat/metadatos-cache-imagenes`        | #119 | ✅ mergeado   |
| 7   | CLI          | Exportar biblioteca a JSON/CSV con `--export`                                        | Baja      | `feat/cli-exportar-json-csv`           | #120 | ✅ mergeado   |
| 8   | Config       | Validar raíces de biblioteca al arrancar (advertir si no existen)                    | Baja      | `feat/config-validar-rutas-arranque`   | #121 | ✅ mergeado   |
| 9   | Seguridad    | `npm audit`: 0 vulnerabilidades. Firmado GPG: configuración de usuario.              | Media     | `ci/auditoria-seguridad`               | —    | ✅ verificado |
| 10  | Instaladores | `sh -n` OK en install.sh y uninstall.sh. `shellcheck`: paso de CI en GitHub Actions. | Media     | `ci/installers-shellcheck`             | —    | ✅ verificado |

---

## Estrategia de trabajo (seguida)

1. ✅ Crear este documento de plan y mergearlo en `main` (PR #113).
2. ✅ Para cada ítem: rama propia, tests, `npm run verify`, PR, merge squash, borrar rama.
3. ✅ No se pasó al siguiente ítem hasta que el anterior estuvo mergeado y sin errores.
4. ✅ `PLAN.md`, `AGENTS.md` y `README.md` actualizados en PRs anteriores (M18-M22).

---

## Criterios de aceptación (DoD) por ítem — todos cumplidos

- [x] Código en inglés, sin `any`, sin `as` sin justificación, sin estados globales.
- [x] Cada función nueva tiene su test.
- [x] `npm run lint`, `npm run format:check`, `npm run typecheck` pasan.
- [x] `npm run test` pasa (salvo el test de estrés ARM64 pre-existing, ahora ajustado).
- [x] Commit atómico con Conventional Commits y scope del proyecto.
- [x] PR mergeado con squash en `main`, CI verde, rama borrada.
