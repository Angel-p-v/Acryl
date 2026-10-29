# AGENTS.md — media-manager

Normas obligatorias para cualquier agente de IA o desarrollador que trabaje en este repositorio. Lee este archivo antes de inspeccionar, modificar, probar, confirmar o fusionar código.

El objetivo es mantener una aplicación de escritorio fiable, segura, mantenible, eficiente y completamente verificable. Prioriza la corrección, la simplicidad y la calidad del código sobre la velocidad de implementación.

Estas instrucciones complementan las configuraciones del repositorio, las políticas de GitHub y los archivos `AGENTS.md` de directorios específicos. Respeta siempre las restricciones más estrictas que sean aplicables.

---

## 1. Objetivo del proyecto

`media-manager` es una aplicación de escritorio para Linux que indexa y gestiona colecciones locales de películas, series y anime mediante una base de datos SQLite. Permite buscar, consultar información y reproducir contenido dentro de la propia aplicación, sin depender de reproductores externos ejecutados mediante `spawn`.

El proyecto migra la capa de presentación de la aplicación CLI/TUI anterior a una aplicación de escritorio nativa basada en Tauri, manteniendo y trasladando progresivamente la lógica de negocio.

Documentación principal:

- Plan de desarrollo: `PLAN_DESKTOP.md`.
- Arquitectura e instrucciones: `AGENTS.md`.
- Decisiones de arquitectura: `docs/adr/`.
- Documentación de usuario: `README.md`.

Antes de empezar cualquier tarea, comprueba el estado real del repositorio. No asumas que la documentación, las rutas, los scripts, las dependencias o el estado de los hitos coinciden con el código actual.

## 2. Stack tecnológico

El stack está fijado. No cambies tecnologías, versiones principales ni arquitectura sin una justificación técnica y, cuando corresponda, un ADR aprobado.

| Capa | Tecnología | Requisitos |
|---|---|---|
| Backend | Rust, edición 2024 | Ownership correcto, errores controlados y API segura |
| Frontend | TypeScript + React | Modo estricto y componentes mantenibles |
| Herramientas frontend | Node.js >= 22 | Respetar las versiones declaradas en el repositorio |
| Aplicación de escritorio | Tauri v2 | Permisos mínimos y comunicación tipada |
| Base de datos | SQLite + `rusqlite` | Consultas parametrizadas, migraciones y FTS5 |
| Reproducción | libmpv vía FFI o GStreamer | Integración nativa segura y sin bloquear la UI |
| Configuración | YAML + Serde | Deserialización y validación explícitas |
| Tests frontend | Vitest | Unitarios, integración y componentes |
| Tests backend | Cargo | Unitarios, integración y regresión |
| E2E | Herramientas compatibles con Tauri | Flujos críticos de la aplicación |

No migres a otro framework, ORM, reproductor, sistema de configuración o herramienta de tests únicamente por preferencia personal.

Si una dependencia propuesta tiene alternativas relevantes, compara mantenimiento, seguridad, compatibilidad con Linux, rendimiento, tamaño de distribución, complejidad y coste de mantenimiento.

No añadas dependencias si la funcionalidad puede resolverse de forma clara y segura con las herramientas existentes.

## 3. Estructura del repositorio

La estructura prevista es:

```text
app-media-manager/
├── frontend/
│   ├── src/
│   │   ├── components/
│   │   ├── services/
│   │   ├── types/
│   │   ├── __tests__/
│   │   ├── App.tsx
│   │   ├── main.tsx
│   │   └── index.css
│   ├── package.json
│   ├── vite.config.ts
│   └── tsconfig.json
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── commands.rs
│   │   ├── media.rs
│   │   ├── db.rs
│   │   └── core/
│   ├── tests/
│   ├── Cargo.toml
│   ├── Cargo.lock
│   └── tauri.conf.json
├── tests/
│   └── e2e/
├── docs/
│   └── adr/
├── PLAN_DESKTOP.md
├── AGENTS.md
├── README.md
└── package.json
```

Esta estructura es orientativa. Comprueba las rutas reales antes de utilizarlas y no crees directorios vacíos o estructuras duplicadas innecesariamente.

## 4. Principios arquitectónicos obligatorios

### 4.1. Separación de responsabilidades

