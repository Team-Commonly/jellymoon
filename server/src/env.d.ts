// Just enough of the Bun and Node globals for tsc; Bun ships them, so no @types package is added (ADR 0005).
declare const process: { env: Record<string, string | undefined>; argv: string[]; exit(code?: number): never };

interface BunSql {
  unsafe(text: string, params?: unknown[]): Promise<Array<Record<string, unknown>>>;
  begin<R>(fn: (tx: BunSql) => Promise<R>): Promise<R>;
}

declare const Bun: {
  serve(opts: { port: number; fetch: (req: Request, server: unknown) => Response | Promise<Response>; websocket: unknown }): { port: number };
  SQL: new (url: string) => BunSql;
};

declare module 'node:crypto' {
  export function createHash(alg: 'sha256'): { update(s: string): { digest(enc: 'hex'): string } };
  export function randomBytes(n: number): Uint8Array;
}

declare module 'node:fs' {
  export function existsSync(path: string): boolean;
  export function readFileSync(path: string): Uint8Array;
  export function writeFileSync(path: string, data: Uint8Array): void;
  export function renameSync(from: string, to: string): void;
}
