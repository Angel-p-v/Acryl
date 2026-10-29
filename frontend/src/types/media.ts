export type MediaType = 'series' | 'anime' | 'movie';

export type Media = {
  readonly id: number;
  readonly type: MediaType;
  readonly title: string;
  readonly sort_title: string;
  readonly year: number | null;
  readonly folder_path: string;
  readonly is_favorite: boolean;
  readonly is_watched: boolean;
  readonly poster_path: string | null;
  readonly synopsis: string | null;
  readonly metadata_fetched_at: string | null;
};

export type MediaFile = {
  readonly id: number;
  readonly media_id: number;
  readonly season: number | null;
  readonly episode: number | null;
  readonly title: string | null;
  readonly file_path: string;
  readonly size_bytes: number | null;
  readonly mtime_ms: number | null;
  readonly indexed_at: string;
};

export type MediaDetails = {
  readonly media: Media;
  readonly files: readonly MediaFile[];
};

export type PlayerStatus = {
  readonly timePosition: number;
  readonly duration: number;
  readonly paused: boolean;
  readonly volume: number;
};
