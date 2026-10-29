# Publicación y release

`media-manager-cli` se distribuye **solo desde GitHub Releases**. No se publica en npm.
Cada release adjunta un bundle portable por plataforma, su checksum SHA-256, un SBOM
CycloneDX y una atestación de procedencia.

La decisión está en el [ADR 0009](../adr/0009-distribucion-github-releases.md). El detalle
del plan y las pruebas está en
[`docs/plans/instaladores-y-seguridad.md`](../plans/instaladores-y-seguridad.md).

## Releases publicadas

| Versión  | Fecha      | Estado                                                                |
| -------- | ---------- | --------------------------------------------------------------------- |
| `v1.0.1` | 2026-10-23 | Actual; corrige la expansión de `~` en los instaladores.              |
| `v1.0.0` | 2026-10-23 | Primera release; su instalador llevaba el fallo de `~` en `--prefix`. |

`v1.0.0` se creó a mano y `v1.0.1` la generó release-please. El job de instalación en
contenedor limpio de `v1.0.0` falló con `/rootoot/.local/bin/mm`: el patrón `~` del `case`
se expandía a `$HOME` antes de comparar, de modo que un prefijo dentro de HOME acababa en
una ruta corrupta. No se movió el tag publicado; la corrección salió como `v1.0.1`, cuyos
bundles, checksums, SBOM, atestaciones e instalación limpia pasaron. Desde entonces ese job
bloquea la release si el instalador no funciona.

## Artefactos

```text
media-manager-cli-1.0.1-linux-x64.tar.gz
media-manager-cli-1.0.1-linux-x64.tar.gz.sha256
media-manager-cli-1.0.1-linux-arm64.tar.gz
media-manager-cli-1.0.1-linux-arm64.tar.gz.sha256
sbom.cdx.json
```

- El bundle incluye `dist/`, dependencias de producción, el runtime oficial de Node, la
  configuración de ejemplo, documentación, `install.json` y `THIRD_PARTY_NOTICES.md`.
- La versión del runtime está fijada en `scripts/build/node-version.txt`.
- El `.sha256` usa el formato de `sha256sum -c` y `shasum -a 256 -c`.
- La atestación se genera con `actions/attest-build-provenance` y se verifica con `gh`.

Plataformas iniciales: **Linux x64** y **Linux arm64** sobre glibc. macOS y Windows
quedan para una fase posterior.

## Requisitos

- `main` verde en Node 22, 24 y 26.
- Permisos del `GITHUB_TOKEN` declarados en los workflows: `contents: write` para subir
  assets, `id-token: write` y `attestations: write` para la atestación.
- No hace falta ningún secreto de npm ni `NPM_TOKEN`.

## Flujo automático (release-please)

1. Los commits convencionales (`feat`, `fix`, `perf`, `deps`, …) llegan a `main`.
2. `release-please.yml` abre una PR de release con `CHANGELOG.md` y el bump de versión.
3. Al mergearla, release-please crea el tag y la GitHub Release.
4. El mismo workflow llama a `release-artifacts.yml`, que:
   - construye el bundle en la matriz `ubuntu-24.04` (x64) y `ubuntu-24.04-arm` (arm64);
   - ejecuta `npm run verify`;
   - descarga el Node oficial y verifica su `SHASUMS256.txt`;
   - genera el `.sha256` y comprueba el artefacto;
   - atesta la procedencia;
   - sube los assets con `gh release upload` (sin `--clobber`);
   - genera y sube el SBOM en un job aparte.
5. El evento `release: published` también llama al workflow, por lo que una release creada
   a mano sigue el mismo camino.

`release-please` invoca el workflow directamente porque los eventos generados con
`GITHUB_TOKEN` no vuelven a disparar otros workflows.

## Primera release (`v1.0.0`)

> `v1.0.0` ya está publicada y fue la primera release manual. El flujo normal desde
> `v1.0.1` es el automático de release-please; esta sección queda como referencia
> histórica del arranque.

El repositorio parte de `1.0.0` en `package.json`, `package-lock.json`,
`CHANGELOG.md`, `release-please-config.json` y `.release-please-manifest.json`.

1. Comprueba que `main` está limpio y con la CI verde.
2. Crea y sube el tag anotado:

   ```bash
   git tag -a v1.0.0 -m "media-manager-cli v1.0.0"
   git push origin v1.0.0
   ```

3. Crea la GitHub Release:

   ```bash
   gh release create v1.0.0 --title "v1.0.0" --notes-file CHANGELOG.md
   ```

4. Espera a que `release-artifacts.yml` termine y verifica los assets:

   ```bash
   gh release view v1.0.0
   gh attestation verify media-manager-cli-1.0.0-linux-x64.tar.gz \
     --repo Angel-p-v/media-manager
   ```

5. Instala desde la release en un entorno limpio (sin Node) y comprueba:

   ```bash
   sh install.sh --version v1.0.0
   mm --version
   mm config --validate
   ```

## Construcción local

El runtime oficial es obligatorio para publicar:

```bash
npm ci
npm run verify
npm run standalone -- --node-source official
sha256sum -c build/standalone/media-manager-cli-*-linux-x64.tar.gz.sha256
```

En desarrollo se puede usar el Node del sistema:

```bash
npm run standalone -- --node-source system
```

El resultado queda en `build/standalone/`. El script genera el `.tar.gz`, su `.sha256` y
un `install.json` con la versión y la plataforma.

Para probar el instalador contra un artefacto local:

```bash
sh install.sh \
  --file build/standalone/media-manager-cli-1.0.0-linux-x64.tar.gz \
  --sha256 build/standalone/media-manager-cli-1.0.0-linux-x64.tar.gz.sha256 \
  --prefix /tmp/mm-test --no-path-check
```

## Verificación de una release

```bash
# Checksum
sha256sum -c media-manager-cli-1.0.1-linux-x64.tar.gz.sha256

# Procedencia (requiere gh autenticado)
gh attestation verify media-manager-cli-1.0.1-linux-x64.tar.gz \
  --repo Angel-p-v/media-manager

# Contenido del bundle
tar -tzf media-manager-cli-1.0.1-linux-x64.tar.gz | head
```

La atestación mejora la integridad, pero la raíz de confianza sigue siendo la cuenta de
GitHub que publica la release. El checksum y el asset viven en el mismo release; usa la
atestación cuando necesites vincular el artefacto al workflow que lo construyó.

## Si algo sale mal

- **El workflow falla al descargar Node:** comprueba `scripts/build/node-version.txt` y que
  la versión exista en `nodejs.org/dist`. La descarga se verifica contra
  `SHASUMS256.txt`; un fallo de checksum aborta la construcción.
- **`gh release upload` falla porque el asset existe:** el workflow no sobrescribe assets.
  Corrige el problema y publica una versión nueva; no reutilices el tag.
- **El checksum no coincide:** no instales el artefacto. Vuelve a construir y publicar.
- **La instalación acaba en una ruta tipo `/rootoot`:** era el bug de expansión de `~` de
  `v1.0.0`, corregido en `v1.0.1`. Usa el `install.sh` de `main` y una versión ≥ 1.0.1.
- **Un tag apunta a un commit incorrecto:** no lo muevas si ya tiene una release. Crea una
  versión nueva.
- **Hay que retirar una versión:** publica una corrección y, si hace falta, marca la
  release anterior como _pre-release_ o edita sus notas; no borres assets de una release
  que los usuarios ya hayan descargado.
- **El instalador rechaza un artefacto:** revisa que `install.json` tenga el nombre,
  versión y plataforma correctos y que el archivo no contenga rutas `../` o absolutas.
