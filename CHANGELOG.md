# Changelog

Todos los cambios notables de este proyecto se documentan en este archivo.

El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y el
versionado [SemVer](https://semver.org/lang/es/). A partir de `v1.0.0`, release-please
mantiene este archivo a partir de los commits convencionales.

## [3.3.0](https://github.com/Angel-p-v/media-manager/compare/v3.2.0...v3.3.0) (2026-10-27)


### Features

* **config:** obtención automática de metadatos al abrir la TUI ([#210](https://github.com/Angel-p-v/media-manager/issues/210)) ([fc17a64](https://github.com/Angel-p-v/media-manager/commit/fc17a64934808eef06700f4e4b112946494c435a))
* **deps:** instalar chafa automáticamente para renderizar pósters ([#226](https://github.com/Angel-p-v/media-manager/issues/226)) ([cba8924](https://github.com/Angel-p-v/media-manager/commit/cba8924b6bb84cc759815ebf0ed206c893363ad0))
* **indexer:** implementar plan 12 - clasificación de archivos y metadatos desde cero ([#207](https://github.com/Angel-p-v/media-manager/issues/207)) ([87239b7](https://github.com/Angel-p-v/media-manager/commit/87239b7a3c16500760b6b8d849046ce0342097dd))
* **ui:** codec de vídeo, selección de subtítulos con Mayús+flechas, indexado al guardar ajustes ([#218](https://github.com/Angel-p-v/media-manager/issues/218)) ([c88e613](https://github.com/Angel-p-v/media-manager/commit/c88e613dcc5bb83fdb90d9f5042c7e74e10ee0ac))
* **ui:** correcciones de UX, metadatos y búsqueda ([#220](https://github.com/Angel-p-v/media-manager/issues/220)) ([f005a03](https://github.com/Angel-p-v/media-manager/commit/f005a03c6688bf82198975165408c2f8600e4428))
* **ui:** gestor de metadatos automático con proveedor por defecto ([#221](https://github.com/Angel-p-v/media-manager/issues/221)) ([626014c](https://github.com/Angel-p-v/media-manager/commit/626014cb2f8251163d1bc9d80112af1b1bf40a0c))
* **ui:** idiomas de audio/subs desde los nombres de fichero ([#215](https://github.com/Angel-p-v/media-manager/issues/215)) ([8df72fd](https://github.com/Angel-p-v/media-manager/commit/8df72fd8adc740b4e4a9cb3209e25ba4dd700e4d))
* **ui:** leer metadatos reales de vídeo (audio y subtítulos) con ffprobe ([#217](https://github.com/Angel-p-v/media-manager/issues/217)) ([3eb1f69](https://github.com/Angel-p-v/media-manager/commit/3eb1f69431cf7ef74deac15b71051b6c9bb2296f))
* **ui:** mejorar vista de detalles de reproducción (play-details) ([#214](https://github.com/Angel-p-v/media-manager/issues/214)) ([bdc4a51](https://github.com/Angel-p-v/media-manager/commit/bdc4a5162bc810665adf0f696922c87d7b506451))
* **ui:** paginación de 12 episodios y navegación de pistas en play-details ([#216](https://github.com/Angel-p-v/media-manager/issues/216)) ([a26cfd4](https://github.com/Angel-p-v/media-manager/commit/a26cfd41668c58b05aa344569fa4bb8555c191f4))
* **ui:** pegar texto en campos de ajustes (Ctrl+V / clic derecho) ([#227](https://github.com/Angel-p-v/media-manager/issues/227)) ([9759a21](https://github.com/Angel-p-v/media-manager/commit/9759a21e4abd3c8ade9964e2a35d6c14b2b1f8e6))
* **ui:** renderizar imágenes en terminal con chafa (pósters de metadatos) ([#225](https://github.com/Angel-p-v/media-manager/issues/225)) ([6ace382](https://github.com/Angel-p-v/media-manager/commit/6ace382bad72eb078a2378bb0275012d117bbd97))
* **ui:** vista de detalles de reproducción tipo Jellyfin ([#213](https://github.com/Angel-p-v/media-manager/issues/213)) ([68c39ff](https://github.com/Angel-p-v/media-manager/commit/68c39ff2bcb34109eacdbfecd750c880c93d11de))


### Bug Fixes

* **indexer:** detectar episodios con sufijos tipo v2 (S01E01v2) ([#209](https://github.com/Angel-p-v/media-manager/issues/209)) ([ece374f](https://github.com/Angel-p-v/media-manager/commit/ece374f1e37b2f14a0c8bec3dc07d562f6f5dace))
* **ui:** AniList sin API key, portadas y metadatos para series ([#224](https://github.com/Angel-p-v/media-manager/issues/224)) ([39bb965](https://github.com/Angel-p-v/media-manager/commit/39bb9659d90ee1ef177f07b44270991500dec02f))
* **ui:** eliminar códec duplicado en play-details y título vacío en metadatos ([#222](https://github.com/Angel-p-v/media-manager/issues/222)) ([f8f9eb0](https://github.com/Angel-p-v/media-manager/commit/f8f9eb0624394bd4ef53b8be1e5b158c232de4ae))

## [3.2.0](https://github.com/Angel-p-v/media-manager/compare/v3.1.0...v3.2.0) (2026-10-26)


### Features

* **cli:** añadir opción --set al comando mm config ([#198](https://github.com/Angel-p-v/media-manager/issues/198)) ([f9453ca](https://github.com/Angel-p-v/media-manager/commit/f9453ca5785255902d289fa8b56f53b9907a2d0c))
* **cli:** clonar repositorio en instalador y crear config.yaml por defecto ([#190](https://github.com/Angel-p-v/media-manager/issues/190)) ([0efba11](https://github.com/Angel-p-v/media-manager/commit/0efba114673f0e4b2fc61d58873780fce18d48b2))
* **cli:** comando mm playlist para reproducir varios títulos en secuencia (plan-9 R3) ([#178](https://github.com/Angel-p-v/media-manager/issues/178)) ([3a732b7](https://github.com/Angel-p-v/media-manager/commit/3a732b7e5a3ec4217c728f6580a418b7eec81e9a))
* **config:** proveedor AniList para metadatos de anime (plan-9 M2) ([#175](https://github.com/Angel-p-v/media-manager/issues/175)) ([d4b692a](https://github.com/Angel-p-v/media-manager/commit/d4b692a71ebd5f233d3ee9f428020b46059c120a))
* **plan-9:** Docker Compose con BD y config persistentes (D1) ([#171](https://github.com/Angel-p-v/media-manager/issues/171)) ([5df4437](https://github.com/Angel-p-v/media-manager/commit/5df4437e28215514278be5430a74c184383db457))
* **plan-9:** temas de color personalizables desde YAML (T1) ([#176](https://github.com/Angel-p-v/media-manager/issues/176)) ([fe2e764](https://github.com/Angel-p-v/media-manager/commit/fe2e764787a6064a443fc7d1dc8ec24177a22c1e))
* **player:** soporte para subtítulos (.srt) en mm play ([#174](https://github.com/Angel-p-v/media-manager/issues/174)) ([04a8ace](https://github.com/Angel-p-v/media-manager/commit/04a8ace17274c5ef2081161aac5d40db4682234e))
* **ui:** ajustes navegables y editables desde la TUI ([#189](https://github.com/Angel-p-v/media-manager/issues/189)) ([b79e0d3](https://github.com/Angel-p-v/media-manager/commit/b79e0d34b7ca75ddce41faef4b902e909579976c))
* **ui:** ajustes reflejan cambios inmediatamente y guardan sin config previa ([#200](https://github.com/Angel-p-v/media-manager/issues/200)) ([b7fb74a](https://github.com/Angel-p-v/media-manager/commit/b7fb74a80a0cc344c0bad3bc704fbfa98811697a))
* **ui:** añadir raíz de biblioteca desde ajustes y corregir retorno al editar ([#202](https://github.com/Angel-p-v/media-manager/issues/202)) ([f5e5bc0](https://github.com/Angel-p-v/media-manager/commit/f5e5bc0a9315ab110f1c528f8df1ce89493b8a40))
* **ui:** atajos de teclado configurables desde la TUI (plan-9 T2) ([#179](https://github.com/Angel-p-v/media-manager/issues/179)) ([88e2965](https://github.com/Angel-p-v/media-manager/commit/88e2965b7da9ad5f516200802cb5d43c94a29548))
* **ui:** editor de configuración desde la TUI ([#192](https://github.com/Angel-p-v/media-manager/issues/192)) ([11ce592](https://github.com/Angel-p-v/media-manager/commit/11ce592a34edd61fcff18145aaa5fbdcc7ff3257))


### Bug Fixes

* **installer:** reescribe install.sh con descarga de release precompilado y verificación de checksum ([#195](https://github.com/Angel-p-v/media-manager/issues/195)) ([d8e7f45](https://github.com/Angel-p-v/media-manager/commit/d8e7f45675238b27768758a979834d704f641534))
* **ui,config,cli:** corregir guardado de ajustes desde la TUI ([#197](https://github.com/Angel-p-v/media-manager/issues/197)) ([6159ca9](https://github.com/Angel-p-v/media-manager/commit/6159ca9b0ca4ef2717e560083ecd3ee520c425ea))
* **ui:** permite volver a biblioteca desde ajustes con Tab ([#188](https://github.com/Angel-p-v/media-manager/issues/188)) ([884301a](https://github.com/Angel-p-v/media-manager/commit/884301acb038dd58ead3dd000c4c41f07ad556a0))

## [3.1.0](https://github.com/Angel-p-v/media-manager/compare/v3.0.7...v3.1.0) (2026-10-25)


### Features

* **cli:** añade --dry-run en operaciones destructivas (mark, metadata clear) ([#147](https://github.com/Angel-p-v/media-manager/issues/147)) ([3f301e5](https://github.com/Angel-p-v/media-manager/commit/3f301e5d25091e957d1a59c3b41e6b290f60e2a8))
* **cli:** añade comando mm metadata cache --clear para limpiar caché de pósters ([#144](https://github.com/Angel-p-v/media-manager/issues/144)) ([8b2b947](https://github.com/Angel-p-v/media-manager/commit/8b2b94754adb36c30c1ef2073158fc7212e7273c))
* **cli:** añade fallback de póster local en metadatos ([#133](https://github.com/Angel-p-v/media-manager/issues/133)) ([09546b0](https://github.com/Angel-p-v/media-manager/commit/09546b0ec5eb153c7fd8e5d77722dc3025c96061))
* **cli:** añade mm db backup y mm db restore ([#126](https://github.com/Angel-p-v/media-manager/issues/126)) ([a73bdd1](https://github.com/Angel-p-v/media-manager/commit/a73bdd17091d170deca65b04c53cd1f5b94943d0))
* **cli:** añade mm doctor --full con información extendida ([#127](https://github.com/Angel-p-v/media-manager/issues/127)) ([0e3755d](https://github.com/Angel-p-v/media-manager/commit/0e3755d5992492dfdc39f725c6adc0622e8c1005))
* **cli:** añade proveedor secundario OMDB para metadatos ([#135](https://github.com/Angel-p-v/media-manager/issues/135)) ([cb2465e](https://github.com/Angel-p-v/media-manager/commit/cb2465ee1d3cf1a4de54fd7feb39ede77fdef8c6))
* **cli:** exportar biblioteca a JSON/CSV con --export ([#120](https://github.com/Angel-p-v/media-manager/issues/120)) ([7cdad65](https://github.com/Angel-p-v/media-manager/commit/7cdad6578deea8075e65834152c5bc7cf86a2b99))
* **config:** recarga library.exclude sin reiniciar (B3) ([#163](https://github.com/Angel-p-v/media-manager/issues/163)) ([3c49278](https://github.com/Angel-p-v/media-manager/commit/3c49278817eb7adf96b3eb98efa482f63d880c84))
* **config:** validar raíces de biblioteca al arrancar ([#121](https://github.com/Angel-p-v/media-manager/issues/121)) ([a73b7d6](https://github.com/Angel-p-v/media-manager/commit/a73b7d639df7878388a719eb32244529367d8a6c))
* **db:** caché local de imágenes/pósteres descargados ([#119](https://github.com/Angel-p-v/media-manager/issues/119)) ([776e967](https://github.com/Angel-p-v/media-manager/commit/776e967cb891bd411e7e4a50075e22b637da93c6))
* **deps:** añade Dockerfile para imagen oficial ([#129](https://github.com/Angel-p-v/media-manager/issues/129)) ([18520e9](https://github.com/Angel-p-v/media-manager/commit/18520e91b0c4ff144d017862a9364a5415009436))
* **distribution:** añade fórmulas Homebrew y manifiesto winget (G2)\n\n- media-manager-cli.rb: fórmula de Homebrew para macOS (pendiente de assets macOS)\n- media-manager-cli.yaml: manifiesto de winget para Windows (x64 y arm64)\n\n## Verificación\n- npm run verify: pasa (lint, format, typecheck, test)\n- 592 tests pasando\n\n## Orden recomendado\nA1-A4 ✅ → E1 ✅ → F1 ✅ → B1 ✅ → G1 ✅ → F3 ✅ → A3 ✅ → A4 ✅ → F2 ✅ → G2 ✅ (este PR)\nSiguiente: B2, B3, H1, I1, D1-D4 según prioridad ([#157](https://github.com/Angel-p-v/media-manager/issues/157)) ([9d8d26d](https://github.com/Angel-p-v/media-manager/commit/9d8d26d951f26f2823ef92c98d20a865aedea7b9))
* **i18n:** infraestructura de traducciones para TUI (H1) ([#167](https://github.com/Angel-p-v/media-manager/issues/167)) ([77ec1be](https://github.com/Angel-p-v/media-manager/commit/77ec1befe87e5c0f7b69246956ad56b4f7768b55))
* **indexer:** índice incremental por carpeta con --folder ([#118](https://github.com/Angel-p-v/media-manager/issues/118)) ([48be7bb](https://github.com/Angel-p-v/media-manager/commit/48be7bb59059e081bb60a73629012b84c3dc6e5f))
* **indexer:** índice incremental real por carpeta con --folder ([#128](https://github.com/Angel-p-v/media-manager/issues/128)) ([763b869](https://github.com/Angel-p-v/media-manager/commit/763b869320d2aea2507ce7ca5c65b20ce9e98fc8))
* **logs:** añade --log-file con rotación de logs (F2) ([#156](https://github.com/Angel-p-v/media-manager/issues/156)) ([6c65639](https://github.com/Angel-p-v/media-manager/commit/6c656390441eb4ae5b199908f643c356a40a33c8))
* **metrics:** añade módulo de métricas simples (F3)\n\n- src/core/metrics.ts: clase Metrics con contadores de indexaciones,\n  errores de metadatos y tiempo medio de consulta, más instancia\n  compartida para uso en CLI/TUI\n- tests/unit/core/metrics.test.ts: tests para todos los métodos\n  (recordIndexation, recordMetadataError, recordQueryTime, snapshot,\n  reset, tiempo negativo, promedio cero) ([#152](https://github.com/Angel-p-v/media-manager/issues/152)) ([a853f1d](https://github.com/Angel-p-v/media-manager/commit/a853f1ddfc1c4b588bca67b3ac40ef28cd944663))
* **tui:** filtros teclas, vista compacta, selección múltiple y ratón (D1-D4) ([#165](https://github.com/Angel-p-v/media-manager/issues/165)) ([3531915](https://github.com/Angel-p-v/media-manager/commit/353191521056ea6660b14e883e2631de8924ba16))
* **ui:** añade atajo m para navegar al panel de detalles (M18.5) ([#108](https://github.com/Angel-p-v/media-manager/issues/108)) ([6af1849](https://github.com/Angel-p-v/media-manager/commit/6af1849273375227344db2ce40af4ed6c1fddbd7))
* **ui:** M19 - rate-limiting y auto-refresco de metadatos expirados ([#110](https://github.com/Angel-p-v/media-manager/issues/110)) ([cc937e0](https://github.com/Angel-p-v/media-manager/commit/cc937e0932781241452e0e88d0b7fee20caca30e))
* **ui:** temas personalizables (claro/oscuro/por defecto) ([#117](https://github.com/Angel-p-v/media-manager/issues/117)) ([89a5d6a](https://github.com/Angel-p-v/media-manager/commit/89a5d6a8e1874f4d61033ff6b6d6584008cb7af4))
* **watcher:** detecta rename/move y actualiza rutas en BD (B2) ([#161](https://github.com/Angel-p-v/media-manager/issues/161)) ([2ebe184](https://github.com/Angel-p-v/media-manager/commit/2ebe1849fce410453d79665eff4c321516e6f817))


### Bug Fixes

* aplica plan fixes-seguridad (bugs B1-B4 y vulnerabilidades V1-V4) ([#142](https://github.com/Angel-p-v/media-manager/issues/142)) ([0208153](https://github.com/Angel-p-v/media-manager/commit/02081535d93afa3f1648639ae597a2f73b988fcc))
* **ci:** ajusta presupuesto de seed de estrés para ARM64 ([#114](https://github.com/Angel-p-v/media-manager/issues/114)) ([97fdd5c](https://github.com/Angel-p-v/media-manager/commit/97fdd5ceccc07f0a29c08a603e453879e089a25f))
* **ci:** añadir noEmitOnError a tsconfig.build.json ([#131](https://github.com/Angel-p-v/media-manager/issues/131)) ([1bbbd6c](https://github.com/Angel-p-v/media-manager/commit/1bbbd6cb7961c5ffafab7fe5669edffa17d68feb))
* **cli:** resuelve TODO de doctor.ts: rastrear estado real de migración ([#115](https://github.com/Angel-p-v/media-manager/issues/115)) ([2d12f0a](https://github.com/Angel-p-v/media-manager/commit/2d12f0a5fd43e0da1f664f747777c5f023ac0da8))
* **cli:** tipa catch como unknown en manejo de errores de caché de pósters ([#145](https://github.com/Angel-p-v/media-manager/issues/145)) ([c39fcc3](https://github.com/Angel-p-v/media-manager/commit/c39fcc3fcbc1132d27ab1719186aeb9663753c2f))
* **test:** subir cobertura de branches por encima del 80% ([#130](https://github.com/Angel-p-v/media-manager/issues/130)) ([013bedf](https://github.com/Angel-p-v/media-manager/commit/013bedfcf0b98abaa3e7ea544c0ab5c257ad64b2))

## [3.0.7](https://github.com/Angel-p-v/media-manager/compare/v3.0.6...v3.0.7) (2026-10-24)


### Bug Fixes

* **ci:** añade paréntesis a Test-Path en condicional de instalador Windows ([#103](https://github.com/Angel-p-v/media-manager/issues/103)) ([a7ed85c](https://github.com/Angel-p-v/media-manager/commit/a7ed85ca71dcec4e3065312c9405e179a3977b1d))
* **ci:** busca marcador en todo el wrapper mm.bat en uninstall.ps1 ([#105](https://github.com/Angel-p-v/media-manager/issues/105)) ([80bbb19](https://github.com/Angel-p-v/media-manager/commit/80bbb197322acfbb1ffffb6e0c1e849e2604191e))
* **ci:** reemplaza Test-Path -Container por Get-Item en instalador Windows ([#102](https://github.com/Angel-p-v/media-manager/issues/102)) ([44cfd03](https://github.com/Angel-p-v/media-manager/commit/44cfd03f0668d124e80834304aa18ba1d83a0382))
* **ci:** resuelve target relativo del symlink current en uninstall.ps1 ([#104](https://github.com/Angel-p-v/media-manager/issues/104)) ([894e626](https://github.com/Angel-p-v/media-manager/commit/894e62621d10dec5eed5582270aece325cd133da))
* **ci:** tolera ZIP plano y corrige estructura de bundles Windows ([#100](https://github.com/Angel-p-v/media-manager/issues/100)) ([301eea6](https://github.com/Angel-p-v/media-manager/commit/301eea64c60ea26bdc625d2407cceea847f6cde8))

## [3.0.6](https://github.com/Angel-p-v/media-manager/compare/v3.0.5...v3.0.6) (2026-10-24)


### Bug Fixes

* **ci:** compara checksum en minúsculas en install.ps1 de Windows ([#96](https://github.com/Angel-p-v/media-manager/issues/96)) ([01a9f79](https://github.com/Angel-p-v/media-manager/commit/01a9f797552ffaabbcb4556362de48ed333384d6))
* **ci:** compara hash real contra token correcto en install.ps1 de Windows ([#98](https://github.com/Angel-p-v/media-manager/issues/98)) ([e434dc2](https://github.com/Angel-p-v/media-manager/commit/e434dc22f99a2716016facc1cccb2bdaa96d1021))
* **ci:** usa tipo ZipFile correcto para PowerShell 7 en instalador Windows ([#99](https://github.com/Angel-p-v/media-manager/issues/99)) ([eca442f](https://github.com/Angel-p-v/media-manager/commit/eca442f8dc6ac94c2ac6a879ca27d6b767aa35c9))

## [3.0.5](https://github.com/Angel-p-v/media-manager/compare/v3.0.4...v3.0.5) (2026-10-24)


### Bug Fixes

* **ci:** compara checksum en minúsculas en install-test Windows ([#94](https://github.com/Angel-p-v/media-manager/issues/94)) ([1783979](https://github.com/Angel-p-v/media-manager/commit/178397945b9fedea8d33f71ca53e9925e3c99104))

## [3.0.4](https://github.com/Angel-p-v/media-manager/compare/v3.0.3...v3.0.4) (2026-10-24)


### Bug Fixes

* **ci:** corrige sintaxis de switch en instaladores Windows ([#91](https://github.com/Angel-p-v/media-manager/issues/91)) ([ca48c16](https://github.com/Angel-p-v/media-manager/commit/ca48c16101936fecd74e43d0001f6b9a8d3b6c89))
* **ci:** usa cláusulas separadas en switch de instaladores Windows ([#93](https://github.com/Angel-p-v/media-manager/issues/93)) ([655d3ab](https://github.com/Angel-p-v/media-manager/commit/655d3ab36ea95af2247fdf18b2a64452bed62b2b))

## [3.0.3](https://github.com/Angel-p-v/media-manager/compare/v3.0.2...v3.0.3) (2026-10-24)


### Bug Fixes

* **ci:** copia install.ps1 y uninstall.ps1 al tempDir en el install-test Windows ([#89](https://github.com/Angel-p-v/media-manager/issues/89)) ([d9936d2](https://github.com/Angel-p-v/media-manager/commit/d9936d264819aba32cd7a4d7624b4cbd7ae455e0))

## [3.0.2](https://github.com/Angel-p-v/media-manager/compare/v3.0.1...v3.0.2) (2026-10-24)


### Bug Fixes

* **ci:** usa rutas absolutas para install.ps1 y uninstall.ps1 en el install-test Windows ([#87](https://github.com/Angel-p-v/media-manager/issues/87)) ([5f62201](https://github.com/Angel-p-v/media-manager/commit/5f6220151ab220c53d0c61acf2120e2a444dd279))

## [3.0.1](https://github.com/Angel-p-v/media-manager/compare/v3.0.0...v3.0.1) (2026-10-24)


### Bug Fixes

* **ci:** busca install.json por nombre en el install-test Windows ([#85](https://github.com/Angel-p-v/media-manager/issues/85)) ([a1cf472](https://github.com/Angel-p-v/media-manager/commit/a1cf472978e7dd47a946e9f72468aa3c00a0f348))
* **ci:** usa nombre de tipo completo System.IO.Compression.ZipFile en install-test Windows ([#84](https://github.com/Angel-p-v/media-manager/issues/84)) ([350a919](https://github.com/Angel-p-v/media-manager/commit/350a919cc4dbcf848c7110f19ddd9fd1dc7a21f2))

## [3.0.0](https://github.com/Angel-p-v/media-manager/compare/v2.0.0...v3.0.0) (2026-10-24)


### ⚠ BREAKING CHANGES

* v2.0.0 consolida los hitos M9–M12:

### Features

* **cli:** auto-completado de shell (M20) ([#64](https://github.com/Angel-p-v/media-manager/issues/64)) ([fd73068](https://github.com/Angel-p-v/media-manager/commit/fd73068cb9fc40dcec73ce31e2961ad2f600e21f))
* **cli:** comando self-update para auto-actualización (M14c) ([#52](https://github.com/Angel-p-v/media-manager/issues/52)) ([543897b](https://github.com/Angel-p-v/media-manager/commit/543897b5ec8b5705f6ad4955fb784df16a5525ac))
* **cli:** sincronización automática de metadatos (M17) ([#58](https://github.com/Angel-p-v/media-manager/issues/58)) ([27a6b4f](https://github.com/Angel-p-v/media-manager/commit/27a6b4f7926557d1a5b14b4d63b4f8830d6f6561))
* **config,db,cli:** configuración YAML metadatos + TVDB proveedor (M16) ([#56](https://github.com/Angel-p-v/media-manager/issues/56)) ([ea4f39a](https://github.com/Angel-p-v/media-manager/commit/ea4f39af5c0a5937b163760162bf2b339646e665))
* **db,cli:** etiquetas y colecciones (M14d) ([#53](https://github.com/Angel-p-v/media-manager/issues/53)) ([50fb278](https://github.com/Angel-p-v/media-manager/commit/50fb27878a1da2316ba61459a2de47bba6358301))
* **db,cli:** metadatos externos TMDB/TVDB (M15) ([#55](https://github.com/Angel-p-v/media-manager/issues/55)) ([52ce526](https://github.com/Angel-p-v/media-manager/commit/52ce5266e13f0f49c10f53c1c8d8c533250d4eb2))
* **db:** añade countFavorites a LibraryService ([#40](https://github.com/Angel-p-v/media-manager/issues/40)) ([58d58dd](https://github.com/Angel-p-v/media-manager/commit/58d58ddff9b27f0ef1997b42105a2012d8b27d44))
* **db:** esquema v3, estado reproducción, doctor y comandos CLI ([#20](https://github.com/Angel-p-v/media-manager/issues/20)) ([93e2ba8](https://github.com/Angel-p-v/media-manager/commit/93e2ba8c35d02d625c3a8554e72b3f922ff14489))
* M11 Biblioteca sana y M12 macOS — filtros, favoritos, TUI, CI 4 plataformas ([#35](https://github.com/Angel-p-v/media-manager/issues/35)) ([7a4e437](https://github.com/Angel-p-v/media-manager/commit/7a4e437a529f20439d46ed88ec5030d9a394b2ce))
* **player:** soporte Windows para v2.1 (M14b) ([#51](https://github.com/Angel-p-v/media-manager/issues/51)) ([7d438b7](https://github.com/Angel-p-v/media-manager/commit/7d438b70116b59e805fadbcb384049a036ea249d))
* **player:** tracker IPC mpv y integración en launcher ([#23](https://github.com/Angel-p-v/media-manager/issues/23)) ([70de97c](https://github.com/Angel-p-v/media-manager/commit/70de97c8c4100c9cfaa1be4d6a79a388709dde76))
* release v2.0.0 — M9/M10/M11/M12 completados ([#45](https://github.com/Angel-p-v/media-manager/issues/45)) ([c2e1410](https://github.com/Angel-p-v/media-manager/commit/c2e1410a930259ffe0ffeb5c580882374df1e1a8))
* **release:** artefactos verificados y publicación sin npm (fases 3-4) ([#13](https://github.com/Angel-p-v/media-manager/issues/13)) ([75f8ea9](https://github.com/Angel-p-v/media-manager/commit/75f8ea97b220ccfbe5bdd5e905c47893f8bfb120))
* **release:** instaladores seguros desde GitHub Releases (fases 1-2) ([#12](https://github.com/Angel-p-v/media-manager/issues/12)) ([5ca5674](https://github.com/Angel-p-v/media-manager/commit/5ca5674530c8b2da9cbb7e306a22bd08d10e1969))
* **test:** mejora cobertura de pruebas (M22) ([#66](https://github.com/Angel-p-v/media-manager/issues/66)) ([64cfa5a](https://github.com/Angel-p-v/media-manager/commit/64cfa5a65626b690d7efc69d1e5f208c9f704fad))
* **test:** tests interactivos con TTY real usando node-pty (M21) ([#65](https://github.com/Angel-p-v/media-manager/issues/65)) ([a4f2b84](https://github.com/Angel-p-v/media-manager/commit/a4f2b8465426bd742d566878604944a300a7949e))
* **ui,db:** sincronización automática de metadatos (M19) ([#62](https://github.com/Angel-p-v/media-manager/issues/62)) ([2fa2e22](https://github.com/Angel-p-v/media-manager/commit/2fa2e22851b0698f74584bdbb31d36ca5f015cdf))
* **ui:** metadatos visuales en TUI (M18) ([#61](https://github.com/Angel-p-v/media-manager/issues/61)) ([5797005](https://github.com/Angel-p-v/media-manager/commit/579700545c3759b468dc81c42b201905773709aa))
* **ui:** sección Continuar en la TUI ([#27](https://github.com/Angel-p-v/media-manager/issues/27)) ([539dcc5](https://github.com/Angel-p-v/media-manager/commit/539dcc5cc7301822cbd356e5dbd5a77390977038))


### Bug Fixes

* **ci:** corrige matrix macOS en release-artifacts ([#46](https://github.com/Angel-p-v/media-manager/issues/46)) ([1ce0ea2](https://github.com/Angel-p-v/media-manager/commit/1ce0ea24b435cda805a5a925e0d6793a31478ccf))
* **cli:** corrige precedencia de operadores en find (macOS quarantine) ([#39](https://github.com/Angel-p-v/media-manager/issues/39)) ([9b4655c](https://github.com/Angel-p-v/media-manager/commit/9b4655c533d731f8ad27c3b64617b517e473278d))
* corrige errores de tipo + actualiza README con diagrama de flujos ([#54](https://github.com/Angel-p-v/media-manager/issues/54)) ([25edc1e](https://github.com/Angel-p-v/media-manager/commit/25edc1ecf060a93e89882e7e766352ea95939ea6))
* **db:** mediaRepo IN clause parameterizado y chunking (&gt;999 params) ([#41](https://github.com/Angel-p-v/media-manager/issues/41)) ([6688934](https://github.com/Angel-p-v/media-manager/commit/66889343410df380cb4b0b9acfa351e75ed8db4b))
* macOS quarantine todos ejecutables + JSDoc watched/unwatched + benchmarks migraciones ([#43](https://github.com/Angel-p-v/media-manager/issues/43)) ([a74096a](https://github.com/Angel-p-v/media-manager/commit/a74096a9797d598aa68f6553df20a6bb9ee8d406))
* **release:** expande ~ correctamente en los instaladores ([#16](https://github.com/Angel-p-v/media-manager/issues/16)) ([3ac5bd4](https://github.com/Angel-p-v/media-manager/commit/3ac5bd4efc3c4fc0f1bd02f645ac55d0a6c0ab4e))
* **ui:** corrige exportación ESM de App para Node 22 + verbatimModuleSyntax ([#44](https://github.com/Angel-p-v/media-manager/issues/44)) ([1f6022f](https://github.com/Angel-p-v/media-manager/commit/1f6022f492edd122133c5cb0f4334af8d3cb486e))

## [1.3.0](https://github.com/Angel-p-v/media-manager/compare/v1.2.0...v1.3.0) (2026-10-23)


### Features

* M11 Biblioteca sana y M12 macOS — filtros, favoritos, TUI, CI 4 plataformas ([#35](https://github.com/Angel-p-v/media-manager/issues/35)) ([7a4e437](https://github.com/Angel-p-v/media-manager/commit/7a4e437a529f20439d46ed88ec5030d9a394b2ce))

## [1.2.0](https://github.com/Angel-p-v/media-manager/compare/v1.1.0...v1.2.0) (2026-10-23)


### Features

* **player:** tracker IPC mpv y integración en launcher ([#23](https://github.com/Angel-p-v/media-manager/issues/23)) ([70de97c](https://github.com/Angel-p-v/media-manager/commit/70de97c8c4100c9cfaa1be4d6a79a388709dde76))
* **ui:** sección Continuar en la TUI ([#27](https://github.com/Angel-p-v/media-manager/issues/27)) ([539dcc5](https://github.com/Angel-p-v/media-manager/commit/539dcc5cc7301822cbd356e5dbd5a77390977038))

## [1.1.0](https://github.com/Angel-p-v/media-manager/compare/v1.0.1...v1.1.0) (2026-10-23)


### Features

* **db:** esquema v3, estado reproducción, doctor y comandos CLI ([#20](https://github.com/Angel-p-v/media-manager/issues/20)) ([93e2ba8](https://github.com/Angel-p-v/media-manager/commit/93e2ba8c35d02d625c3a8554e72b3f922ff14489))

## [1.0.1](https://github.com/Angel-p-v/media-manager/compare/v1.0.0...v1.0.1) (2026-10-23)


### Bug Fixes

* **release:** expande ~ correctamente en los instaladores ([#16](https://github.com/Angel-p-v/media-manager/issues/16)) ([3ac5bd4](https://github.com/Angel-p-v/media-manager/commit/3ac5bd4efc3c4fc0f1bd02f645ac55d0a6c0ab4e))

## [1.0.0] - 2026-10-23

Primera versión estable de `media-manager-cli`: un gestor de series, anime y películas
100 % terminal, con índice local y reproducción en `mpv` o `vlc`.

### Añadido

- CLI `mm` con `config`, `index`, `list`, `search`, `show`, `play`, `watch`, `db stats` y
  `db optimize`, con salida humana y `--json` para scripts.
- TUI a pantalla completa con navegación, búsqueda, detalles, ayuda y atajos
  configurables.
- Base SQLite local con WAL, FTS5, migraciones versionadas y repositorios inyectables.
- Indexación incremental y modo `watch` con reindexado automático.
- Parser de nombres de series, anime y películas, con detección de temporada, episodio,
  año y calidad.
- Reproductor configurable con plantillas de argumentos y modos `detach`/`foreground`;
  soporte de `mpv` y `vlc`.
- Configuración YAML validada con Zod, variables de entorno y valores por defecto.
- Logging estructurado (`text`/`json`), niveles configurables y `--verbose`.
- Errores con código estable y pila opcional con `--verbose`.
- Migración v2 con índices de rendimiento y mantenimiento explícito de SQLite
  (`PRAGMA optimize` / `VACUUM`).
- Benchmarks de escaneo, indexado y consultas, y test de estrés de 100 000 títulos.
- Instaladores POSIX `install.sh` y `uninstall.sh` con checksum SHA-256 obligatorio,
  validación del artefacto, instalación atómica y desinstalación conservadora.
- Bundles portables para Linux x64 y arm64 con Node oficial verificado, `install.json`,
  avisos de terceros, SBOM CycloneDX y atestación de procedencia.
- Documentación de arquitectura, rendimiento, seguridad, contribución y publicación.

### Cambiado

- La distribución pasa a GitHub Releases; no se publica en npm. El binario sigue
  llamándose `mm`.
- La configuración de logging emite siempre en `stderr`, dejando `stdout` para los datos.

### Documentación

- README con instalación, demo, configuración, atajos y contrato de la CLI.
- ADR 0001–0007 y presupuesto de rendimiento en `docs/guides/performance.md`.

[1.0.0]: https://github.com/Angel-p-v/media-manager/releases/tag/v1.0.0
