# ADR 0008 — Publicación npm y distribución standalone

- **Estado:** sustituida parcialmente por el [ADR 0009](./0009-distribucion-github-releases.md)
- **Fecha:** 2026-10-22

> La publicación en npm queda sustituida por la distribución mediante GitHub Releases
> (ADR 0009). Siguen vigentes el nombre del paquete, el binario `mm` y el formato del
> bundle portable.

## Contexto

La release `v1.0.0` necesita un nombre público, un canal de publicación y una alternativa
para usuarios sin Node. El identificador `media-manager` ya está ocupado en npm por otro
proyecto, y `better-sqlite3` incluye un addon nativo que no encaja en un único ejecutable
de Node SEA. Además, los eventos creados por `release-please` con `GITHUB_TOKEN` no
disparan otros workflows, lo que afecta a cómo se encadena la publicación.

## Decisión

### Nombre y binario

- El paquete npm se llama **`media-manager-cli`**.
- El binario sigue siendo **`mm`**; el nombre corto pertenece al comando, no al paquete.
- El repositorio y la aplicación conservan el nombre `media-manager`.

Se descartaron `media-manager` (ocupado), `media-manager-terminal` y `mm-media-manager`
porque no aportaban una ventaja clara frente a un nombre que dice exactamente qué es.

### Publicación

- `package.json` declara `publishConfig.access: "public"` y `files` como lista blanca.
- `prepack` compila `dist/`; el tarball se prueba con `npm run smoke` (`npm pack` →
  instalación limpia → `--version`, `config --validate`, `index --dry-run`).
- La publicación usa `npm publish --provenance --access public` con `NPM_TOKEN` y
  `id-token: write`. No se publica desde una rama ni con el token en el repositorio.
- La primera `v1.0.0` se prepara a mano (tag + GitHub Release) para fijar la versión.
  Después, release-please gestiona `CHANGELOG.md`, el bump y el tag.
- `release.yml` (evento `release: published`) y `release-please.yml` (release creada por
  el bot) delegan ambos en el workflow reutilizable `publish.yml`. Así la publicación no
  depende de que un evento generado con `GITHUB_TOKEN` vuelva a disparar workflows.

### Distribución standalone

`npm run standalone` genera una **carpeta portable** con el runtime de Node y las
dependencias de producción, más un `.tar.gz`, para la plataforma y arquitectura actuales.
No es un único ejecutable: el addon de `better-sqlite3` debe permanecer en disco. Se
descarta Node SEA para no añadir un proceso frágil de extracción/carga de `.node`; la
alternativa portable es opcional y no se publica en npm.

## Consecuencias

- El comando instalado sigue siendo familiar (`mm`) aunque el paquete tenga otro nombre.
- La procedencia de npm queda ligada al repositorio y al commit del tag.
- El tarball y el contenido publicado se validan antes de cada release.
- release-please no necesita un PAT solo para publicar, pero sí `NPM_TOKEN` para npm.
- El binario standalone es específico de plataforma y más pesado que el paquete npm;
  se ofrece como adjunto de la GitHub Release, no como reemplazo del paquete.
- Mientras no exista `NPM_TOKEN`, el tag y la release quedan pendientes; el resto de la
  preparación puede mergearse sin publicar nada.
