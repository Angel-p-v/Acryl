export function findChecksum(contents: string, filename: string): string | undefined;

export function fetchNodeRuntime(options: {
  version: string;
  cacheRoot?: string;
  system?: NodeJS.Platform;
  arch?: string;
}): Promise<{
  binaryPath: string;
  licensePath: string;
  nodePlatform: string;
  version: string;
}>;