1. El dominio no depende del frontend, del WebView ni de los comandos de Tauri.
2. El frontend no ejecuta SQL ni accede directamente al sistema de archivos mediante APIs de Node.js.
3. El acceso a SQLite se concentra en la capa de persistencia.
4. Los comandos de Tauri actúan como adaptadores de entrada: validan la solicitud, delegan en los servicios correspondientes y convierten los resultados en respuestas serializables.
5. La lógica de negocio no debe duplicarse entre React y Rust.
6. La UI no debe conocer detalles internos de SQLite, FFI o del reproductor.
7. Las dependencias deben apuntar hacia el dominio, no al contrario.
8. Las interfaces deben ser pequeñas y específicas; no introduzcas abstracciones hasta que aporten una ventaja real.

Mantén una separación clara entre dominio, persistencia, servicios, configuración, comandos y presentación, adaptándola a la complejidad real del proyecto.

### 4.2. Inyección de dependencias

- Prefiere dependencias explícitas y controladas.
- Evita singletons globales y estado global mutable sin necesidad justificada.
- En Tauri, utiliza el mecanismo de estado gestionado cuando corresponda, con sincronización y ciclo de vida adecuados.
- No abras y cierres innecesariamente la base de datos en cada comando si existe una conexión gestionada de forma segura y compartida.
- Define la estrategia de concurrencia según las restricciones reales de SQLite, Tauri y `rusqlite`.
- Evita bloquear el hilo principal o el runtime asíncrono con operaciones costosas.

### 4.3. Contratos Rust ↔ TypeScript

Cada comando expuesto al frontend debe tener un contrato claro y estable.

- Define entradas y salidas explícitas.
- Usa Serde para serialización y deserialización.
- Mantén sincronizados los tipos de TypeScript con los contratos reales de Rust.
- Valida los datos en el límite de entrada.
- No confíes en que TypeScript valida datos en tiempo de ejecución.
- Evita duplicar manualmente tipos compartidos cuando pueda generarse un contrato fiable sin añadir complejidad innecesaria.
- Define errores que permitan al frontend distinguir errores recuperables, datos no encontrados, entradas inválidas y fallos internos sin revelar información sensible.

### 4.4. Compatibilidad de datos

- Investiga el esquema de SQLite existente antes de modificarlo.
- Preserva los datos del usuario durante las actualizaciones.
- No recrees ni elimines bases de datos automáticamente para resolver errores.
- Toda modificación incompatible del esquema debe tener una migración versionada y probada.
- Las migraciones deben ser transaccionales cuando sea posible, idempotentes cuando proceda y seguras frente a interrupciones.
- Comprueba la compatibilidad con bases de datos existentes y con instalaciones nuevas.
- No elimines columnas, índices, archivos o metadatos sin justificar la necesidad y definir una migración segura.

## 5. Flujo obligatorio antes de modificar código

Antes de implementar una tarea:

1. Ejecuta `git status --short --branch`.
2. Comprueba la rama actual y si hay cambios sin confirmar.
3. Lee las instrucciones `AGENTS.md` aplicables y la documentación relevante.
4. Inspecciona el código existente y los tests relacionados.
5. Identifica los scripts reales de `package.json`, `frontend/package.json` y `Cargo.toml`.
6. Comprueba las versiones de Node.js, Rust y las herramientas requeridas.
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
- Para libmpv o GStreamer, comprueba los requisitos de inicialización, destrucción, callbacks y afinidad de hilos.
- No invoques funciones nativas después de destruir sus recursos.

### 6.5. Dependencias y compilación

- Respeta `Cargo.lock` para la aplicación.
- Evita dependencias duplicadas o sin mantenimiento.
- Revisa compatibilidad, licencias, avisos de seguridad y coste de mantenimiento antes de añadir dependencias.
- No actualices masivamente las dependencias dentro de una funcionalidad ordinaria.
- Utiliza las herramientas de lint de forma estricta y corrige las advertencias reales.
- No añadas atributos `allow` para ocultar problemas sin justificación.

## 7. Buenas prácticas obligatorias de TypeScript y React

### 7.1. Tipado estricto

