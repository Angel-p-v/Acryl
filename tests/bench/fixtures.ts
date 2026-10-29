import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';

export const BENCH_LIBRARY_FILES = 10_000;
const FILES_PER_FOLDER = 100;
const FOLDERS_PER_BATCH = 5;

/**
 * Crea una biblioteca sintética de series con nombres clasificables.
 *
 * Los ficheros se escriben en lotes para no agotar descriptores en máquinas con
 * límites bajos; el coste de creación queda fuera de la medición del benchmark.
 */
export async function createBenchLibrary(
  root: string,
  totalFiles: number = BENCH_LIBRARY_FILES,
): Promise<void> {
  const folders = Math.ceil(totalFiles / FILES_PER_FOLDER);

  for (let start = 0; start < folders; start += FOLDERS_PER_BATCH) {
    const batch = Array.from(
      { length: Math.min(FOLDERS_PER_BATCH, folders - start) },
      async (_, offset) => {
        const folder = start + offset;
        const folderPath = path.join(root, `Show ${folder} (2020)`);
        await mkdir(folderPath, { recursive: true });

        const files = Math.min(FILES_PER_FOLDER, totalFiles - folder * FILES_PER_FOLDER);
        await Promise.all(
          Array.from({ length: files }, async (__, episode) => {
            const name = `Show.${folder}.S01E${String(episode + 1).padStart(3, '0')}.mkv`;
            await writeFile(path.join(folderPath, name), '');
          }),
        );
      },
    );

    await Promise.all(batch);
  }
}
