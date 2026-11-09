// The one storage seam (ADR 0006): a real Postgres when DATABASE_URL is set, PGlite (in-process Postgres) otherwise, for local runs.
// Everything above this file speaks SQL through `rows` and `tx` and does not know which one it has.
import { existsSync, readFileSync } from 'node:fs';

export interface Tx { rows<T>(sql: string, params?: unknown[]): Promise<T[]> }
export interface Db extends Tx {
  exec(sql: string): Promise<void>;
  tx<R>(fn: (t: Tx) => Promise<R>): Promise<R>;
  /**
   * The boot lock (ADR 0010, "A rollout's overlap"): one process owns a database's pods. Answers undefined once this process
   * holds it, else why not. `lost` is called if the lock is later found gone, after which this process must stop serving.
   * PGlite has none: the in-process database is the local trial's own, and no other process reaches it.
   */
  bootLock?(lost: (why: string) => void): Promise<string | undefined>;
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

/** The boot lock's first key ("JM"); the second is the schema's hash, so servers confined to different schemas never meet. */
const LOCK_CLASS = 19021;
/** How long a refused start keeps trying: a just-killed owner's session can take a moment to close on the database side. */
const LOCK_GRACE_MS = 2000;
/** How often the holder checks it still holds the lock. Under IDLE_MS, so the check also keeps its own session from timing out. */
const LOCK_CHECK_MS = 5000;
/** The lock's session is ended by the database after this long idle, so an owner whose host vanished frees it within about this. */
const IDLE_MS = 30000;

/**
 * A session-level advisory lock on a connection reserved for the process's life. Postgres frees it when that session ends, so
 * a process that dies never wedges the next boot. The holder re-checks it; a lost session (a network cut, the idle timeout)
 * calls `lost`, because another process may now hold it.
 */
async function takeBootLock(sql: BunSql, schema: string | undefined, lost: (why: string) => void): Promise<string | undefined> {
  const conn = await sql.reserve();
  // Without PG_SCHEMA, unqualified queries resolve through the role's search_path, so the key is the schema they land in,
  // not "public": a server with PG_SCHEMA=app and one whose search_path puts app first must meet on the same lock.
  const [cur] = schema === undefined ? await conn.unsafe('select current_schema() as s') : [{ s: schema }];
  const key = String(cur?.s ?? '');
  // Postgres 14 and later; an older one keeps the lock until its own TCP keepalive notices a vanished owner.
  await conn.unsafe(`set idle_session_timeout = ${IDLE_MS}`).catch(() => undefined);
  const until = Date.now() + LOCK_GRACE_MS;
  for (;;) {
    const [row] = await conn.unsafe(`select pg_try_advisory_lock(${LOCK_CLASS}, hashtext($1)) as held`, [key]);
    if (row?.held === true) break;
    if (Date.now() >= until) {
      conn.release();
      return `Another server holds this database's boot lock${schema === undefined ? '' : ` (schema ${schema})`}: one process owns every pod (ADR 0010). Stop it first, or point this server at another database.`;
    }
    await new Promise((r) => setTimeout(r, 100));
  }
  const check = `select count(*)::int as n from pg_locks where locktype = 'advisory' and granted and pid = pg_backend_pid() and classid = ${LOCK_CLASS}`;
  const timer = setInterval(() => {
    conn.unsafe(check).then(
      (rows) => { if (rows[0]?.n !== 1) lost('The boot lock is gone from its session; another server may own the pods now. Stopping.'); },
      (e: unknown) => lost(`The boot lock's database session failed (${e instanceof Error ? e.message : String(e)}); stopping.`),
    );
  }, LOCK_CHECK_MS);
  (timer as { unref?: () => void }).unref?.();
  return undefined;
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
    bootLock: (lost) => takeBootLock(sql, schema, lost),
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
