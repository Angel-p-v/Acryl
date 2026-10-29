# ADR 0003 — Parseo y clasificación de nombres de fichero

- **Estado:** aceptada
- **Fecha:** 2026-10-22

## Contexto

La biblioteca del usuario ya está ordenada en disco, pero cada colección sigue
convenciones distintas: `Show.S01E02.1080p`, `Show - 03`, `Season 1 Episode 2`,
`T1E2`, películas con año, grupos de subtítulos entre corchetes o etiquetas de
calidad pegadas al título. El indexador debe entender esos nombres **sin** pedir
metadatos a Internet y **sin** abortar la indexación completa cuando un fichero
no encaja.

Además hay que decidir qué es un «título»: una carpeta de serie, una carpeta de
anime o un fichero de película. Esa decisión determina la clave de unicidad en
la tabla `media` (`folder_path`) y, por tanto, qué se actualiza y qué se duplica.

## Decisión

- **Series y anime:** el título es la primera carpeta bajo la raíz
  (`Severance (2022)` → título `Severance`, año `2022`). Un fichero suelto en la
  raíz de esa biblioteca es ambiguo y se descarta; nunca se inventa una carpeta.
- **Películas:** cada fichero es un título independiente y su `folder_path` es
  la **ruta del propio fichero**. Así dos películas que comparten carpeta no se
  fusionan en una sola fila.
- **Patrones de episodio**, evaluados de más específico a más genérico:
  `S01E02`/`S01.E02`/`S01EP02` → `T1E2` → `1x02` → `Season 1 Episode 2` →
  `E03`/`Ep. 3` → numeración absoluta `Show - 03`. El primero que coincide gana,
  de modo que un nombre con varios patrones es determinista.
- **Numeración absoluta** (`Show - 03`): se guarda `season = null` y el número
  como episodio. No se inventa una temporada que el nombre no declara.
- **Etiquetas de release** (`1080p`, `BluRay`, `x264`, `WEB-DL`, `DDP5.1`,
  grupos `[SubsPlease]`…) se eliminan del título. Los corchetes que contienen un
  año (`[2022]`) se conservan como año, porque aportan información.
- **Año:** se toma el último token de cuatro cifras que esté en un rango
  razonable. Si el nombre es solo un año (`1917`, `1984`), se conserva como
  título: es más probable que sea el nombre de la película.
- **Ambigüedad:** los parsers devuelven `null` explícito y nunca lanzan
  excepciones. El indexador cuenta esos ficheros como `skipped` y sigue con el
  resto.

## Alternativas consideradas

- **Usar una librería de parsing de releases** (`parse-torrent-title` y
  similares): añade dependencia y una gramática pensada para torrents, no para
  una biblioteca local; además dificulta cubrir al 100 % los patrones que nos
  interesan.
- **Una carpeta por película como clave**: obliga a que el usuario reorganice su
  biblioteca y rompe colecciones planas de películas.
- **Guardar el nombre original como título**: ensucia la búsqueda con `1080p` y
  nombres de grupo.
- **Lanzar una excepción ante un nombre raro**: un solo fichero con un nombre
  atípico impediría indexar miles de ficheros correctos.

## Consecuencias

- El parseo es puro y testeable en milisegundos; CLI y TUI comparten la misma
  lógica.
- Renombrar la carpeta de una serie se interpreta como un título nuevo; el
  índice elimina el anterior en la siguiente indexación completa.
- Los nombres con patrones no contemplados se omiten de forma visible en las
  estadísticas (`skipped`), que es preferible a indexar datos incorrectos.
- Añadir un patrón nuevo es cambiar una entrada de `PATTERNS` y su caso en la
  tabla de tests; el resto del indexador no se toca.
