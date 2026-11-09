// Just enough of bun:sqlite for tsc (the D1 fake in d1.ts); the module ships with Bun, so no dependency is added.
declare module 'bun:sqlite' {
  interface Statement { all(...args: unknown[]): unknown[]; get(...args: unknown[]): unknown; run(...args: unknown[]): unknown }
  export class Database {
    constructor(path: string);
    exec(sql: string): void;
    prepare(sql: string): Statement;
    transaction<T>(fn: () => T): () => T;
  }
}
