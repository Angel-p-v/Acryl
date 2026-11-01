# AGENTS.md — Acryl

Normas obligatorias para cualquier agente de IA o desarrollador que trabaje en este repositorio. Lee este archivo antes de inspeccionar, modificar, probar, confirmar o fusionar código.

El objetivo es mantener una aplicación de escritorio fiable, segura, mantenible, eficiente y completamente verificable. Prioriza la corrección, la simplicidad y la calidad del código sobre la velocidad de implementación.

Estas instrucciones complementan las configuraciones del repositorio, las políticas de GitHub y los archivos `AGENTS.md` de directorios específicos. Respeta siempre las restricciones más estrictas que sean aplicables.

---

## 1. Objetivo del proyecto

`acryl` es una aplicación de escritorio nativa para Linux que indexa y gestiona colecciones locales de películas, series y anime mediante una base de datos SQLite. Permite buscar, consultar información y reproducir contenido mediante el reproductor externo `mpv`.

La capa de presentación está implementada en Rust puro con GTK4 / Libadwaita (sin motor Web, sin Tauri, sin frontend web). La lógica de negocio reside en capas de dominio y persistencia en Rust.

Documentación principal:

- Plan de desarrollo: `PLAN_DESKTOP.md`.
- Arquitectura e instrucciones: `AGENTS.md`.
- Decisiones de arquitectura: `docs/adr/`.
- Documentación de usuario: `README.md`.

Antes de empezar cualquier tarea, comprueba el estado real del repositorio. No asumas que la documentación, las rutas, los scripts, las dependencias o el estado de los hitos coinciden con el código actual.

---

## 2. Stack tecnológico

El stack está fijado. No cambies tecnologías, versiones principales ni arquitectura sin una justificación técnica y, cuando corresponda, un ADR aprobado.

| Capa | Tecnología | Requisitos |
|---|---|---|
| Backend / UI | Rust, edición 2024 | Ownership correcto, errores controlados, API segura |
| Interfaz gráfica | GTK4 + Libadwaita (gdk-rs) | Renderizado nativo, sin WebView, soporte Wayland/Hyprland |
| Base de datos | SQLite + `rusqlite` | Consultas parametrizadas, migraciones y FTS5, WAL |
| Reproducción | mpv (proceso externo) | Delegación de reproducción a reproductor externo `mpv` |
| Configuración | YAML + Serde | Deserialización y validación explícitas |
| Pósters / imágenes | `gdk-pixbuf` / `cairo` | Renderizado nativo dentro de GTK4 |
| Tests backend | Cargo | Unitarios, integración y regresión |
| E2E | Herramientas compatibles con GTK4 | Flujos críticos de la aplicación |

No migres a otro framework, ORM, reproductor, sistema de configuración o herramienta de tests únicamente por preferencia personal.

Si una dependencia propuesta tiene alternativas relevantes, compara mantenimiento, seguridad, compatibilidad con Linux, rendimiento, tamaño de distribución, complejidad y coste de mantenimiento.

No añadas dependencias si la funcionalidad puede resolverse de forma clara y segura con las herramientas existentes.

No uses Node.js, npm, Tauri, React, TypeScript ni WebView: han sido eliminados (ver ADR-0017 y PLAN_DESKTOP.md).

---

## 3. Estructura del repositorio

La estructura real es:

