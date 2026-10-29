# Índice de planes de desarrollo

Este directorio contiene los planes de ejecución del proyecto `media-manager`. Cada plan documenta alcance, tareas, criterios de aceptación y estado.

## Planes activos

| Plan | Estado | Descripción |
| ---- | ------ | ----------- |

## Planes completados

| Plan                                                                                               | Estado        | Descripción                                                                                                                           | PR de cierre |
| -------------------------------------------------------------------------------------------------- | ------------- | ------------------------------------------------------------------------------------------------------------------------------------- | ------------ |
| [`plan-01-mejoras-continuas-fase2-COMPLETADO.md`](./plan-01-mejoras-continuas-fase2-COMPLETADO.md) | ✅ Completado | Mejoras continuas post-M22 (Fase 2): pruebas, indexador, metadatos, TUI, CLI, doctor, empaquetado, i18n, seguridad.                   | —            |
| [`plan-02-fixes-code-review-COMPLETADO.md`](./plan-02-fixes-code-review-COMPLETADO.md)             | ✅ Completado | Corrección de errores del code review: bug `find` en install.sh, `countFavorites`, SQL injection en mediaRepo, tests de filtros, etc. | —            |
| [`plan-03-fixes-seguridad-COMPLETADO.md`](./plan-03-fixes-seguridad-COMPLETADO.md)                 | ✅ Completado | Corrección de errores y vulnerabilidades (bugs B1-B4, vulnerabilidades V1-V4).                                                        | #142         |
| [`plan-04-hitos-m18-m22-COMPLETADO.md`](./plan-04-hitos-m18-m22-COMPLETADO.md)                     | ✅ Completado | Hitos M18–M22: TUI metadatos visuales, sync en segundo plano, autocompletado shell, E2E TTY, cobertura.                               | —            |
| [`plan-05-mejoras-post-m22-COMPLETADO.md`](./plan-05-mejoras-post-m22-COMPLETADO.md)               | ✅ Completado | 10 ítems post-M22 todos completados y mergeados.                                                                                      | #114–#121    |
| [`plan-06-version-2-0-COMPLETADO.md`](./plan-06-version-2-0-COMPLETADO.md)                         | ✅ Completado | Plan v2.0: estado de reproducción, continuar viendo, biblioteca sana, soporte macOS.                                                  | —            |
| [`plan-07-instaladores-seguridad-COMPLETADO.md`](./plan-07-instaladores-seguridad-COMPLETADO.md)   | ✅ Completado | Instaladores POSIX, distribución por GitHub Releases, checksums, SBOM, atestaciones.                                                  | —            |
| [`plan-08-distribucion-sin-npm-SUSTITUIDO.md`](./plan-08-distribucion-sin-npm-SUSTITUIDO.md)       | 🔄 Sustituido | Sustituido por `plan-07-instaladores-seguridad-COMPLETADO.md`.                                                                        | —            |
| [`plan-09-nuevas-mejoras-futuras-COMPLETADO.md`](./plan-09-nuevas-mejoras-futuras-COMPLETADO.md)   | ✅ Completado | Nuevas mejoras futuras: TUI, indexador, metadatos, reproductor, distribución, testing, documentación y comunidad.                     | #171–#180    |
| [`plan-10-configuracion-desde-COMPLETADO.md`](./plan-10-configuracion-desde-COMPLETADO.md)         | ✅ Completado | Edición de configuración desde la TUI: opción `--set` en `mm config` para modificar claves anidadas sin editar el YAML a mano.        | #198         |
| [`plan-11-ajustes-settings-COMPLETADO.md`](./plan-11-ajustes-settings-COMPLETADO.md)               | ✅ Completado | Corrección de errores de ajustes (settings): etiqueta reproductor, guardar sin archivo YAML previo, `mm config --path` advierte.      | #197         |

## Leyenda de estado

- 🟡 **Activo**: plan en ejecución, ítems pendientes por implementar.
- 🔴 **Pendiente**: plan aprobado pero no iniciado.
- ✅ **Completado**: todos los ítems implementados, verificados y mergeados en `main`.
- 🔄 **Sustituido**: plan reemplazado por otro más actual.

## Flujo de trabajo obligatorio (AGENTS.md)

1. **Una rama por ítem o hito**: `feat/…`, `fix/…`, `test/…`, `ci/…`, `docs/…`.
2. **Commits con Conventional Commits y scope del proyecto** (`config`, `db`, `indexer`, `parser`, `player`, `ui`, `cli`, `deps`, `ci`, `docs`, `test`, `release`).
3. **`npm run verify` debe pasar antes de cada push** (pre-push lo comprueba; pre-commit ejecuta lint-staged y typecheck).
4. **`git push -u origin <rama>` y PR a `main`** con `gh pr create`.
5. Revisar que la CI está en verde y **mergear con squash**: `gh pr merge <n> --squash --delete-branch`.
6. `main` queda siempre desplegable: cada commit compila, pasa lint y tiene tests.
