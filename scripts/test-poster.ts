/**
 * Script de prueba independiente para verificar la descarga de pósters.
 *
 * Llama directamente a cachePoster() con una URL de imagen real y un título,
 * para comprobar de forma aislada si el sistema guarda correctamente la
 * imagen en ~/.local/share/media-manager/posters/.
 *
 * Uso: npx tsx scripts/test-poster.ts
 */

import { cachePoster } from '../src/core/metadata/posterCache.js';
import { createLogger } from '../src/shared/logger.js';

// URL de imagen real de prueba (picsum.photos - estable, pública, sin restricciones de User-Agent)
const TEST_IMAGE_URL = 'https://picsum.photos/200/300';

const TEST_TITLE = 'black-torch-test';

async function main(): Promise<void> {
  const logger = createLogger({ level: 'debug', format: 'text' });

  console.log('----------------------------------------');
  console.log('  PRUEBA DE DESCARGA DE PÓSTER');
  console.log('========================================');
  console.log(`URL de imagen: ${TEST_IMAGE_URL}`);
  console.log(`Título: ${TEST_TITLE}`);
  console.log('----------------------------------------');

  try {
    const localPath = await cachePoster(
      TEST_IMAGE_URL,
      TEST_TITLE,
      'tv',
      logger,
    );

    if (localPath) {
      console.log('----------------------------------------');
      console.log('RESULTADO: ÉXITO');
      console.log(`Ruta local: ${localPath}`);
      console.log('----------------------------------------');

      // Verificar que el archivo existe y tiene contenido
      const fs = await import('node:fs');
      const stat = fs.statSync(localPath);
      console.log(`Tamaño del archivo: ${stat.size} bytes`);
      console.log(
        `¿Archivo existe?: ${fs.existsSync(localPath)}`,
      );
    } else {
      console.log('----------------------------------------');
      console.log('RESULTADO: FALLO');
      console.log('cachePoster retornó null');
      console.log('----------------------------------------');
    }
  } catch (error) {
    console.log('----------------------------------------');
    console.log('RESULTADO: EXCEPCIÓN');
    console.log(`Error: ${error}`);
    console.log('----------------------------------------');
  }
}

void main();
