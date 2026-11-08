// The one storage seam (ADR 0006): a real Postgres when DATABASE_URL is set, PGlite (in-process Postgres) otherwise, for local runs.
// Everything above this file speaks SQL through `rows` and `tx` and does not know which one it has.
import { existsSync, readFileSync } from 'node:fs';

export interface Tx { rows<T>(sql: string, params?: unknown[]): Promise<T[]> }
export interface Db extends Tx {
  exec(sql: string): Promise<void>;
  tx<R>(fn: (t: Tx) => Promise<R>): Promise<R>;
}

/**
 * DATABASE_URL: a real Postgres. PGLITE_DIR: where PGlite keeps its files (memory when unset). PGLITE_TEMPLATE: a data-dir tar
 * made by pglite-template.ts; a fresh PGlite dir starts from it, which skips initdb (about 5 s of CPU, against about 1 s).
 * PG_SCHEMA: confine this server to one schema of a shared Postgres, so the e2e suite can give every server fresh state.
 */
export interface DbEnv { DATABASE_URL?: string; PGLITE_DIR?: string; PGLITE_TEMPLATE?: string; PG_SCHEMA?: string }

/** A value stored as text comes back as a string; one stored as json may come back parsed. Accept both. */
export const json = <T>(v: unknown): T => (typeof v === 'string' ? JSON.parse(v) : v) as T;

async function openPglite(dir: string | undefined, template: string | undefined): Promise<Db> {
  const { PGlite } = await import('@electric-sql/pglite');
  // A dir that already holds a cluster is a restart: loading the template over it would erase the data.
  const fresh = dir !== undefined && !existsSync(`${dir}/PG_VERSION`);
  const loadDataDir = fresh && template !== undefined && existsSync(template) ? new Blob([readFileSync(template)]) : undefined;
  const pg = new PGlite(dir, { loadDataDir });
  await pg.waitReady;
  return {
    rows: async <T>(sql: string, params?: unknown[]) => (await pg.query<T>(sql, params)).rows,
    exec: async (sql) => { await pg.exec(sql); },
    tx: (fn) => pg.transaction((t) => fn({ rows: async <T>(sql: string, params?: unknown[]) => (await t.query<T>(sql, params)).rows })),
  };
}

async function openWire(url: string, schema: string | undefined): Promise<Db> {
  const sql = new Bun.SQL(url);
  if (schema !== undefined) {
    if (!/^[a-z][a-z0-9_]{0,40}$/.test(schema)) throw new Error('PG_SCHEMA is lower-case letters, digits and underscores.');
    await sql.unsafe(`create schema if not exists ${schema}`);
  }
  // With a schema, every statement runs in a transaction that sets the search path first; that holds on any pooled connection.
  const scoped = <R>(fn: (c: BunSql) => Promise<R>): Promise<R> =>
    schema === undefined ? fn(sql) : sql.begin(async (t) => { await t.unsafe(`set local search_path to ${schema}`); return fn(t); });
  const wrap = (c: BunSql): Tx => ({ rows: async <T>(text: string, params?: unknown[]) => (await c.unsafe(text, params)) as T[] });
  return {
    rows: <T>(text: string, params?: unknown[]) => scoped((c) => wrap(c).rows<T>(text, params)),
    exec: async (text) => { await scoped((c) => c.unsafe(text)); },
    tx: (fn) => sql.begin(async (t) => {
      if (schema !== undefined) await t.unsafe(`set local search_path to ${schema}`);
      return fn(wrap(t));
    }),
  };
}

export async function openDb(env: DbEnv): Promise<Db> {
  return env.DATABASE_URL ? openWire(env.DATABASE_URL, env.PG_SCHEMA) : openPglite(env.PGLITE_DIR, env.PGLITE_TEMPLATE);
}