- Mantén `strict: true` y las opciones de seguridad de tipos adecuadas en TypeScript.
- No utilices `any`, ni siquiera temporalmente en código que se vaya a confirmar.
- Para valores desconocidos, utiliza `unknown` y valida antes de operar con ellos.
- Evita conversiones de tipo inseguras.
- No utilices `as` para silenciar errores que deberían resolverse mediante un contrato o una validación correcta.
- Las aserciones justificadas son aceptables si existe una garantía verificable.
- Evita el operador de no nulidad `!` salvo que la garantía esté documentada o resulte evidente.
- Usa tipos explícitos en interfaces públicas, parámetros y retornos de API; permite la inferencia local cuando sea segura y legible.
- Activa, cuando sea compatible con el proyecto, opciones como `noUncheckedIndexedAccess` y `exactOptionalPropertyTypes`.
- No debilites la configuración del compilador para hacer pasar una implementación.

### 7.2. Modelado de datos

- Utiliza `interface` para contratos de objetos cuando favorezca su extensión y `type` para uniones, intersecciones y composiciones.
- Emplea uniones discriminadas para representar estados y resultados excluyentes.
- Usa `readonly` e inmutabilidad cuando sean apropiados.
- Evita tipos excesivamente genéricos y abstracciones que no aporten valor.
- No dupliques modelos de dominio entre componentes si pueden centralizarse correctamente.
- Valida datos externos en tiempo de ejecución cuando el contrato no sea plenamente confiable.

### 7.3. React y estado

- Sigue las reglas de los Hooks.
- No llames Hooks de forma condicional ni dentro de bucles.
- Utiliza componentes con una única responsabilidad clara.
- Evita efectos secundarios dentro del renderizado.
- No uses `useEffect` para derivar estado que pueda calcularse directamente.
- No introduzcas `useMemo` o `useCallback` indiscriminadamente; utilízalos cuando exista una razón concreta.
- Define correctamente las dependencias de los efectos.
- Limpia listeners, suscripciones, temporizadores y recursos cuando corresponda.
- Evita condiciones de carrera y actualizaciones de estado obsoletas.
- Usa identificadores estables en listas; no uses índices como claves si la colección puede cambiar de orden.
- Representa correctamente los estados de carga, vacío, error y éxito.
- Evita que los componentes acumulen lógica de acceso a datos, negocio y presentación.

### 7.4. Calidad de la interfaz

- Mantén una interfaz coherente, accesible y adaptable a diferentes tamaños de ventana.
- Utiliza HTML semántico, etiquetas accesibles y navegación por teclado.
- Respeta el foco, los estados de interacción y los mensajes de error.
- Añade atributos ARIA únicamente cuando sean necesarios.
- Evita regresiones visuales en los componentes existentes.
- Mantén las proporciones, el recorte y la posición de imágenes y pósteres de acuerdo con el diseño establecido.
- No cambies el diseño o el comportamiento visual fuera del alcance solicitado.
- No añadas dependencias para resolver problemas que puedan solucionarse correctamente con CSS, React o las herramientas existentes.

### 7.5. Asincronía y servicios

- Centraliza las llamadas a comandos de Tauri en la capa de servicios.
- Tipifica entradas, respuestas y errores.
- Gestiona correctamente las operaciones asíncronas y su cancelación cuando proceda.
- Evita llamadas duplicadas, condiciones de carrera y actualizaciones de componentes desmontados.
- No dependas de APIs de Node.js disponibles únicamente en entornos de servidor.
- No expongas tokens, rutas sensibles ni información interna en mensajes visibles al usuario.

## 8. Base de datos y sistema de archivos

### SQLite

- Utiliza siempre consultas parametrizadas.
- Nunca construyas consultas con concatenación de valores procedentes del usuario.
- Valida identificadores y fragmentos SQL dinámicos mediante listas permitidas cuando sean necesarios.
- Gestiona las transacciones y los errores de forma explícita.
- Comprueba índices y planes de consulta si hay problemas de rendimiento.
- Evita consultas N+1 y lecturas repetidas innecesarias.
- Respeta las garantías de concurrencia y las limitaciones de SQLite.
- Configura WAL y FTS5 según los requisitos reales de la aplicación.
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

- Versiona los cambios del esquema.
- Prueba las migraciones desde bases de datos antiguas y desde instalaciones limpias.
- Verifica que una migración fallida no deje datos parcialmente modificados.
- Conserva los datos existentes.
- Documenta cualquier incompatibilidad inevitable y cómo se gestiona.

## 9. Seguridad de Tauri y de la aplicación

La aplicación debe seguir el principio de mínimo privilegio.

