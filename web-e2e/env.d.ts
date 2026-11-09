// Just enough of Node and of ws for tsc; Playwright runs the specs under Node, so no @types package is added (as server/src/env.d.ts).
declare const process: { env: Record<string, string | undefined> };

declare module 'node:child_process' {
  interface Child {
    stdout: { on(e: 'data', f: (d: unknown) => void): void } | null;
    stderr: { on(e: 'data', f: (d: unknown) => void): void } | null;
    on(e: 'exit', f: (code: unknown, signal: unknown) => void): void;
    kill(signal: string): boolean;
  }
  export function spawn(cmd: string, args: string[], opts: { cwd: string; env: Record<string, string | undefined>; stdio: string[] }): Child;
}

declare module 'node:fs' {
  export function existsSync(path: string): boolean;
  export function mkdtempSync(prefix: string): string;
  export function rmSync(path: string, opts: { recursive: boolean; force: boolean }): void;
}

declare module 'node:net' {
  interface Server {
    once(e: 'error', f: (err: unknown) => void): void;
    listen(port: number, host: string, f: () => void): void;
    address(): unknown;
    close(f: () => void): void;
  }
  export function createServer(): Server;
}

declare module 'node:os' {
  export function tmpdir(): string;
}

declare module 'node:path' {
  export function join(...parts: string[]): string;
}

declare module 'node:url' {
  export function fileURLToPath(url: URL): string;
}

declare module 'ws' {
  export default class WebSocket {
    static readonly OPEN: number;
    readonly readyState: number;
    constructor(url: string, opts: { headers: Record<string, string> });
    on(e: 'open' | 'close', f: () => void): void;
    on(e: 'error', f: (err: unknown) => void): void;
    on(e: 'message', f: (data: unknown) => void): void;
    once(e: 'open', f: () => void): void;
    once(e: 'error', f: (err: unknown) => void): void;
    send(text: string): void;
    close(): void;
  }
}
