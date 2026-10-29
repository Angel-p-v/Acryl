import { InvalidArgumentError } from 'commander';

import type { MediaType } from '../core/config/schema.js';
import type { ListMediaOptions } from '../core/db/repositories/mediaRepo.js';

const MEDIA_TYPES: readonly MediaType[] = ['series', 'anime', 'movie'];
const SORT_ORDERS: readonly NonNullable<ListMediaOptions['orderBy']>[] = [
  'title',
  'added',
  'recent',
];

/** Convierte `--type` en un tipo válido o lanza un error de uso. */
export function parseMediaType(value: string): MediaType {
  const match = MEDIA_TYPES.find((type) => type === value);
  if (!match) {
    throw new InvalidArgumentError(`Tipo no válido: «${value}». Usa series, anime o movie.`);
  }
  return match;
}

/** Convierte `--sort` en un criterio válido o lanza un error de uso. */
export function parseSortOrder(value: string): NonNullable<ListMediaOptions['orderBy']> {
  const match = SORT_ORDERS.find((order) => order === value);
  if (!match) {
    throw new InvalidArgumentError(`Orden no válido: «${value}». Usa title, added o recent.`);
  }
  return match;
}

/** Convierte un entero positivo; cualquier otra cosa es un error de uso. */
export function parsePositiveInteger(value: string): number {
  const parsed = Number(value);
  if (!Number.isInteger(parsed) || parsed < 1) {
    throw new InvalidArgumentError(`«${value}» no es un entero positivo.`);
  }
  return parsed;
}

/** Convierte un entero no negativo; cualquier otra cosa es un error de uso. */
export function parseNonNegativeInteger(value: string): number {
  const parsed = Number(value);
  if (!Number.isInteger(parsed) || parsed < 0) {
    throw new InvalidArgumentError(`«${value}» no es un entero no negativo.`);
  }
  return parsed;
}