```text
app-media-manager/
├── src/
│   ├── main.rs             Punto de entrada GTK4
│   ├── lib.rs              Biblioteca (config, core, db, ui)
│   ├── config/             Gestión de configuración YAML (mod.rs)
│   ├── core/               Lógica de negocio (library, metadata, player, poster_service, mod.rs)
│   │   ├── library.rs      Indexado de carpetas, detección de medios, limpieza de huérfanos
│   │   ├── metadata.rs     Sincronización de metadatos y pósters desde TVmaze / AniList
│   │   ├── player.rs       Lanzamiento de mpv, argumentos, socket IPC, historial de reproducción
│   │   ├── poster_service.rs Caché y descarga asíncrona de pósters
│   │   └── mod.rs
│   ├── db/                 SQLite, migraciones, repositorios, FTS5
│   │   ├── connection.rs   Apertura de BD, migraciones, PRAGMA
│   │   ├── file_repo.rs    Repositorio de archivos multimedia y pistas
│   │   ├── media_repo.rs   Repositorio de medios, búsquedas FTS5, historial de reproducción
│   │   ├── migrations.rs   Migraciones versionadas (schema_migrations)
│   │   └── mod.rs
│   └── ui/                 Interfaz GTK4 / Libadwaita
│       ├── mod.rs
│       ├── i18n.rs         Internacionalización (diccionarios es/en, cambio en caliente)
│       ├── main_window.rs  Ventana principal, navegación por Stack, galería, historial
│       ├── media_details.rs Vista de detalles de medio, episodios, selector de temporada
│       ├── media_list.rs   Galería virtualizada con GtkGridView y tarjetas de póster
│       ├── poster.rs       Renderizado de pósters y banners con gdk-pixbuf
│       ├── search_bar.rs   Barra de búsqueda por título
│       ├── settings_window.rs Vista de ajustes (carpetas, idioma, metadatos)
│       ├── theme.rs        Tema visual (CSS y paleta de colores Crunchyroll)
│       ├── track_dialog.rs Selector de pistas de audio y subtítulos
│       └── locales/        Diccionarios de traducción (es.json, en.json)
├── tests/
│   ├── media_persistence.rs Tests de integración con SQLite temporal
│   ├── media_stress.rs     Pruebas de rendimiento con miles de registros
│   ├── mpv_player.rs       Tests de integración con reproductor mpv
│   └── e2e_app.rs          Tests end-to-end de flujos críticos GTK4
├── scripts/
│   ├── install.sh          Instalador (sin Node.js, solo Rust + GTK)
│   ├── uninstall.sh        Desinstalador
│   ├── update.sh           Actualizador
│   └── diagnostic/acryl-check.sh  Comprobación general de salud y gráficos
├── docs/
│   ├── adr/                Decisiones de arquitectura (ADR-0017 a ADR-0020)
│   │   ├── 0017-migracion-gtk4-libadwaita.md
│   │   ├── 0018-virtualizacion-galeria-gtkgridview.md
│   │   ├── 0019-deteccion-pistas-ffprobe-seleccion-mpv.md
│   │   └── 0020-cache-poster-descarga-asincrona.md
│   ├── guides/             Guías de arquitectura, ejemplos, migración, performance, release
│   ├── plan-ui-visual-pistas-escala-masiva.md  Plan de UI visual y escalado
│   └── README.md
├── PLAN_DESKTOP.md         Plan de migración y hitos (incluye hitos E.1-E.5)
├── AGENTS.md               Este archivo
├── README.md               Documentación de usuario
├── CONTRIBUTING.md         Flujo de trabajo y Pull Requests
├── SECURITY.md             Política de seguridad
├── LICENSE                 Licencia MIT
├── Cargo.toml              Proyecto Rust único (edición 2024, rust 1.90)
├── Cargo.lock
├── CHANGELOG.md
├── .github/workflows/ci.yml CI de GitHub Actions (fmt, clippy, test, audit, tarpaulin)
├── acryl.desktop           Entrada de menú de aplicaciones
└── icons/                  Recursos gráficos (icono, etc.)
```

Esta estructura es orientativa. Comprueba las rutas reales antes de utilizarlas y no crees directorios vacíos o estructuras duplicadas innecesariamente.

No existen las carpetas `frontend/` ni `src-tauri/` ni un `package.json` en la raíz. No las recrees.

---

## 4. Principios arquitectónicos obligatorios

### 4.1. Separación de responsabilidades

1. El dominio no depende de la UI, de GTK ni de `mpv` directamente en capas superiores.
2. La UI no ejecuta SQL ni accede directamente al sistema de archivos mediante llamadas sueltas.
3. El acceso a SQLite se concentra en la capa de persistencia (`src/db/`).
4. La lógica de negocio reside en `src/core/` y es llamada directamente desde la UI (sin serialización JSON intermedia ni comandos Tauri).
5. La lógica de negocio no debe duplicarse entre UI y Rust.
6. La UI no debe conocer detalles internos de SQLite, FFI o del reproductor.
7. Las dependencias deben apuntar hacia el dominio, no al contrario.
8. Las interfaces deben ser pequeñas y específicas; no introduzcas abstracciones hasta que aporten una ventaja real.

Mantén una separación clara entre dominio, persistencia, servicios, configuración y presentación, adaptándola a la complejidad real del proyecto.

### 4.2. Inyección de dependencias

- Prefiere dependencias explícitas y controladas.
- Evita singletons globales y estado global mutable sin necesidad justificada.
- No abras y cierres innecesariamente la base de datos en cada operación si existe una conexión gestionada de forma segura.
- Define la estrategia de concurrencia según las restricciones reales de SQLite y `rusqlite`.
- Evita bloquear el hilo principal de la interfaz con operaciones costosas.

### 4.3. Contratos internos Rust

Cada función expuesta entre capas debe tener un contrato claro y estable.

- Define entradas y salidas explícitas.
- Usa Serde para serialización interna cuando corresponda.
- Valida los datos en el límite de entrada.
- Define errores que permitan distinguir errores recuperables, datos no encontrados, entradas inválidas y fallos internos sin revelar información sensible.

### 4.4. Compatibilidad de datos

- Investiga el esquema de SQLite existente antes de modificarlo (`src/db/migrations.rs`).
- Preserva los datos del usuario durante las actualizaciones.
- No recrees ni elimines bases de datos automáticamente para resolver errores.
- Toda modificación incompatible del esquema debe tener una migración versionada y probada.
- Las migraciones deben ser transaccionales, idempotentes cuando proceda y seguras frente a interrupciones.
- Comprueba la compatibilidad con bases de datos existentes y con instalaciones nuevas.
- No elimines columnas, índices, archivos o metadatos sin justificar la necesidad y definir una migración segura.

