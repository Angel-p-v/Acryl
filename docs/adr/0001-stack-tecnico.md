# ADR 0001 — Stack técnico

- **Estado:** aceptada
- **Fecha:** 2026-10-22

## Contexto

El proyecto nace como una reorientación de una aplicación web (monorepo) hacia una
herramienta 100 % terminal. Necesitamos un runtime, un framework de interfaz, un
lenguaje con tipos, una base de datos local y un sistema de build que funcione en
Linux, macOS y Windows sin fricción para quien quiera instalarlo desde npm.

## Decisión

| Capa           | Elección                              | Versión    |
| -------------- | ------------------------------------- | ---------- |
| Runtime        | Node.js                               | ≥ 22       |
| Lenguaje       | TypeScript en modo `strict`           | `~6.0.3`   |
| Interfaz       | Ink + React                           | 8.x / 19.x |
| Widgets        | `@inkjs/ui`                           | 2.x        |
| CLI            | Commander                             | 15.x       |
| Configuración  | YAML + Zod                            | 2.x / 4.x  |
| Persistencia   | better-sqlite3                        | 13.x       |
| Tests          | Vitest                                | 5.x        |
| Lint / formato | ESLint 10 (type-aware) + Prettier 3.9 | —          |

## Alternativas consideradas

- **Bun**: arranque más rápido, pero exige instalar un runtime extra y el ecosistema de
  Ink está pensado para Node. Descartado por reproducibilidad.
- **OpenTUI**: ofrece un dashboard más cercano a `btop`, pero su API (0.6.x) todavía
  cambia con frecuencia. Descartado como dependencia crítica; la capa `ui/` queda
  aislada para poder reintroducirlo.
- **Biome en lugar de ESLint**: más rápido y sin dependencias de Node, pero pierde el
  análisis de tipos. Se mantiene ESLint con reglas _type-aware_.
- **Deno**: buen soporte de TypeScript, pero menor ecosistema CLI/TUI.
- **TypeScript 7**: su compilador nativo es mucho más rápido, pero `typescript-eslint`
  declara `typescript: ">=4.8.4 <6.1.0"`, por lo que todavía no se puede usar con
  lint _type-aware_. Se fija `~6.0.3` y se revisará cuando se libere el soporte.

## Consecuencias

- El proyecto se instala con `npm i -g media-manager` y funciona con Node ya presente.
- `better-sqlite3` es una dependencia nativa, pero publica binarios precompilados para
  las plataformas habituales, así que no hace falta toolchain de compilación.
- Fijar `~6.0.3` implica no poder adoptar TS7 hasta que el linter lo soporte.
