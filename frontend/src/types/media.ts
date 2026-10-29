export type MediaType = 'series' | 'anime' | 'movie';

export type Media = {
  readonly id: number;
  readonly type: MediaType;
  readonly title: string;
  readonly sort_title: string;
  readonly year: number | null;
  readonly folder_path: string;
};

export type PlayerStatus = {
  readonly timePosition: number;
  readonly duration: number;
  readonly paused: boolean;
  readonly volume: number;
};
