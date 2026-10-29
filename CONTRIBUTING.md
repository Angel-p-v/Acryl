# Contribuir a media-manager

Gracias por querer mejorar el proyecto. Esta guía resume el flujo de trabajo y las
reglas que mantienen `main` siempre desplegable.

## Antes de empezar

- Node.js **22 o superior** y npm 11.
- Git y, opcionalmente, `mpv`/`vlc` para probar la reproducción real.
- Un sistema Linux/macOS; Windows funciona para desarrollo, aunque los scripts de
  empaquetado portable se prueban en Linux.

```bash
git clone https://github.com/Angel-p-v/media-manager.git
cd media-manager
npm install
npm run verify
```

`npm install` prepara Husky: el hook de pre-commit pasa ESLint y Prettier sobre los
ficheros staged; el de pre-push ejecuta `npm run verify`.

## Arquitectura

```
ui/  commands/  ──▶  core/  ──▶  shared/
                     └──▶ core/db/
```

- `core/**` no importa de `ui/**` ni de `commands/**`.
- `ui/**` y `commands/**` no escriben SQL; usan repositorios.
- Solo `core/db/` importa `better-sqlite3`.
- `shared/**` no depende del resto del proyecto.
- La conexión y los repositorios se inyectan; no hay singletons globales.

Si un cambio cruza estas reglas, probablemente necesite un ADR en `docs/adr/`. Las
decisiones con más de una opción válida se documentan ahí antes de implementarlas.

## Flujo de trabajo

1. Crea una rama desde `main`: `feat/…`, `fix/…`, `refactor/…`, `chore/…` o `docs/…`.
2. Haz commits pequeños con [Conventional Commits](https://www.conventionalcommits.org/):
   `feat(indexer): soporta el patrón 1x02`.
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
| Unitario    | `tests/unit/`        | lógica pura, parsers y configuración                  |
| Integración | `tests/integration/` | SQLite real, `fs` real y render de Ink                |
| E2E         | `tests/e2e/`         | binario compilado: argv, stdout, `stderr` y exit code |

```bash
npm test                 # suite completa
npm run test:unit
npm run test:integration
npm run test:e2e
npm run coverage
npm run bench            # benchmarks de rendimiento
```

Reglas:

- Se prueba comportamiento público, no detalles de implementación.
- Cada test es independiente: su propio `mkdtemp`, su propia base.
- Nombres `describe > it` en Given/When/Then.
- Los umbrales de cobertura viven en `vitest.config.ts` (85/85/80/85).
- Si tocas escáner, indexador, parser o consultas, adjunta el resultado de
  `npm run bench` en la PR. El presupuesto está en `docs/guides/performance.md`.

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
- `docs/plans/`: planes de trabajo.
- `PLAN.md`: hitos y estado.

Al cerrar un hito, actualiza las tablas de estado de `PLAN.md`, `README.md` y `AGENTS.md`
en el mismo PR.

## Publicación

El binario es `mm` y la distribución oficial son las GitHub Releases: bundles portables,
checksums SHA-256, SBOM y atestaciones de procedencia. No se publica en npm. Los pasos
completos están en [`docs/guides/release.md`](./docs/guides/release.md); no muevas un tag
ni subas assets manualmente desde una rama de desarrollo.