- Configura los permisos y capabilities de Tauri estrictamente según las necesidades reales.
- No concedas acceso general al sistema de archivos, shell o procesos si basta con una operación limitada.
- Valida en Rust las entradas recibidas desde el frontend.
- No consideres el frontend una frontera de seguridad confiable.
- No construyas comandos de shell con datos de usuario.
- Evita ejecutar procesos externos; cualquier excepción debe estar justificada y restringida.
- No expongas rutas arbitrarias, operaciones de borrado ni ejecución de comandos a través de interfaces genéricas.
- Protege contra traversal de directorios y accesos no autorizados.
- Evita XSS y la inserción de HTML no confiable.
- No desactives mecanismos de seguridad de la WebView para resolver problemas de compatibilidad.
- No incluyas secretos, tokens, contraseñas o datos personales en el código, los logs o los commits.
- Utiliza `.env.example` para documentar variables de ejemplo sin credenciales reales.
- Mantén las dependencias actualizadas de forma controlada.
- Investiga las alertas de seguridad y no las ignores automáticamente.

Si una funcionalidad requiere privilegios adicionales, explica la necesidad y utiliza el mecanismo más restrictivo posible.

## 10. Política obligatoria de pruebas

Cada cambio debe incluir pruebas proporcionales al riesgo y al comportamiento modificado.

No basta con comprobar que el proyecto compila. Los tests deben demostrar que la funcionalidad se comporta correctamente y que los casos de error están controlados.

### 10.1. Tests unitarios

Comprueba, según corresponda:

- Casos normales y límites.
- Valores vacíos, inválidos y ausentes.
- Errores y recuperación.
- Transformaciones de datos.
- Reglas de negocio.
- Estados de carga, error y éxito.
- Funciones puras y utilidades.
- Parsing y validación de datos.

Los tests deben ser deterministas, rápidos, aislados y fáciles de entender.

### 10.2. Tests de integración

Comprueba la interacción real entre módulos:

- Repositorios y SQLite.
- Transacciones y migraciones.
- Servicios y lógica de dominio.
- Comandos de Tauri y contratos de entrada/salida.
- Lectura de archivos y metadatos.
- Configuración YAML.
- Manejo de errores y permisos.

Utiliza una base de datos y directorios temporales independientes por test. No dependas de la biblioteca personal del usuario ni de datos reales de su equipo.

### 10.3. Tests frontend

Utiliza Vitest y las herramientas ya presentes en el proyecto para probar:

- Renderizado y estados de componentes.
- Interacciones del usuario.
- Búsqueda, filtros y ordenación.
- Servicios y comandos invocados.
- Manejo de errores y datos ausentes.
- Accesibilidad y navegación por teclado en flujos relevantes.
- Regresiones de comportamiento en componentes existentes.

Prefiere comprobar el comportamiento observable en lugar de los detalles internos de implementación.

### 10.4. Tests de regresión

Cuando se corrija un bug:

1. Reproduce el problema.
2. Añade un test que falle antes de la corrección cuando sea viable.
3. Implementa el cambio mínimo correcto.
4. Comprueba que el test ahora pasa.
5. Ejecuta las pruebas relacionadas para descartar regresiones.

No elimines, desactives ni debilites un test porque revele un fallo de la implementación.

### 10.5. Tests E2E

Para los flujos críticos, comprueba la aplicación integrada cuando la infraestructura lo permita:

- Inicio de la aplicación.
- Carga de la biblioteca.
- Búsqueda y navegación.
- Apertura de los detalles de un elemento.
- Gestión de imágenes y pósteres.
- Reproducción y cierre del reproductor cuando estén implementados.
- Persistencia entre reinicios.
- Manejo de errores de datos y archivos.
- Cierre correcto de la aplicación.

Si el E2E requiere hardware, codecs, servicios o una sesión gráfica no disponible, ejecuta las verificaciones alternativas posibles y documenta exactamente la limitación. No declares que una prueba se ha superado si no se ha ejecutado.

### 10.6. Cobertura

- Mantén o mejora la cobertura de las rutas importantes.
- Define umbrales de cobertura cuando exista una configuración adecuada.
- No persigas un porcentaje elevado mediante tests triviales que no comprueban comportamientos reales.
- Prioriza las reglas de negocio, persistencia, seguridad, manejo de errores y flujos críticos.
- No reduzcas los umbrales existentes sin justificación y revisión explícitas.

### 10.7. Calidad de los propios tests

