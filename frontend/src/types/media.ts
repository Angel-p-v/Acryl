export type MediaType = 'series' | 'anime' | 'movie';

export interface Media {
  readonly id: number;
  readonly type: MediaType;
  readonly title: string;
  readonly sort_title: string;
  readonly year: number | null;
  readonly folder_path: string;
}