---

## 5. Flujo obligatorio antes de modificar código

Antes de implementar una tarea:

1. Ejecuta `git status --short --branch`.
2. Comprueba la rama actual y si hay cambios sin confirmar.
3. Lee las instrucciones `AGENTS.md` aplicables y la documentación relevante.
4. Inspecciona el código existente y los tests relacionados.
5. Identifica los scripts reales de `Cargo.toml` y `scripts/`.
6. Comprueba las versiones de Rust y las herramientas requeridas.
7. Identifica los comportamientos existentes que no deben romperse.
8. Determina el alcance, los riesgos y las dependencias de la tarea.
9. Establece qué pruebas y verificaciones demostrarán que el cambio es correcto.

No sobrescribas cambios previos del usuario ni archivos modificados por otros procesos. No ejecutes `git reset --hard`, `git clean -fd`, `git checkout -- .` ni operaciones equivalentes para descartar trabajo sin autorización explícita.

Si hay cambios preexistentes, identifica cuáles pertenecen a la tarea y evita incluirlos accidentalmente en los commits.

### Planificación

Para tareas pequeñas, utiliza un plan breve.

Para funcionalidades complejas, migraciones, cambios de arquitectura o modificaciones que afecten a varias capas, define:

- Objetivo y comportamiento esperado.
- Archivos y módulos afectados.
- Contratos e interfaces implicados.
- Riesgos y posibles regresiones.
- Estrategia de pruebas.
- Criterios de aceptación.
- Documentación que deberá actualizarse.

Divide las tareas complejas en pasos verificables. No amplíes el alcance de un hito sin justificarlo.

---

## 6. Buenas prácticas obligatorias de Rust

Aplica las convenciones oficiales de Rust y los patrones recomendados por el ecosistema.

### 6.1. Diseño y ownership

- Prefiere la propiedad y el borrowing antes que clones innecesarios.
- Utiliza lifetimes explícitos cuando sean necesarios, no por sistema.
- Evita `clone()` cuando una referencia, un movimiento o una estructura de datos diferente sea más apropiada.
- Prefiere composición frente a jerarquías artificiales.
- Mantén funciones pequeñas, cohesionadas y con responsabilidades claras.
- Utiliza iteradores y las abstracciones idiomáticas de Rust cuando mejoren la claridad.
- No optimices prematuramente: mide antes de introducir complejidad por rendimiento.
- Evita asignaciones, copias y conversiones innecesarias en operaciones frecuentes.

### 6.2. Errores y resultados

- Utiliza `Result<T, E>` para operaciones que pueden fallar.
- Define tipos de error específicos cuando aporten contexto y permitan una recuperación adecuada.
- Utiliza bibliotecas como `thiserror` o `anyhow` únicamente cuando encajen con la necesidad concreta.
- En bibliotecas y dominio, prioriza errores tipados y recuperables.
- En los límites de la aplicación, añade contexto útil sin ocultar la causa original.
- No conviertas todos los errores en cadenas genéricas demasiado pronto.
- No ignores resultados de operaciones críticas.
- No utilices `unwrap()`, `expect()` o `panic!()` en rutas de ejecución normales que puedan recibir datos externos o fallar por circunstancias operativas.
- Pueden utilizarse en tests, invariantes demostrables o inicializaciones realmente infalibles, siempre que la justificación sea clara.
- No silencies errores con `let _ = ...` salvo que ignorar el resultado sea deliberado y seguro.

### 6.3. Concurrencia y asincronía

- No bloquees el hilo principal de la interfaz.
- No mantengas bloqueos durante operaciones de E/S lentas.
- Evita bloqueos anidados y diseños que puedan provocar deadlocks.
- Elige `Mutex`, `RwLock`, canales o primitivas asíncronas según las necesidades reales.
- No uses primitivas asíncronas para resolver problemas que no requieren asincronía.
- Controla cancelación, cierre, timeouts y propagación de errores en operaciones de larga duración cuando proceda.
- Evita crear tareas que puedan quedar ejecutándose después de cerrar la aplicación.

### 6.4. Código inseguro y FFI

- Prohíbe el uso de `unsafe` salvo necesidad técnica demostrable.
- Encapsula el código `unsafe` en módulos pequeños con interfaces seguras.
- Documenta los invariantes de seguridad de cada bloque `unsafe`.
- Valida punteros, tamaños, alineación, ownership y vida útil de los recursos.
- No conviertas punteros en referencias sin demostrar las garantías necesarias.
- Gestiona explícitamente la adquisición y liberación de recursos nativos.
- Implementa `Drop` cuando corresponda y evita liberaciones dobles, use-after-free y fugas.
- Para `mpv`, comprueba los requisitos de inicialización, destrucción, callbacks, afinidad de hilos y gestión de procesos externos.
- No invoques funciones nativas después de destruir sus recursos.

