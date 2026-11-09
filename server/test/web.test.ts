import { expect, test } from 'bun:test';
import { buildApp } from '../src/app';
import { appOptions } from '../src/config';
import { openDb } from '../src/db';
import { Hub } from '../src/hub';

// The built web client is served by the server itself (ADR 0006), from JM_WEB_DIR; the API keeps its JSON 404.
const dir = new URL('./web-fixture', import.meta.url).pathname;
const INDEX = '<!doctype html>\n<title>fixture client</title>\n';

const apps = new Map<string | undefined, ReturnType<typeof buildApp>>();
async function get(webDir: string | undefined, path: string, method = 'GET'): Promise<{ status: number; type: string; text: string }> {
  if (!apps.has(webDir)) apps.set(webDir, buildApp(await Hub.restore(await openDb({})), { local: true, webDir }));
  const res = await apps.get(webDir)!.fetch(new Request(`http://x${path}`, { method }));
  return { status: res.status, type: res.headers.get('content-type') ?? '', text: await res.text() };
}

test('a client route gets index.html, and a file of the build is served as itself', async () => {
  for (const path of ['/', '/pod/pod_1', '/invite/abc?x=1', '/assets', '/assets/missing.js']) {
    const r = await get(dir, path);
    expect(`${path} ${r.status} ${r.type.split(';')[0]} ${r.text}`).toBe(`${path} 200 text/html ${INDEX}`);
  }
  const css = await get(dir, '/assets/app.css');
  expect(`${css.status} ${css.type.split(';')[0]} ${css.text}`).toBe('200 text/css body { margin: 0; }\n');
});

test('nothing outside the build is served, and the API paths keep their JSON 404', async () => {
  for (const path of ['/%2e%2e/web.test.ts', '/..%2fweb.test.ts', '/..%2fweb-fixture-sibling/secret.txt', '/%2e%2e%2f%2e%2e/src/app.ts', '/%E0%A4%A']) {
    const r = await get(dir, path);
    expect(`${path} ${r.status} ${r.text}`).toBe(`${path} 200 ${INDEX}`);
  }
  for (const path of ['/v1', '/v1/nope', '/v1/pods/pod_1/nope', '/__test/nope', '/mcp']) {
    const r = await get(dir, path);
    expect(`${path} ${r.status} ${(JSON.parse(r.text) as { code: string }).code}`).toBe(`${path} 404 not_found`);
  }
  expect((await get(dir, '/healthz')).text).toBe('ok');
  expect((await get(dir, '/pod/pod_1', 'POST')).status).toBe(404);
});

test('with no JM_WEB_DIR the server serves no client', async () => {
  const r = await get(undefined, '/pod/pod_1');
  expect(`${r.status} ${(JSON.parse(r.text) as { code: string }).code}`).toBe('404 not_found');
  expect(appOptions({ JM_WEB_DIR: '/srv/web' }).webDir).toBe('/srv/web');
  expect(appOptions({ JM_WEB_DIR: '' }).webDir).toBe(undefined);
  expect(appOptions({}).webDir).toBe(undefined);
});
