declare module 'node-pty' {
  export type IPty = {
    pid: number;
    fd: number;
    exitCode?: number | null;
    write(data: string): void;
    resize(cols: number, rows: number): void;
    kill(signal?: string | number): void;
    on(event: 'data', listener: (data: string) => void): this;
    on(event: 'exit', listener: (code: number, signal: number) => void): this;
    on(event: string, listener: (...args: unknown[]) => void): this;
    removeListener(event: 'data', listener: (data: string) => void): this;
    removeListener(event: 'exit', listener: (code: number, signal: number) => void): this;
    removeListener(event: string, listener: (...args: unknown[]) => void): this;
    off(event: 'data', listener: (data: string) => void): this;
    off(event: 'exit', listener: (code: number, signal: number) => void): this;
    off(event: string, listener: (...args: unknown[]) => void): this;
  };

  export function spawn(
    file: string,
    args?: string[],
    options?: {
      name?: string;
      cols?: number;
      rows?: number;
      cwd?: string;
      env?: Record<string, string>;
      encoding?: string | null;
      handleFlowControl?: boolean;
    },
  ): IPty;
}
