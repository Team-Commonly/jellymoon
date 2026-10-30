// The entry for local runs and deploys that the e2e suite and the load driver talk to. It is the public routes plus /__test/*:
//   ENV=local   every /__test route is open (session, clock, stats) and the D1 schema is created on first use;
//   otherwise   every /__test route needs `Authorization: Bearer <TEST_BEARER>` (a Worker secret) and the clock route does not exist.
// BOOT_SWEEP=1 wakes every pod in D1 on the first request after a start, because a local restart does not re-fire stored alarms
// without a request. The production entry (src/index.ts) has none of this.
import schemaSql from '../../schema.sql';
import { allPodIds, createUser } from '../directory';
import type { Env } from '../env';
import { handle, internal } from '../front';
import { bad, fail, parse, str } from '../wire';
import type { Body } from '../wire';
import { TestPodDO } from './pod-test';

let ready: Promise<void> | undefined;

async function prepare(env: Env): Promise<void> {
  if (env.ENV === 'local') {
    await env.DB.batch(schemaSql.split(';').map((q) => q.trim()).filter((q) => q !== '').map((q) => env.DB.prepare(q)));
  }
  if (env.BOOT_SWEEP === '1') await Promise.all((await allPodIds(env.DB)).map((id) => internal(env, id, 'ping', {})));
}

async function fanOut(env: Env, name: string, body: Body): Promise<Response[]> {
  return Promise.all((await allPodIds(env.DB)).map((id) => internal(env, id, name, body)));
}

async function clockRoute(req: Request, env: Env): Promise<Response> {
  if (req.method === 'GET') {
    const row = await env.DB.prepare("select v from kv where k = 'clock'").first<{ v: string }>();
    return Response.json({ now: row === null ? Date.now() : Number(row.v) });
  }
  const now = (await parse(req))?.now;
  if (typeof now !== 'number' || !Number.isInteger(now) || now < 0) return bad('now is unix milliseconds.');
  await env.DB.prepare("insert or replace into kv (k, v) values ('clock', ?)").bind(String(now)).run();
  await fanOut(env, 'clock', { now });
  return Response.json({ now });
}

async function statsRoute(env: Env): Promise<Response> {
  const total: Record<string, { commands: number; rowsWritten: number }> = {};
  for (const res of await fanOut(env, 'stats', {})) {
    for (const [type, w] of Object.entries((await res.json()) as typeof total)) {
      const t = total[type] ?? { commands: 0, rowsWritten: 0 };
      total[type] = { commands: t.commands + w.commands, rowsWritten: t.rowsWritten + w.rowsWritten };
    }
  }
  return Response.json(total);
}

async function testRoute(req: Request, env: Env, path: string): Promise<Response> {
  const local = env.ENV === 'local';
  if (!local && (env.TEST_BEARER === undefined || env.TEST_BEARER === '' || req.headers.get('Authorization') !== `Bearer ${env.TEST_BEARER}`)) {
    return fail('unauthorized', 'Send the test bearer.');
  }
  if (path === '/__test/session' && req.method === 'POST') {
    const name = str((await parse(req))?.name, 1, 80);
    return name === undefined ? bad('name is required.') : Response.json(await createUser(env.DB, name));
  }
  if (path === '/__test/clock' && local) return clockRoute(req, env);
  const fired = /^\/__test\/alarm\/([^/]+)$/.exec(path);
  if (fired) return internal(env, fired[1], 'alarm-fired', {});
  const evict = /^\/__test\/evict\/([^/]+)$/.exec(path);
  if (evict && req.method === 'POST') return internal(env, evict[1], 'evict', {});
  if (path === '/__test/stats' && req.method === 'GET') return statsRoute(env);
  return fail('not_found', 'No such test route.');
}

export { TestPodDO as PodDO };
export default {
  async fetch(req: Request, env: Env): Promise<Response> {
    ready ??= prepare(env);
    await ready;
    const path = new URL(req.url).pathname;
    return path.startsWith('/__test/') ? testRoute(req, env, path) : handle(req, env);
  },
};
