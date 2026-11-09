import { expect, test } from 'bun:test';
import type { Env } from '../src/env';
import { handle } from '../src/front';
import { podNamespace, sqliteD1, workerFile } from './d1';

// The Worker sends `Referrer-Policy: no-referrer` on every reply, as the server does, so the one-time code in a /device or
// /invite URL never leaves in a Referer header.
test('every reply says no-referrer, errors included', async () => {
  const env = { DB: sqliteD1(workerFile('schema.sql')).d1, POD: podNamespace(async () => Response.json({})) } as unknown as Env;
  for (const path of ['/device?code=ABCD-EFGH', '/invite/tok_secret', '/healthz', '/v1/pods']) {
    const res = await handle(new Request(`https://w.example${path}`), env);
    expect(`${path} ${res.headers.get('Referrer-Policy')}`).toBe(`${path} no-referrer`);
  }
});

// A route that throws is a JSON `internal` reply with the same header, and the error text never reaches the body (twin of
// server/test/web.test.ts "a route that throws still says no-referrer on the onError reply").
test('a route that throws replies 500 internal with no-referrer and without the error text', async () => {
  const boom = 'boom-secret-detail';
  const env = { DB: sqliteD1(workerFile('schema.sql')).d1, POD: podNamespace(async () => Response.json({})), get CLOCK_MS(): number { throw new Error(boom); } } as unknown as Env; // the clock is read on every route but /healthz
  const log = console.error;
  console.error = () => {};
  try {
    const res = await handle(new Request('https://w.example/v1/pods'), env);
    const text = await res.text();
    expect(res.status).toBe(500);
    expect(res.headers.get('Referrer-Policy')).toBe('no-referrer');
    expect((JSON.parse(text) as { code: string }).code).toBe('internal');
    expect(text).not.toContain(boom);
  } finally { console.error = log; }
});