### 6.5. Dependencias y compilación

- Respeta `Cargo.lock` para la aplicación.
- Evita dependencias duplicadas o sin mantenimiento.
- Revisa compatibilidad, licencias, avisos de seguridad y coste de mantenimiento antes de añadir dependencias.
- No actualices masivamente las dependencias dentro de una funcionalidad ordinaria.
- Utiliza las herramientas de lint de forma estricta y corrige las advertencias reales.
- No añadas atributos `allow` para ocultar problemas sin justificación.

---

## 7. Base de datos y sistema de archivos

### SQLite

- Utiliza siempre consultas parametrizadas.
- Nunca construyas consultas con concatenación de valores procedentes del usuario.
- Valida identificadores y fragmentos SQL dinámicos mediante listas permitidas cuando sean necesarios.
- Gestiona las transacciones y los errores de forma explícita.
- Comprueba índices y planes de consulta si hay problemas de rendimiento.
- Evita consultas N+1 y lecturas repetidas innecesarias.
- Respeta las garantías de concurrencia y las limitaciones de SQLite.
- Configura WAL y FTS5 según los requisitos reales de la aplicación (`PRAGMA journal_mode = WAL` en `connection.rs`).
- Comprueba la recuperación tras fallos y la integridad de los datos cuando corresponda.
- No almacenes secretos ni información sensible sin una necesidad justificada.

### Archivos multimedia

- Gestiona rutas con las APIs apropiadas de Rust.
- No asumas que las rutas son válidas, existen o apuntan a archivos accesibles.
- Controla archivos desaparecidos, permisos insuficientes, rutas Unicode y enlaces simbólicos cuando sean relevantes.
- Evita recorrer directorios completos repetidamente sin necesidad.
- No sigas enlaces simbólicos de forma recursiva sin controlar ciclos y límites.
- Maneja colecciones grandes sin bloquear la interfaz.
- No elimines ni modifiques archivos multimedia del usuario como efecto secundario inesperado.

### Migraciones

- Versiona los cambios del esquema en `src/db/migrations.rs`.
- Prueba las migraciones desde bases de datos antiguas y desde instalaciones limpias.
- Verifica que una migración fallida no deje datos parcialmente modificados.
- Conserva los datos existentes.
- Documenta cualquier incompatibilidad inevitable y cómo se gestiona.

---

## 8. Seguridad de la aplicación

La aplicación debe seguir el principio de mínimo privilegio.

- No concedas acceso general al sistema de archivos, shell o procesos si basta con una operación limitada.
- Valida en Rust las entradas recibidas desde la UI.
- No consideres la UI una frontera de seguridad confiable.
- No construyas comandos de shell con datos de usuario.
- Evita ejecutar procesos externos; cualquier excepción debe estar justificada y restringida.
- **Excepción aprobada:** `mpv` se ejecuta como proceso externo mediante `std::process::Command` para la reproducción de vídeo, ya que es la solución más robusta y ampliamente soportada para reproducir contenido multimedia en Linux. Esta excepción está documentada y justificada por la falta de fiabilidad de GtkMediaFile con ciertos codecs y formatos.
- No expongas rutas arbitrarias, operaciones de borrado ni ejecución de comandos a través de interfaces genéricas.
- Protege contra traversal de directorios y accesos no autorizados.
- No incluyas secretos, tokens, contraseñas o datos personales en el código, los logs o los commits.
- Utiliza `.env.example` para documentar variables de ejemplo sin credenciales reales.
- Mantén las dependencias actualizadas de forma controlada.
- Investiga las alertas de seguridad y no las ignores automáticamente.

Si una funcionalidad requiere privilegios adicionales, explica la necesidad y utiliza el mecanismo más restrictivo posible.

No hay WebView ni comandos Tauri, por tanto no aplican permisos de Tauri ni sanitización de IPC web.

---

## 9. Política obligatoria de pruebas

Cada cambio debe incluir pruebas proporcionales al riesgo y al comportamiento modificado.

No basta con comprobar que el proyecto compila. Los tests deben demostrar que la funcionalidad se comporta correctamente y que los casos de error están controlados.

### 9.1. Tests unitarios

Comprueba, según corresponda:

- Casos normales y límites.
- Valores vacíos, inválidos y ausentes.
- Errores y recuperación.
- Transformaciones de datos.
- Reglas de negocio.
- Funciones puras y utilidades.
- Parsing y validación de datos.

Los tests deben ser deterministas, rápidos, aislados y fáciles de entender.

### 9.2. Tests de integración

Comprueba la interacción real entre módulos:

- Repositorios y SQLite.
- Transacciones y migraciones.
- Servicios y lógica de dominio.
- Lectura de archivos y metadatos.
- Configuración YAML.
- Manejo de errores y permisos.

