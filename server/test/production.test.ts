import { expect, test } from 'bun:test';
import { websocket } from 'hono/bun';
import { buildApp } from '../src/app';
import { appOptions, startupRefusal } from '../src/config';
import { openDb } from '../src/db';
import { Hub } from '../src/hub';

// F13 step 4, the server half: the app a production start builds answers no /__test path, over a real socket. The environment is the
// production one (a database, no ENV), taken through the same appOptions main.ts uses; PGlite stands in for the database.
const production = { DATABASE_URL: 'postgres://db.example/jellymoon' };

test('a server started without ENV=local answers every /__test path with 404', async () => {
  expect(startupRefusal(production) === undefined).toBe(true);
  const opts = appOptions(production); expect(opts.local).toBe(false);
  const server = Bun.serve({ port: 0, hostname: '127.0.0.1', fetch: buildApp(await Hub.restore(await openDb({})), opts).fetch, websocket });
  try {
    const base = `http://127.0.0.1:${server.port}`; expect((await fetch(`${base}/healthz`)).status).toBe(200);
    const paths = ['/__test', '/__test/', '/__test/session', '/__test/clock', '/__test/evict/pod_1', '/__test/evict/__proto__', '/__test/discard/pod_1', '/__test/instance/pod_1',
      '/__test/woke/pod_1', '/__test/stored-alarm/pod_1', '//__test/session', '/__test/../__test/session', '/%5F%5Ftest/session', '/__TEST/session', '/__test/session/'];
    for (const path of paths) {
      for (const method of ['GET', 'POST', 'PUT', 'DELETE']) {
        for (const auth of [undefined, 'Bearer sekret', 'Bearer ']) {
          const res = await fetch(base + path, { method, headers: auth === undefined ? {} : { Authorization: auth }, body: method === 'POST' || method === 'PUT' ? '{"name":"a","now":1}' : undefined });
          expect(`${method} ${path} ${auth ?? ''} -> ${res.status}`).toBe(`${method} ${path} ${auth ?? ''} -> 404`);
          expect(((await res.json()) as { code?: string }).code).toBe('not_found');
        }
      }
    }
  } finally { server.stop(true); }
});

test('the test routes open only for ENV=local or a configured TEST_BEARER', () => {
  expect(appOptions({ ENV: 'local' }).local).toBe(true);
  for (const env of [{}, { ENV: 'production' }, { ENV: 'Local' }, { ENV: '' }, { ENV: 'local ' }]) expect(appOptions(env).local).toBe(false);
  expect(appOptions({ TEST_BEARER: 'x' }).testBearer).toBe('x');
});