- Evita tests frágiles basados en detalles internos.
- No uses sleeps arbitrarios cuando puedan emplearse condiciones o sincronización determinista.
- No hagas depender un test del orden de ejecución de otro.
- No ocultes errores con mocks que simulan un éxito imposible.
- Utiliza mocks para límites externos cuando sean útiles, pero conserva tests de integración con dependencias reales cuando sean importantes.
- No introduzcas dependencias de red en la suite ordinaria si no son necesarias.
- Los tests que requieren recursos externos deben identificarse claramente.

## 11. Puertas de calidad antes de confirmar cambios

Antes de hacer un commit, ejecuta las comprobaciones pertinentes para las capas modificadas.

### Frontend

Desde la raíz del proyecto, utiliza los scripts que realmente existan:

```bash
npm run typecheck
npm run lint
npm test
npm run build
```

Si el proyecto dispone de `npm run verify`, execútalo también. Comprueba su definición antes de asumir qué incluye.

No des por hecho que todos los scripts existen. Si falta una comprobación necesaria, utiliza el comando equivalente disponible o documenta lo que falta. Añade scripts permanentes cuando sea parte del alcance y exista una justificación clara.

### Backend Rust

Desde `src-tauri/`:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo build --all-features
```

Ajusta los comandos si existen características incompatibles entre sí, targets específicos o dependencias opcionales que requieran otro entorno. No omitas una verificación silenciosamente.

### Aplicación completa

Cuando corresponda, comprueba también:

```bash
npm run tauri build
```

Utiliza el comando y el entorno adecuados para la configuración real del proyecto.

Si se han modificado dependencias, ejecuta las comprobaciones de seguridad pertinentes, por ejemplo:

```bash
npm audit
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

## 12. Git, commits y protección de cambios

### 12.1. Ramas

- Nunca trabajes directamente en `main`.
- Utiliza ramas específicas por funcionalidad, corrección o tarea técnica.
- Prefiere los prefijos `feat/`, `fix/`, `refactor/`, `test/`, `docs/` y `chore/`.
- Actualiza la rama de trabajo respecto a `main` cuando corresponda y resuelve conflictos de forma explícita.
- No sobrescribas trabajo de terceros.

Ejemplo:

```text
feat/hito-2-capa-dominio-rust
fix/poster-rendering
test/sqlite-migrations
```

### 12.2. Commits

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

### 12.3. No falsificar verificaciones

Nunca afirmes que se han ejecutado o superado tests, compilaciones, revisiones o checks si no existe evidencia de ello.

Distingue entre:

- Ejecutado y superado.
- Ejecutado y fallido.
- No ejecutado.
- Bloqueado por el entorno.
- No aplicable, con justificación.

No ocultes fallos mediante redirecciones, `|| true`, desactivación de reglas o eliminación de tests.

## 13. Política obligatoria de Pull Requests

Toda funcionalidad o corrección que llegue a `main` debe pasar por una Pull Request, salvo que exista una política de repositorio aprobada que establezca otra cosa.

### 13.1. Antes de abrir la PR

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

### 13.2. Revisión técnica de la PR

Antes de fusionar, revisa la PR como si fueras un revisor independiente. No te limites a leer el mensaje del commit ni a comprobar que el código compila.

Inspecciona:

- Bugs lógicos y casos límite.
- Regresiones respecto al comportamiento anterior.
- Seguridad, permisos y validación de entradas.
- Errores de ownership, concurrencia y gestión de recursos en Rust.
- Contratos inconsistentes entre TypeScript y Rust.
- Problemas de asincronía y estado en React.
- Consultas SQL inseguras y migraciones destructivas.
- Manejo incompleto de errores.
- Fugas de memoria o recursos nativos.
- Rendimiento y uso excesivo de recursos.
- Accesibilidad y regresiones visuales.
- Tests ausentes, débiles, frágiles o que no verifican el comportamiento real.
- Dependencias innecesarias o vulnerables.
- Documentación obsoleta.
- Cambios accidentales fuera del alcance.

Utiliza `gh pr diff`, `gh pr view` y las herramientas de revisión disponibles para examinar el cambio real. Si hay comentarios de revisión o resultados de herramientas automáticas, analízalos y resuelve los problemas relevantes.

No marques un comentario como resuelto hasta haber comprobado su corrección.

### 13.3. Checks de GitHub

Antes de fusionar:

