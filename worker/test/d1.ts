import { Database } from 'bun:sqlite';
import { readFileSync } from 'node:fs';

// D1 and the pod namespace as small fakes for unit tests. The D1 is a real SQLite (bun:sqlite) behind the few calls the Worker
// makes: prepare, bind, first, all, run, and batch as one transaction.
type Arg = string | number | null;

class Stmt {
  constructor(private readonly db: Database, private readonly sql: string, private readonly args: Arg[] = []) {}
  bind(...args: Arg[]): Stmt { return new Stmt(this.db, this.sql, args); }
  rows(): Record<string, unknown>[] { return this.db.prepare(this.sql).all(...this.args) as Record<string, unknown>[]; }
  async all(): Promise<{ results: Record<string, unknown>[] }> { return { results: this.rows() }; }
  async first(): Promise<Record<string, unknown> | null> { return this.rows()[0] ?? null; }
  async run(): Promise<{ success: true }> { this.rows(); return { success: true }; }
}

/** Reads a file of worker/ (schema.sql, migrations/...). */
export const workerFile = (path: string): string => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');

/** A D1 over an in-memory SQLite, with each SQL script applied in order; `sqlite` is the database underneath. */
export function sqliteD1(...scripts: string[]): { d1: D1Database; sqlite: Database } {
  const sqlite = new Database(':memory:');
  for (const s of scripts) sqlite.exec(s);
  const d1 = {
    prepare: (q: string) => new Stmt(sqlite, q),
    batch: async (stmts: Stmt[]) => sqlite.transaction(() => stmts.map((s) => ({ results: s.rows() })))(),
  };
  return { d1: d1 as unknown as D1Database, sqlite };
}

/** A pod namespace whose objects answer with `on(podId, request)`. */
export function podNamespace(on: (podId: string, req: Request) => Promise<Response>): DurableObjectNamespace {
  const ns = {
    idFromName: (name: string) => name,
    get: (id: string) => ({ fetch: (input: Request | string, init?: RequestInit) => on(id, input instanceof Request ? input : new Request(input, init)) }),
  };
  return ns as unknown as DurableObjectNamespace;
}
