# Contribuir a Acryl

Gracias por querer mejorar el proyecto. Esta guía resume el flujo de trabajo y las
reglas que mantienen `main` siempre desplegable.

## Antes de empezar

- Rust ≥ 1.90 y Cargo.
- Git.
- Un sistema Linux con GTK4 y Libadwaita instalados (ver `scripts/install.sh` o los
  requisitos del sistema en `README.md`).

```bash
git clone https://github.com/Angel-p-v/app-media-manager.git
cd app-media-manager
cargo build
cargo test
```

## Arquitectura

```
src/                      ← Rust puro (sin Tauri, sin WebView, sin Node.js)
  ├── main.rs             Punto de entrada GTK4
  ├── lib.rs              Biblioteca
  ├── config/             Gestión de configuración YAML (serde + serde_yaml)
  ├── core/               Lógica de negocio (library, metadata, player)
  ├── db/                 SQLite, migraciones, repositorios, FTS5
  └── ui/                 GTK4 / Libadwaita (ventana, widgets, vistas)
```

- El dominio no depende de la UI ni de GTK.
- La UI no ejecuta SQL ni accede directamente al sistema de archivos mediante llamadas sueltas.
- El acceso a SQLite se concentra en `src/db/`.
- La lógica de negocio reside en `src/core/` y es llamada directamente desde la UI.
- No hay singletons globales ni estado global mutable sin justificación.

Si un cambio cruza estas reglas, probablemente necesite un ADR en `docs/adr/`. Las
decisiones con más de una opción válida se documentan ahí antes de implementarlas.

## Flujo de trabajo

1. Crea una rama desde `main`: `feat/…`, `fix/…`, `refactor/…`, `chore/…` o `docs/…`.
2. Haz commits pequeños con [Conventional Commits](https://www.conventionalcommits.org/):
   `feat(config): añade soporte para idioma inglés`.
3. Ejecuta las verificaciones de calidad antes de abrir la PR:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo build --all-features
```

4. Abre la PR contra `main` con una descripción de **qué** y **por qué**, y el checklist
   de verificación.
5. Espera a que la CI esté verde.
6. El merge es **squash**; la rama se elimina. Nunca se commitea directamente en `main`.

Scopes admitidos: `config`, `db`, `core`, `ui`, `player`, `ci`, `docs`, `test`, `release`.

## Tests

| Nivel       | Ubicación          | Objetivo                                         |
| ----------- | ------------------ | ------------------------------------------------ |
| Unitario    | `src/**`           | lógica pura, parsing y configuración             |
| Integración | `tests/`           | SQLite real, repositorios, servicios de dominio  |

```bash
cargo test --all-targets --all-features
```

Reglas:

- Se prueba comportamiento público, no detalles de implementación.
- Cada test es independiente: su propio entorno, su propia base de datos temporal.
- No uses `unwrap()`/`expect()` en rutas que puedan recibir datos externos.

## Estilo

- Código e identificadores en inglés; documentación de usuario en español.
- Rust edición 2024, ownership correcto, errores controlados.
- Comenta **por qué**, no lo obvio.
- `cargo fmt` y `cargo clippy` son la fuente de verdad; no discutas el formato en la review.
- No añadas dependencias sin justificar el coste y pasar por el ADR si afecta a la
  arquitectura.
- No se permite `Node.js`, `npm`, `Tauri`, `React`, `TypeScript` ni `WebView`.

## Documentación

- `README.md`: instalación y uso.
- `docs/README.md`: índice de la documentación.
- `docs/adr/`: decisiones de arquitectura.
- `PLAN_DESKTOP.md`: plan de migración desktop y estado de hitos.
- `AGENTS.md`: instrucciones obligatorias para agentes de IA y desarrolladores.

Al cerrar un hito, actualiza las tablas de estado de `PLAN_DESKTOP.md`, `README.md` y `AGENTS.md`
en el mismo PR.

## Publicación

El binario es `acryl` y la distribución oficial son las GitHub Releases. No se publica en npm.
Los pasos completos están en los workflows de `.github/workflows/`; no muevas un tag ni
subas assets manualmente desde una rama de desarrollo.
