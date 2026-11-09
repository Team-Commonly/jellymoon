// Just enough of the Bun and Node globals for tsc; Bun ships them, so no @types package is added (ADR 0006).
declare const process: { env: Record<string, string | undefined>; argv: string[]; exit(code?: number): never };

interface BunSql {
  unsafe(text: string, params?: unknown[]): Promise<Array<Record<string, unknown>>>;
  begin<R>(fn: (tx: BunSql) => Promise<R>): Promise<R>;
}

declare const Bun: {
  serve(opts: { port: number; hostname?: string; fetch: (req: Request, server: unknown) => Response | Promise<Response>; websocket: unknown }): { port: number; stop(close?: boolean): void };
  SQL: new (url: string) => BunSql;
  file(path: string): Blob;
};

declare module 'node:crypto' {
  export function createHash(alg: 'sha256'): { update(s: string): { digest(enc: 'hex'): string } };
  export function randomBytes(n: number): Uint8Array;
  export function timingSafeEqual(a: Uint8Array, b: Uint8Array): boolean;
}

declare module 'node:fs' {
  export function existsSync(path: string): boolean;
  export function readFileSync(path: string): Uint8Array<ArrayBuffer>;
  export function writeFileSync(path: string, data: Uint8Array): void;
  export function renameSync(from: string, to: string): void;
  export function statSync(path: string, opts: { throwIfNoEntry: false }): { isFile(): boolean } | undefined;
  export function realpathSync(path: string): string;
  // for the tests
  export function writeFileSync(path: string, data: string): void;
  export function mkdtempSync(prefix: string): string;
  export function mkdirSync(path: string, opts: { recursive: boolean }): void;
  export function symlinkSync(target: string, path: string): void;
  export function rmSync(path: string, opts: { recursive: boolean; force: boolean }): void;
}

declare module 'node:os' {
  export function tmpdir(): string;
}

declare module 'node:path' {
  export const sep: string;
  export function join(...parts: string[]): string;
  export function resolve(...parts: string[]): string;
}
