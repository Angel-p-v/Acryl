# Instaladores de media-manager

Este directorio contiene los instaladores y desinstaladores de `media-manager-cli` (`mm`), organizados por sistema operativo.

## Estructura

```
installers/
├── linux/          # Instalador POSIX para Linux (x64, arm64)
│   ├── install.sh
│   └── uninstall.sh
├── windows/        # Instalador PowerShell para Windows (x64)
│   ├── install.ps1
│   └── uninstall.ps1
└── macos/          # macOS no tiene assets oficiales en v3.1.0
    └── README.md
```

## Uso rápido

### Linux (x64, arm64)

#### Opción A: desde repositorio clonado (recomendado para desarrolladores)

```bash
git clone https://github.com/Angel-p-v/media-manager.git
cd media-manager
npm ci
sh installers/linux/install.sh
```

Requiere `node` y `npm` instalados. El instalador usa `dist/` y `node_modules/` del repositorio clonado.

#### Opción B: script suelto (release precompilado)

```bash
curl -fsSL https://raw.githubusercontent.com/Angel-p-v/media-manager/main/installers/linux/install.sh -o /tmp/mm-install.sh
sh /tmp/mm-install.sh
```

Descarga un release precompilado de GitHub Releases, verifica el checksum SHA-256 y no requiere `git`, `npm` ni herramientas de compilación.

### Windows (x64)

```powershell
git clone https://github.com/Angel-p-v/media-manager.git
cd media-manager
.\installers\windows\install.ps1
```

### macOS

**No hay assets oficiales de macOS en v3.1.0.** Consulta [`installers/macos/README.md`](./macos/README.md) para más información.

## Desinstalar

```bash
# Linux
sh installers/linux/uninstall.sh            # conserva configuración y base de datos
sh installers/linux/uninstall.sh --purge    # las elimina también
```

## Notas

- Los instaladores no requieren `sudo`; instalan en `~/.local` por defecto.
- Añade `~/.local/bin` a tu PATH para usar `mm` sin ruta absoluta.
- La desinstalación conserva configuración y base de datos salvo que se use `--purge`.
- La biblioteca multimedia nunca se toca.
- En modo repositorio clonado, el repo debe permanecer en su lugar (el instalador crea un symlink a `node_modules/`).
