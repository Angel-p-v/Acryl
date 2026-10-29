import pc from 'picocolors';

import { withCommandContext, type CommandOptions } from './context.js';
import { formatEpisode, mediaTypeLabel, toMediaFileJson, toMediaJson } from './output.js';

export type ShowCommandOptions = CommandOptions & {
  json?: boolean;
};

/** Implementa `mm show`: detalle de un título y sus ficheros. */
export async function runShowCommand(query: string, options: ShowCommandOptions): Promise<void> {
  await withCommandContext(options, ({ library }) => {
    const { media, files } = library.find(query);

    if (options.json) {
      console.log(
        JSON.stringify(
          {
            ...toMediaJson(media),
            fileCount: files.length,
            files: files.map(toMediaFileJson),
          },
          null,
          2,
        ),
      );
      return;
    }

    const year = media.year === null ? '' : ` (${media.year})`;
    console.log(pc.bold(`${media.title}${year}`));
    console.log(pc.dim(`  #${media.id} · ${mediaTypeLabel(media.type)} · ${media.folderPath}`));
    console.log('');

    if (files.length === 0) {
      console.log(pc.dim('  (sin ficheros asociados)'));
      return;
    }

    console.log(pc.bold(`${files.length} ficheros`));
    for (const file of files) {
      console.log(`  ${pc.dim(`#${file.id}`)} ${pc.cyan(formatEpisode(file))}  ${file.filePath}`);
    }
  });
}
