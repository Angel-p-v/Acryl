# Contribuir a Acryl

Gracias por querer mejorar el proyecto. Esta guía resume el flujo de trabajo y las
reglas que mantienen `main` siempre desplegable.

## Antes de empezar

- Node.js **22 o superior** y npm 11.
- Git y, opcionalmente, `mpv`/`vlc` para probar la reproducción real.
- Un sistema Linux/macOS; Windows funciona para desarrollo, aunque los scripts de
  empaquetado portable se prueban en Linux.

```bash
git clone https://github.com/Angel-p-v/app-media-manager.git
cd app-media-manager
npm install
npm run verify
```

`npm install` prepara Husky: el hook de pre-commit pasa ESLint y Prettier sobre los
ficheros staged; el de pre-push ejecuta `npm run verify`.

## Arquitectura

```
frontend/  (React + TypeScript, WebView de Tauri)
   │  servicios (invoke a comandos Tauri) ──▶  src-tauri/  (Rust)
   │                                             ├── core/        lógica de negocio
   │                                             ├── db/          rusqlite, migraciones, FTS5
   │                                             ├── player/      bindings a libmpv
   │                                             ├── commands/    comandos Tauri expuestos
   │                                             └── config/      serde + YAML
```

- `src-tauri/src/**` no importa desde `frontend/`.
- `frontend/` no escribe SQL: accede a datos mediante comandos Tauri (`invoke`).
- `src-tauri/src/db/` habla con SQLite; nadie más importa `rusqlite` fuera de `db/`.
- La conexión y los repositorios se inyectan; no hay singletons globales ni estado
  mutable global.

Si un cambio cruza estas reglas, probablemente necesite un ADR en `docs/adr/`. Las
decisiones con más de una opción válida se documentan ahí antes de implementarlas.

## Flujo de trabajo

1. Crea una rama desde `main`: `feat/…`, `fix/…`, `refactor/…`, `chore/…` o `docs/…`.
2. Haz commits pequeños con [Conventional Commits](https://www.conventionalcommits.org/):
   `feat(config): añade soporte para idioma inglés`.
3. Ejecuta `npm run verify` antes de abrir la PR.
4. Abre la PR contra `main` con una descripción de **qué** y **por qué**, y el checklist
   de verificación.
5. Espera a que la CI esté verde en Node 22, 24 y 26.
6. El merge es **squash**; la rama se elimina. Nunca se commitea directamente en `main`.

Scopes admitidos: `config`, `db`, `indexer`, `parser`, `player`, `ui`, `cli`, `deps`,
`ci`, `docs`, `test` y `release`.

## Tests

| Nivel       | Ubicación            | Objetivo                                              |
| ----------- | -------------------- | ----------------------------------------------------- |
| Unitario    | `src/**/__tests__/`  | lógica pura, parsers y configuración                  |
| Integración | `src/**/__tests__/`  | SQLite real, `fs` real y render de componentes        |
| E2E         | `frontend/tests/e2e/`| binario compilado: flujos críticos de la aplicación   |

```bash
npm test                 # suite completa de vitest
npm run test:e2e         # pruebas E2E de la aplicación integrada
```

Reglas:

- Se prueba comportamiento público, no detalles de implementación.
- Cada test es independiente: su propio entorno, su propia base de datos temporal.
- Nombres `describe > it` en Given/When/Then.

## Estilo

- Código e identificadores en inglés; documentación de usuario en español.
- TypeScript `strict`, sin `any` ni `as` injustificados.
- Comenta **por qué**, no lo obvio.
- Prettier y ESLint son la fuente de verdad; no discutas el formato en la review.
- No añadas dependencias sin justificar el coste y pasar por el ADR si afecta a la
  arquitectura.

Si añades una dependencia con script de instalación, apruébala con
`npm install-scripts approve <paquete>` y commitea el cambio en `package.json`
(`allowScripts`). Si no, el CI fallará.

## Documentación

- `README.md`: instalación y uso.
- `docs/README.md`: índice de la documentación.
- `docs/guides/architecture.md`: capas y flujos.
- `docs/guides/performance.md`: benchmarks y presupuesto.
- `docs/guides/release.md`: publicación y credenciales.
- `docs/adr/`: decisiones de arquitectura.
- `PLAN_DESKTOP.md`: plan de migración desktop y estado de hitos.

Al cerrar un hito, actualiza las tablas de estado de `PLAN_DESKTOP.md`, `README.md` y `AGENTS.md`
en el mismo PR.

## Publicación

El binario es `acryl` y la distribución oficial son las GitHub Releases: bundles portables,
checksums SHA-256, SBOM y atestaciones de procedencia. No se publica en npm. Los pasos
completos están en [`docs/guides/release.md`](./docs/guides/release.md); no muevas un tag
ni subas assets manualmente desde una rama de desarrollo.