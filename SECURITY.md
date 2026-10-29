# Política de seguridad

Acryl es una aplicación de escritorio local: no expone un servidor ni envía la
biblioteca a Internet. Aun así, procesa rutas y nombres de ficheros del sistema y
llama a procesos externos para la reproducción, así que los fallos de seguridad se
tratan con seriedad.

## Versiones soportadas

| Versión | Soporte de seguridad |
| ------- | -------------------- |
| 0.x     | Sí                   |
| < 0.1.0 | No                   |

## Cómo reportar una vulnerabilidad

**No abras una issue pública** para un problema de seguridad. Usa el canal privado de
GitHub Security Advisories:

1. Abre <https://github.com/Angel-p-v/app-media-manager/security/advisories/new>.
2. Describe el impacto, la versión afectada y los pasos mínimos para reproducirlo.
3. Incluye, si puedes, una propuesta de solución o un parche.

También puedes contactar con el mantenedor a través de su perfil de GitHub si el canal
privado no está disponible. No publiques detalles explotables hasta que exista una
versión corregida.

## Qué incluir

- Versión de `acryl` (`acryl --version`) y sistema operativo.
- Configuración relevante, ocultando rutas o datos personales.
- Un caso de reproducción pequeño y determinista.
- Impacto esperado: lectura/escritura de ficheros fuera de la biblioteca, ejecución de
  comandos, denegación de servicio, fuga de información, etc.

## Alcance

Se consideran vulnerabilidades, entre otras:

- Inyección de comandos o argumentos al lanzar procesos externos.
- Escritura o lectura de ficheros fuera de las rutas configuradas.
- Ejecución de código a través de la configuración o del índice.
- Problemas de permisos en la base SQLite o en los ficheros temporales.
- Vulnerabilidades explotables en dependencias directas.

Quedan fuera del alcance los fallos que requieran acceso físico o privilegios de
administrador, y los ataques contra una biblioteca que el usuario ya haya marcado como
de confianza.

## Modelo de confianza y verificación

La distribución oficial son las GitHub Releases. Cada release adjunta:

- `acryl-<versión>-<plataforma>.tar.gz`;
- su checksum SHA-256 (`.sha256`);
- `sbom.cdx.json` en formato CycloneDX;
- una atestación de procedencia de GitHub Actions.

El instalador descarga por HTTPS con validación TLS, exige checksum antes de extraer,
rechaza rutas absolutas o con `..` dentro del archivo y valida el manifiesto de la
release. El instalador no ejecuta ningún binario del bundle antes de verificar la
integridad.

Verificación manual:

```bash
sha256sum -c acryl-<versión>-linux-x64.tar.gz.sha256
gh attestation verify acryl-<versión>-linux-x64.tar.gz \
  --repo Angel-p-v/app-media-manager
```

La atestación y el checksum son complementarios, no sustitutos: ambos viven en el mismo
release y la raíz de confianza última es la cuenta de GitHub que publica. No hay firma
GPG ni minisign en la versión 0.x. El bundle Linux depende de glibc; Alpine/musl no está
soportado.

## Respuesta

El mantenedor acusará recibo y evaluará el impacto lo antes posible. Si el reporte es
válido, se preparará un parche, se publicará una versión corregida y se coordinará la
divulgación en un GitHub Security Advisory. Las versiones anteriores a la corregida no
recibirán parches salvo que el impacto lo justifique.