Utiliza una base de datos y directorios temporales independientes por test (`tests/media_persistence.rs` usa `tempfile`). No dependas de la biblioteca personal del usuario ni de datos reales de su equipo.

### 9.3. Tests de regresión

Cuando se corrija un bug:

1. Reproduce el problema.
2. Añade un test que falle antes de la corrección cuando sea viable.
3. Implementa el cambio mínimo correcto.
4. Comprueba que el test ahora pasa.
5. Ejecuta las pruebas relacionadas para descartar regresiones.

No elimines, desactives ni debilites un test porque revele un fallo de la implementación.

### 9.4. Tests E2E

Para los flujos críticos, comprueba la aplicación integrada cuando la infraestructura lo permita:

- Inicio de la aplicación GTK4.
- Carga de la biblioteca desde SQLite.
- Búsqueda y navegación.
- Apertura de los detalles de un elemento.
- Gestión de imágenes y pósteres.
- Reproducción y cierre del reproductor cuando estén implementados.
- Persistencia entre reinicios.
- Manejo de errores de datos y archivos.
- Cierre correcto de la aplicación.

Si el E2E requiere hardware, codecs, servicios o una sesión gráfica no disponible, ejecuta las verificaciones alternativas posibles y documenta exactamente la limitación. No declares que una prueba se ha superado si no se ha ejecutado.

### 9.5. Cobertura

- Mantén o mejora la cobertura de las rutas importantes.
- Define umbrales de cobertura cuando exista una configuración adecuada.
- No persigas un porcentaje elevado mediante tests triviales que no comprueben comportamientos reales.
- Prioriza las reglas de negocio, persistencia, seguridad, manejo de errores y flujos críticos.
- No reduzcas los umbrales existentes sin justificación y revisión explícitas.

### 9.6. Calidad de los propios tests

- Evita tests frágiles basados en detalles internos.
- No uses sleeps arbitrarios cuando puedan emplearse condiciones o sincronización determinista.
- No hagas depender un test del orden de ejecución de otro.
- No ocultes errores con mocks que simulan un éxito imposible.
- Utiliza mocks para límites externos cuando sean útiles, pero conserva tests de integración con dependencias reales cuando sean importantes.
- No introduzcas dependencias de red en la suite ordinaria si no son necesarias.
- Los tests que requieren recursos externos deben identificarse claramente.

---

## 10. Puertas de calidad antes de confirmar cambios

Antes de hacer un commit, ejecuta las comprobaciones pertinentes para las capas modificadas.

### Backend Rust

