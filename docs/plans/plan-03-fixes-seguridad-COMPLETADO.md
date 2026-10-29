# Plan: Corrección de errores y vulnerabilidades (fixes/security) — ✅ COMPLETADO

> **Estado:** todos los ítems resueltos y mergeados en `main` (PR #142).
> **Índice de planes:** [`docs/plans/README.md`](./README.md)

**Objetivo:** Estabilizar la release v3.0.0, eliminar fallos intermitentes y cerrar vectores de vulnerabilidad antes de avanzar con nuevos hitos.

## Contexto

La release v3.0.0 ya está publicada y el workflow de release-artifacts está verde, pero durante su estabilización se detectaron varios fallos en el instalador de Windows y un test E2E flaky que deben corregirse formalmente. Además, conviene auditar dependencias y superficie de ataque antes de seguir incorporando funcionalidad.

---

## 1. Errores conocidos (bugs) pendientes de corrección formal

| #   | Área                   | Descripción                                                                                                                                                                                                                                                                                                                      | Prioridad |
| --- | ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------- |
| B1  | install-test Windows   | Varios fallos en cadena del install-test de Windows (install.json, rutas absolutas, checksum case, switch syntax, `System.IO.Compression.ZipFile`, `Test-Path -Container`, marcador en wrapper `mm.bat`). Aunque el workflow ya pasa, los fixes van como commits sueltos sin un ADR que documente por qué se tomó cada decisión. | Alta      |
| B2  | Test E2E flaky         | `tests/e2e/cli.e2e.test.ts` falla intermitentemente con `AssertionError` en `index` (exit code 1 en lugar de 0). No está claro si es race condition en la compilación del binario o problema ambiental.                                                                                                                          | Alta      |
| B3  | Instalador Linux/macOS | Revisar `install.sh` / `uninstall.sh` con `shellcheck` y pruebas de desinstalación en escenarios edge (prefix con espacios, versión ya instalada sin `--force`).                                                                                                                                                                 | Media     |
| B4  | Cierre de SQLite       | Verificar que `closeDatabase()` nunca tape el error original y que `PRAGMA optimize` no falle silenciosamente en entornos restringidos.                                                                                                                                                                                          | Media     |

---

## 2. Auditoría de vulnerabilidades y superficie de ataque

| #   | Área                | Acción                                                                                                                                                                                                           | Prioridad |
| --- | ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------- |
| V1  | Dependencias        | Ejecutar `npm audit` y revisar dependencias con CVEs conocidas (especialmente `better-sqlite3`, `ink`, `commander`, `zod`). Aprobar scripts de instalación con `npm install-scripts approve` si se añade alguna. | Alta      |
| V2  | Firmado de commits  | Verificar que todos los commits en `main` tengan firma GPG (verificación de commit). Configurar `gpgsign` si falta.                                                                                              | Media     |
| V3  | Rutas en installers | Auditar que ni `install.sh`, `install.ps1` ni `uninstall.ps1` usen `eval`, `sudo` ni rutas bajo `/proc` (según gotchas del AGENTS.md).                                                                           | Media     |
| V4  | Metadatos externos  | Validar que las claves API de TMDB/TVDB no se compleen en el código ni en el historial de git; usar variables de entorno (`MM_TMDB_API_KEY`, `MM_TVDB_API_KEY`).                                                 | Media     |

---

## 3. Criterios de aceptación (Definition of Done)

- [ ] `npm run verify` pasa en Node 22, 24 y 26 sin fallos flaky.
- [ ] `npm test` (suite completa) pasa de forma consistente (≥3 runs sin fallos intermitentes).
- [ ] `npm run coverage` cumple umbrales: statements 85%, functions 85%, branches 80%, lines 85%.
- [ ] `npm run smoke` (tarball → instalación limpia → CLI) pasa.
- [ ] `npm audit` sin vulnerabilidades de severidad alta o crítica sin aprobar.
- [ ] release-artifacts workflow verde en Linux x64/arm64, Windows x64, SBOM, install-test Linux y Windows.

---

## 4. Estrategia de commits

Cada corrección va como un commit atómico separado con scope del proyecto (`ci`, `db`, `test`, `deps`, etc.), siguiendo Conventional Commits y la política de commits pequeños (5–10 archivos o 200–300 líneas por commit). Los cambios de documentación y dependencias van en commits aislados.

---

## Siguiente paso

Una vez este plan esté en verde, se procede al plan de hitos y mejoras (`hitos-mejoras-m18-m22.md`).
