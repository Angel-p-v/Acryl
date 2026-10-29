import pc from 'picocolors';

import { withCommandContext, type CommandOptions } from './context.js';
import { toMediaJson } from './output.js';

export type FavoriteCommandOptions = CommandOptions & {
  off?: boolean;
  json?: boolean;
};

/** Implementa `mm favorite`: marca o desmarca favorito. */
export async function runFavoriteCommand(
  query: string,
  options: FavoriteCommandOptions,
): Promise<void> {
  await withCommandContext(options, ({ library, preferenceRepo }) => {
    const { media } = library.find(query);

    const isFavorite = options.off === true ? false : true;
    const updated = preferenceRepo.setFavorite({
      mediaId: media.id,
      favorite: isFavorite,
      updatedAt: new Date().toISOString(),
    });

    if (options.json === true) {
      console.log(
        JSON.stringify(
          {
            media: toMediaJson(media),
            favorite: updated.favorite,
          },
          null,
          2,
        ),
      );
      return;
    }

    const year = media.year === null ? '' : ` (${media.year})`;
    const status = isFavorite ? '★ favorito' : '☆ sin favorito';
    console.log(`${pc.green(status)} #${media.id} ${media.title}${year}`);
  });
}
