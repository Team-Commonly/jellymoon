import { expect, test } from 'bun:test';
import type { Database } from 'bun:sqlite';
import { readdirSync } from 'node:fs';
import { authRoute } from '../src/auth';
import { authenticate, createUser } from '../src/directory';
import type { Env } from '../src/env';
import { hashToken } from '../src/ids';
import { podNamespace, sqliteD1, workerFile } from './d1';

// A real D1 is made and changed only by worker/migrations (`wrangler d1 migrations apply`); schema.sql is the test entry's fresh
// shape. 0000 is the schema before the sign-in hardening, so a D1 created from that schema.sql takes 0001 alone.
const names = readdirSync(new URL('../migrations', import.meta.url)).sort();
const migration = (n: string): string => workerFile(`migrations/${n}`);

/** Every table's columns and every index, as SQLite reports them. */
function shape(db: Database): unknown {
  const tables = db.prepare("select name from sqlite_master where type = 'table' and name not like 'sqlite_%' order by name").all() as { name: string }[];
  const indexes = db.prepare("select name, tbl_name from sqlite_master where type = 'index' and sql is not null order by name").all();
  return { indexes, tables: tables.map((t) => [t.name, db.prepare(`select name, type, "notnull", dflt_value, pk from pragma_table_info('${t.name}')`).all()]) };
}

test('the migrations are numbered the way wrangler applies them, and together make what schema.sql makes', () => {
  expect(names).toEqual(['0000_initial.sql', '0001_auth_hardening.sql']);
  expect(shape(sqliteD1(...names.map(migration)).sqlite)).toEqual(shape(sqliteD1(workerFile('schema.sql')).sqlite));
});

test('a D1 created before the hardening serves sign-in once 0001 is applied, and its old user tokens are no callers', async () => {
  const { d1, sqlite } = sqliteD1(migration('0000_initial.sql'));
  expect(await createUser(d1, 'early', 0).then(() => 'minted', () => 'refused')).toBe('refused'); // no expires_at column yet
  const old = `jmu_${'o'.repeat(43)}`;
  sqlite.exec("insert into users (id, name) values ('usr_old', 'old')");
  sqlite.prepare("insert into tokens (hash, kind, subject) values (?, 'user', 'usr_old')").run(await hashToken(old));
  sqlite.exec(migration('0001_auth_hardening.sql'));
  expect(await authenticate(d1, `Bearer ${old}`, 0)).toBe(null);
  const env = { DB: d1, POD: podNamespace(async () => Response.json({})), CLOCK_MS: 1 } as unknown as Env;
  const { token } = await createUser(d1, 'ana', 0);
  expect((await authenticate(d1, `Bearer ${token}`, 0))?.kind).toBe('user');
  expect((await authRoute(new Request('https://w.example/v1/auth/device', { method: 'POST' }), env, 0))?.status).toBe(200);
  await d1.prepare('insert into token_pods (hash, pod_id) values (?, ?)').bind(await hashToken(token), 'pod_1').run();
  expect((await authRoute(new Request('https://w.example/v1/auth/logout', { method: 'POST', headers: { Authorization: `Bearer ${token}` } }), env, 1))?.status).toBe(200);
  expect(await authenticate(d1, `Bearer ${token}`, 1)).toBe(null);
});