1. Comprueba que la PR apunta a la rama de destino correcta.
2. Comprueba que todos los checks obligatorios han finalizado.
3. Confirma que los checks requeridos están en verde.
4. Inspecciona los fallos de CI aunque sean intermitentes.
5. Confirma que la rama está actualizada según la política del repositorio.
6. Revisa los comentarios de revisión y las conversaciones pendientes.
7. Comprueba que no se han añadido cambios inesperados desde la última revisión.
8. Si se han modificado archivos después de la revisión o de los tests, repite las verificaciones afectadas y revisa el diff actualizado.

Comandos orientativos:

```bash
gh pr view <PR> --json state,baseRefName,headRefName,mergeStateStatus,reviewDecision,statusCheckRollup
gh pr checks <PR>
gh pr diff <PR>
gh pr view <PR> --comments
```

Adapta los campos a la versión de GitHub CLI disponible. Si un comando no permite verificar un requisito, utiliza otra forma de obtener la evidencia.

Un check pendiente, fallido, obligatorio ausente o cuyo resultado no pueda determinarse no equivale a un check superado.

### 13.4. Condiciones de merge

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

### 13.5. Después del merge

- Comprueba que GitHub confirma el merge.
- Actualiza `main` localmente de forma segura.
- Verifica que el commit fusionado corresponde a la PR esperada.
- Comprueba el estado de CI de `main`.
- Verifica el estado del árbol de trabajo.
- Actualiza `PLAN_DESKTOP.md` y `README.md` cuando corresponda.
- Comprueba que la PR y el hito han quedado correctamente documentados.

No empieces el siguiente hito hasta que el actual esté fusionado y sus verificaciones requeridas estén completas.

## 14. Integración continua obligatoria

La calidad no debe depender exclusivamente de la memoria del agente. Siempre que el repositorio lo permita, implementa y mantiene una CI reproducible en GitHub Actions.

La CI debe ejecutarse en Pull Requests y en las ramas relevantes, como `main`.

Como mínimo, debe comprobar:

- Instalación reproducible de dependencias.
- Formato de Rust.
- Clippy sin advertencias no justificadas.
- Tests unitarios y de integración de Rust.
- Typecheck de TypeScript.
- Lint del frontend.
- Tests de Vitest.
- Build de producción del frontend.
- Build de Tauri cuando el runner disponga de las dependencias nativas necesarias.
- Auditoría de dependencias cuando esté configurada de forma estable.

Añade pruebas E2E y builds de distribución en Linux cuando sean viables y aporten cobertura relevante.

Buenas prácticas de CI:

- Fija versiones de acciones y herramientas de forma controlada.
- Usa `npm ci` cuando exista un lockfile compatible.
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

## 15. Estrategia de rendimiento

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
- Decodificación de pósteres.
- Memoria y copias innecesarias.
- Comunicación entre Rust y TypeScript.
- Operaciones nativas de reproducción.

Evita bloquear la interfaz, cargar colecciones completas cuando no sea necesario o repetir consultas costosas en cada renderizado.

No añadas concurrencia, cachés o sistemas de virtualización sin demostrar que resuelven un problema real o un requisito del proyecto.

## 16. Documentación y decisiones técnicas

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

## 17. Política de dependencias

Antes de añadir o actualizar una dependencia:

- Comprueba si ya existe una alternativa adecuada en el proyecto.
- Evalúa mantenimiento, compatibilidad, licencias y avisos de seguridad.
- Revisa el impacto en el tamaño del binario y los requisitos del sistema.
- Comprueba las versiones compatibles con el toolchain.
- Añade únicamente las dependencias necesarias.
- Actualiza el lockfile correspondiente.
- Ejecuta los tests y builds pertinentes.
- Documenta los cambios relevantes.

No ejecutes actualizaciones generales de dependencias dentro de una tarea funcional sin justificar su necesidad.

No elimines dependencias basándote únicamente en una búsqueda textual; verifica su uso indirecto, los scripts y la configuración de compilación.

## 18. Política de alcance y cambios visuales

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

## 19. Política de commits, PR y autonomía

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

## 20. Criterios de finalización de una tarea

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

## 21. Reglas finales no negociables

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

**Principio rector:** una funcionalidad no está terminada porque funcione en un caso concreto; está terminada cuando su comportamiento es correcto, sus errores están controlados, sus pruebas aportan confianza, sus cambios han sido revisados y la integración en `main` cumple las garantías del proyecto.
