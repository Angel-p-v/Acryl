# Plan: Hitos futuros, mejoras y cobertura (M18–M22 y más allá) — ✅ COMPLETADO

> **Estado:** todos los hitos M18–M22 completados y mergeados en `main`.
> **Índice de planes:** [`docs/plans/README.md`](./README.md)

**Objetivo:** Continuar el roadmap del proyecto, completar los hitos pendientes (M18–M22), cubrir nuevas funcionalidades y mejorar las existentes, manteniendo `main` desplegable y `npm run verify` verde.

## Contexto

Según `PLAN.md` §7 y el estado actual, todos los hitos M0–M22 están completados.

---

## 1. Hitos pendientes (M18–M22)

### M18 — TUI con metadatos visuales (`2.2.0`)

| Item  | Descripción                                                                                             |
| ----- | ------------------------------------------------------------------------------------------------------- |
| M18.1 | Panel de detalles en TUI muestra metadatos (título, año, sinopsis, calificación, fecha estreno, idioma) |
| M18.2 | Mostrar información del proveedor (TMDB/TVDB) junto a los metadatos                                     |
| M18.3 | Si no hay metadatos, mostrar opción para obtenerlos desde la TUI                                        |
| M18.4 | Tecla `r` en detalles para refrescar metadatos del título actual                                        |
| M18.5 | Tecla `m` en listado para navegar a detalles con metadatos si existen                                   |

### M19 — Sincronización en segundo plano (watch metadatos) (`2.2.0`)

| Item  | Descripción                                                                       |
| ----- | --------------------------------------------------------------------------------- |
| M19.1 | Watch de metadatos al abrir TUI: verificar qué títulos tienen metadatos expirados |
| M19.2 | Refrescar metadatos expirados al iniciar (configurable)                           |
| M19.3 | Manejo elegante de errores de red                                                 |
| M19.4 | Rate-limiting respetado (TMDB/TVDB)                                               |

### M20 — Auto-completado de shell (`2.2.0`)

| Item  | Descripción                                      |
| ----- | ------------------------------------------------ |
| M20.1 | Comando `mm completion bash`                     |
| M20.2 | Comando `mm completion zsh`                      |
| M20.3 | Comando `mm completion fish`                     |
| M20.4 | Documentación de instalación por shell en README |

### M21 — E2E interactivos con TTY real (`2.2.0`)

| Item  | Descripción                                     |
| ----- | ----------------------------------------------- |
| M21.1 | Integrar `node-pty` para tests E2E con TTY real |
| M21.2 | Test de navegación por TUI con teclas reales    |
| M21.3 | Test de selección y reproducción desde TUI      |

### M22 — Mejorar cobertura de pruebas (`2.2.0`)

| Item  | Descripción                                                      |
| ----- | ---------------------------------------------------------------- |
| M22.1 | Tests para `launcher.ts`, `watcher.ts`, `watch.ts`, `service.ts` |
| M22.2 | Cobertura global ≥ 85% (statements, functions, branches, lines)  |

---

## 2. Mejoras sugeridas (más allá de M22)

| #   | Área        | Mejora                                                                  | Justificación                      |
| --- | ----------- | ----------------------------------------------------------------------- | ---------------------------------- |
| I1  | Reproductor | Soporte para múltiples reproductores simultáneos o cola de reproducción | UX mejorada                        |
| I2  | TUI         | Temas personalizables y modo oscuro/claro                               | Accesibilidad                      |
| I3  | Indexación  | Índice incremental por carpeta (no solo reindexado completo)            | Rendimiento en bibliotecas grandes |
| I4  | Metadatos   | Caché local de imágenes/pósteres descargadas                            | Uso offline y menor consumo de API |
| I5  | CLI         | Exportar biblioteca a JSON/CSV                                          | Portabilidad y backup              |
| I6  | Config      | Validación de rutas de biblioteca al arrancar (advertir si no existen)  | UX y menos errores silenciosos     |
| I7  | Assets      | Assets macOS x64/arm64 en release (cuando haya runners disponibles)     | Cobertura de plataformas           |

---

## 3. Criterios de aceptación por hito

Cada hito M18–M22 se considera completo cuando:

- [ ] Código implementado en inglés (identificadores, comentarios técnicos)
- [ ] Documentación de usuario en español (README, docs si aplica)
- [ ] Cada función nueva tiene su test; los bugs curiosos/documentados incluyen test
- [ ] Nada de `any`, nada de `as` sin justificación, nada de estados globales
- [ ] Se respetó la regla de arquitectura: `core/` nunca importa de `ui/` ni `commands/`
- [ ] `npm run verify` pasa (lint + format + typecheck + test)
- [ ] Cobertura contribuye a mantener umbrales globales (≥85/85/80/85)
- [ ] Commit atómico por intención revisable (política de commits pequeños)
- [ ] PR a `main` con merge squash, CI verde, y rama borrada

---

## 4. Estrategia de trabajo

1. No se pasa al siguiente hito hasta que la PR del actual esté mergeada y en `main`.
2. Cada hito se ejecuta en su propia rama (`feat/m18-tui-metadata`, etc.).
3. Antes de empezar un hito nuevo, revisar que el plan de fixes (`fixes-seguridad-COMPLETADO.md`) no tenga ítems críticos pendientes.
4. Los cambios masivos (formato, movimientos, dependencias) van en commits separados.
5. Al finalizar cada hito: actualizar tabla de estado en `PLAN.md` y `README.md`.

---

## 5. Métricas de éxito

- Cobertura global: statements ≥85%, functions ≥85%, branches ≥80%, lines ≥85%
- `npm run verify` verde en cada push a `main`
- release-artifacts workflow verde para cada release
- Cada hito M18–M22 documentado en `PLAN.md` y reflejado en `README.md`
