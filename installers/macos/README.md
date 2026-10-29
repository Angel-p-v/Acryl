# macOS - Sin assets oficiales en v3.1.0

## Estado

**No hay assets oficiales de macOS en v3.1.0.**

## Razón

El autor del proyecto no tiene acceso a hardware macOS, y los runners de GitHub Actions tienen capacidad limitada (exit code 127), por lo que no es posible construir y verificar los bundles para macOS.

## Alternativas para usuarios de macOS

### Opción 1: Compilar localmente

En un Mac con Node.js instalado:

```bash
git clone https://github.com/Angel-p-v/media-manager.git
cd media-manager
npm ci
npm run standalone -- --node-source official
```

Esto generará un bundle portable en `build/standalone/` que puedes usar en macOS.

### Opción 2: Usar binario Linux vía Rosetta 2 / Docker

Puedes intentar ejecutar el bundle de Linux x64 en macOS mediante Rosetta 2 o contenedores Docker, aunque no está oficialmente soportado.

## Plataformas soportadas oficialmente

- **Linux** (x64, arm64) — assets oficiales con checksums, SBOM y atestaciones
- **Windows** (x64) — instalador PowerShell con verificación de checksums

## Futuro

El soporte oficial para macOS se reconsiderará cuando haya acceso a hardware para construir y verificar los bundles.