Desde la raíz del proyecto:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo build --all-features
```

Ajusta los comandos si existen características incompatibles entre sí, targets específicos o dependencias opcionales que requieran otro entorno. No omitas una verificación silenciosamente.

Si se han modificado dependencias, ejecuta las comprobaciones de seguridad pertinentes:

```bash
cargo audit
```

Comprueba la disponibilidad de las herramientas y revisa los resultados. Distingue los problemas introducidos por el cambio de las vulnerabilidades preexistentes. No ignores alertas relevantes ni actualices dependencias de forma masiva para silenciarlas sin analizar el impacto.

### Reglas de aceptación

- No debe haber errores de compilación.
- No deben quedar errores de lint nuevos.
- Los tests aplicables deben pasar.
- No deben aparecer regresiones conocidas.
- No debe haber errores de formato.
- Los cambios deben estar revisados.
- No deben quedar dependencias, archivos o código temporal innecesarios.
- El build de producción debe pasar cuando el cambio afecte al artefacto distribuido.
- Los fallos preexistentes deben documentarse con evidencia; no deben atribuirse automáticamente al entorno.

Si una comprobación falla, investiga la causa y corrígela antes de continuar. No repitas comandos sin modificar nada esperando un resultado distinto.

---

## 11. Git, commits y protección de cambios

### 11.1. Ramas

- Nunca trabajes directamente en `main`.
- Utiliza ramas específicas por funcionalidad, corrección o tarea técnica.
- Prefiere los prefijos `feat/`, `fix/`, `refactor/`, `test/`, `docs/` y `chore/`.
- Actualiza la rama de trabajo respecto a `main` cuando corresponda y resuelve conflictos de forma explícita.
- No sobrescribas trabajo de terceros.

Ejemplo:

```text
feat/hito-e2-ui-completa
fix/poster-rendering
test/sqlite-migrations
```

### 11.2. Commits

Utiliza Conventional Commits con scopes coherentes:

```text
feat(db): migrate media repository to rusqlite
fix(ui): preserve poster alignment
test(db): cover migration rollback
refactor(core): separate library services
docs(plan): update milestone status
chore(ci): enforce Rust quality checks
```

Cada commit debe representar una intención coherente y revisable.

Como referencia, intenta mantener cada commit por debajo de 15 archivos y 500 líneas modificadas. Si el cambio es mayor, divídelo cuando resulte práctico. No fragmentes artificialmente una modificación inseparable para cumplir un límite numérico.

Antes de confirmar:

```bash
git status --short
git diff --check
git diff --stat
git diff
```

- Revisa los cambios completos, no solo el resumen.
- Utiliza `git add` con rutas específicas o `git add -p`.
- No incluyas secretos, artefactos generados ni cambios ajenos.
- No confirmes código que no hayas inspeccionado.
- Cada commit debe dejar el código en un estado coherente.

### 11.3. No falsificar verificaciones

Nunca afirmes que se han ejecutado o superado tests, compilaciones, revisiones o checks si no existe evidencia de ello.

Distínguete entre:

- Ejecutado y superado.
- Ejecutado y fallido.
- No ejecutado.
- Bloqueado por el entorno.
- No aplicable, con justificación.

No ocultes fallos mediante redirecciones, `|| true`, desactivación de reglas o eliminación de tests.

---

## 12. Política obligatoria de Pull Requests

Toda funcionalidad o corrección que llegue a `main` debe pasar por una Pull Request, salvo que exista una política de repositorio aprobada que establezca otra cosa.

### 12.1. Antes de abrir la PR

1. Confirma que la rama contiene únicamente cambios relacionados.
2. Revisa `git diff main...HEAD` y los commits de la rama.
3. Ejecuta las comprobaciones de calidad aplicables.
4. Comprueba que la compilación de producción funciona cuando corresponde.
5. Verifica que los tests cubren los nuevos comportamientos y errores relevantes.
6. Revisa posibles secretos, dependencias, permisos, migraciones y cambios incompatibles.
7. Comprueba que la documentación está actualizada.
8. Asegúrate de que no se ha debilitado ninguna protección existente.

Abre la PR con un título descriptivo y un cuerpo que incluya:

- Objetivo y contexto.
- Resumen de los cambios.
- Decisiones técnicas relevantes.
- Tests y verificaciones ejecutadas.
- Resultados de esas verificaciones.
- Riesgos, limitaciones y cuestiones pendientes.
- Capturas de pantalla cuando haya cambios visuales relevantes.
- Migraciones o pasos de actualización cuando proceda.

### 12.2. Revisión técnica de la PR

Antes de fusionar, revisa la PR como si fueras un revisor independiente. No te limites a leer el mensaje del commit ni a comprobar que el código compila.

Inspecciona:

- Bugs lógicos y casos límite.
- Regresiones respecto al comportamiento anterior.
- Seguridad, permisos y validación de entradas.
- Errores de ownership, concurrencia y gestión de recursos en Rust.
- Consultas SQL inseguras y migraciones destructivas.
- Manejo incompleto de errores.
- Fugas de memoria o recursos nativos.
- Rendimiento y uso excesivo de recursos.
- Accesibilidad y regresiones visuales.
- Tests ausentes, débiles, frágiles o que no verifican el comportamiento real.
- Dependencias innecesarias o vulnerables.
- Documentación obsoleta.
- Cambios accidentales fuera del alcance.

No marques un comentario como resuelto hasta haber comprobado su corrección.

### 12.3. Condiciones de merge

**Está prohibido fusionar una PR si se cumple cualquiera de estas condiciones:**

- Hay checks obligatorios pendientes o fallidos.
- No se ha realizado la revisión técnica del diff.
- Existen errores de implementación conocidos.
- Hay comentarios de revisión relevantes sin resolver.
- Los tests pertinentes no han pasado.
- La compilación requerida falla.
- La PR introduce regresiones sin resolver.
- La rama contiene cambios ajenos o peligrosos.
- No se cumplen las reglas de protección de ramas del repositorio.

Cuando todos los requisitos estén satisfechos, utiliza squash merge si esa es la política del proyecto:

```bash
gh pr merge <PR> --squash --delete-branch
```

No uses `--admin`, no omitas checks, no desactives protecciones ni fuerces un merge para acelerar el trabajo. Si se requiere una aprobación humana que el agente no puede proporcionar legítimamente, detente y solicítala.

Si GitHub no permite fusionar por una restricción legítima, informa del bloqueo. No busques formas de eludirlo.

### 12.4. Después del merge

- Comprueba que GitHub confirma el merge.
- Actualiza `main` localmente de forma segura.
- Verifica que el commit fusionado corresponde a la PR esperada.
- Comprueba el estado de CI de `main`.
- Verifica el estado del árbol de trabajo.
- Actualiza `PLAN_DESKTOP.md` y `README.md` cuando corresponda.
- Comprueba que la PR y el hito han quedado correctamente documentados.

No empieces el siguiente hito hasta que el actual esté fusionado y sus verificaciones requeridas estén completas.

---

## 13. Integración continua obligatoria

La calidad no debe depender exclusivamente de la memoria del agente. Siempre que el repositorio lo permita, implementa y mantiene una CI reproducible en GitHub Actions.

La CI debe ejecutarse en Pull Requests y en las ramas relevantes, como `main`.

Como mínimo, debe comprobar:

- Instalación reproducible de dependencias.
- Formato de Rust (`cargo fmt --check`).
- Clippy sin advertencias no justificadas.
- Tests unitarios y de integración de Rust (`cargo test`).
- Build de Rust (`cargo build`).
- Auditoría de dependencias (`cargo audit`) cuando esté configurada de forma estable.

Añade pruebas E2E y builds de distribución en Linux cuando sean viables y aporten cobertura relevante.

Buenas prácticas de CI:

- Fija versiones de acciones y herramientas de forma controlada.
- Respeta `Cargo.lock`.
- Configura permisos mínimos para los `GITHUB_TOKEN`.
- No expongas secretos en logs.
- No ejecutes código no confiable de Pull Requests con secretos privilegiados.
- Utiliza cachés sin comprometer la reproducibilidad.
- Evita condiciones de carrera entre workflows.
- Configura cancelación de ejecuciones obsoletas cuando sea apropiado.
- Separa los jobs para identificar rápidamente los fallos.
- Mantén los logs suficientemente claros para diagnosticar errores.
- No hagas que los checks obligatorios dependan de servicios externos innecesarios.
- No marques como correcto un job porque un paso de test haya sido omitido.

Antes de crear un workflow, inspecciona `.github/workflows/` y reutiliza lo que ya exista. No dupliques pipelines sin necesidad.

Las protecciones de `main` deben configurarse en GitHub cuando corresponda; escribirlas en este archivo no las activa automáticamente.

---

## 14. Estrategia de rendimiento

No introduzcas optimizaciones especulativas.

Cuando una operación resulte lenta:

1. Reproduce el problema.
2. Mide dónde se consume el tiempo.
3. Identifica la causa.
4. Implementa la solución más sencilla que la resuelva.
5. Comprueba que no afecta a la corrección.
6. Añade benchmarks o tests de rendimiento cuando sean útiles.

Presta especial atención a:

- Bibliotecas multimedia grandes.
- Consultas SQLite y búsquedas FTS5.
- Lectura de metadatos y archivos.
- Renderizado de listas e imágenes.
- Decodificación de pósteres con gdk-pixbuf.
- Memoria y copias innecesarias.
- Operaciones nativas de reproducción con `mpv`.

Evita bloquear la interfaz, cargar colecciones completas cuando no sea necesario o repetir consultas costosas en cada renderizado.

No añadas concurrencia, cachés o sistemas de virtualización sin demostrar que resuelven un problema real o un requisito del proyecto.

---

## 15. Documentación y decisiones técnicas

- El código, los identificadores, los nombres de tipos y los comentarios técnicos se escriben en inglés.
- La documentación dirigida al usuario se escribe en español, salvo que el proyecto establezca otra convención.
- Los comentarios explican el porqué, las restricciones o los invariantes; no repiten el código.
- Actualiza la documentación cuando cambien los comandos, la arquitectura, los requisitos, el comportamiento o la instalación.
- Mantén `README.md` y `PLAN_DESKTOP.md` coherentes con la implementación real.
- No marques una funcionalidad como completada si solo existe parcialmente.
- No documentes comandos que no funcionen.

Cuando una decisión arquitectónica tenga varias alternativas razonables y consecuencias relevantes, crea un ADR en `docs/adr/` que incluya:

1. Contexto.
2. Opciones consideradas.
3. Decisión.
4. Motivos.
5. Consecuencias y compromisos.
6. Alternativas descartadas.

Anota la decisión en el plan cuando afecte a los hitos.

No crees ADR para decisiones triviales que puedan resolverse mediante las convenciones existentes.

---

## 16. Política de dependencias

Antes de añadir o actualizar una dependencia:

- Comprueba si ya existe una alternativa adecuada en el proyecto.
- Evalúa mantenimiento, compatibilidad, licencias y avisos de seguridad.
- Revisa el impacto en el tamaño del binario y los requisitos del sistema.
- Comprueba las versiones compatibles con el toolchain.
- Añade únicamente las dependencias necesarias.
- Actualiza el lockfile correspondiente (`Cargo.lock`).
- Ejecuta los tests y builds pertinentes.
- Documenta los cambios relevantes.

No ejecutes actualizaciones generales de dependencias dentro de una tarea funcional sin justificar su necesidad.

No elimines dependencias basándote únicamente en una búsqueda textual; verifica su uso indirecto, los scripts y la configuración de compilación.

No añadas dependencias de Node.js, npm, Tauri, React o TypeScript: han sido eliminadas del proyecto.

---

## 17. Política de alcance y cambios visuales

- Implementa únicamente el alcance acordado.
- No refactorices módulos no relacionados durante una corrección puntual.
- No modifiques contratos públicos sin analizar los consumidores afectados.
- No cambies rutas, formatos de datos o configuraciones por conveniencia.
- Conserva el comportamiento existente salvo que el requisito indique lo contrario.
- En cambios visuales, compara el resultado con el diseño y el comportamiento anteriores.
- No modifiques posicionamiento, dimensiones, proporciones, recortes o distribución de componentes fuera del objetivo de la tarea.
- Si un arreglo necesita cambios adicionales, explica la relación y mantenlos dentro del alcance necesario.
- Si descubres un problema independiente, documéntalo y propón una tarea separada.

La ausencia de instrucciones detalladas no autoriza a rediseñar partes existentes ni a eliminar funcionalidades.

---

## 18. Política de commits, PR y autonomía

El agente debe completar las tareas de forma autónoma siempre que disponga de permisos, herramientas y contexto suficientes.

El flujo habitual es:

1. Inspeccionar.
2. Planificar.
3. Crear una rama.
4. Implementar.
5. Añadir o actualizar tests.
6. Ejecutar las verificaciones.
7. Revisar el diff.
8. Crear commits coherentes.
9. Hacer push.
10. Abrir la PR.
11. Revisar la implementación y la CI.
12. Corregir los fallos y repetir las verificaciones.
13. Fusionar si se cumplen todas las condiciones y los permisos lo permiten.
14. Verificar el estado final.
15. Actualizar la documentación.

No te detengas únicamente porque una comprobación falla: investiga y trata de resolver el problema dentro del alcance acordado.

Detente y solicita intervención humana cuando sea necesario para:

- Obtener permisos o credenciales.
- Resolver una decisión de producto realmente ambigua.
- Aprobar cambios que requieren autorización humana.
- Aceptar una migración destructiva o una pérdida de compatibilidad.
- Resolver una vulnerabilidad o un riesgo que no pueda mitigarse de forma segura.
- Superar una limitación real del entorno.
- Aprobar una excepción a las protecciones del repositorio.

Nunca inventes resultados ni ocultes bloqueos para aparentar que la tarea está terminada.

---

## 19. Criterios de finalización de una tarea

Una tarea no se considera completada hasta que se hayan satisfecho todos los criterios aplicables:

- [ ] El comportamiento solicitado está implementado.
- [ ] Se ha respetado la arquitectura.
- [ ] Los errores y casos límite relevantes están controlados.
- [ ] Se han añadido o actualizado los tests necesarios.
- [ ] Las pruebas pertinentes se han ejecutado y han pasado.
- [ ] El tipado, el lint y el formato son correctos.
- [ ] Los builds pertinentes se han completado.
- [ ] Se ha revisado el diff completo.
- [ ] No se han incluido cambios ajenos.
- [ ] La seguridad y la compatibilidad se han evaluado.
- [ ] La documentación está actualizada cuando corresponde.
- [ ] La PR y sus checks cumplen las condiciones de merge.
- [ ] Si se ha fusionado, se ha comprobado el resultado.

Si algún criterio no puede cumplirse, informa del motivo, la evidencia disponible y el siguiente paso necesario. No presentes una tarea parcialmente verificada como completada.

Al terminar, proporciona un resumen conciso con:

1. Qué se ha cambiado.
2. Decisiones técnicas importantes.
3. Tests y comandos ejecutados, con sus resultados.
4. Limitaciones o riesgos pendientes.
5. Rama, commits y PR.
6. Estado del merge y de la CI, si corresponde.

No incluyas resultados hipotéticos ni afirmaciones de éxito sin evidencia.

---

## 20. Reglas finales no negociables

1. Nunca trabajes directamente en `main`.
2. Nunca fusiones una PR con checks obligatorios fallidos o pendientes.
3. Nunca eludas las protecciones de GitHub para forzar un merge.
4. Nunca des por superados tests que no se han ejecutado.
5. Nunca elimines tests o reduzcas la seguridad para ocultar fallos.
6. Nunca sobrescribas cambios del usuario sin autorización.
7. Nunca introduzcas SQL inseguro, secretos o permisos innecesarios.
8. Nunca uses `unsafe` sin necesidad demostrable y garantías documentadas.
9. Nunca debilites el tipado o las herramientas de calidad para facilitar una implementación.
10. Nunca cambies la arquitectura o las tecnologías principales sin una justificación adecuada.
11. Nunca marques un hito como terminado antes de completar su proceso de revisión y verificación.
12. Prioriza siempre la corrección, la seguridad, la mantenibilidad y la experiencia de usuario.
13. No reintroduzcas Node.js, npm, Tauri, React, TypeScript ni WebView; el proyecto es Rust puro + GTK4/Libadwaita.

**Principio rector:** una funcionalidad no está terminada porque funcione en un caso concreto; está terminada cuando su comportamiento es correcto, sus errores están controlados, sus pruebas aportan confianza, sus cambios han sido revisados y la integración en `main` cumple las garantías del proyecto.

