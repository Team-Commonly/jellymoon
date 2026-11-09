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
