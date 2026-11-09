import { expect, test } from 'bun:test';
import { mkdirSync, mkdtempSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { buildApp } from '../src/app';
import { appOptions, startupRefusal } from '../src/config';
import { openDb } from '../src/db';
import { Hub } from '../src/hub';

// The built web client is served by the server itself (ADR 0006), from JM_WEB_DIR; the API keeps its JSON 404.
const dir = new URL('./web-fixture', import.meta.url).pathname;
const INDEX = '<!doctype html>\n<title>fixture client</title>\n';

const apps = new Map<string | undefined, ReturnType<typeof buildApp>>();
async function get(webDir: string | undefined, path: string, method = 'GET'): Promise<{ status: number; type: string; text: string; headers: Headers }> {
  if (!apps.has(webDir)) apps.set(webDir, buildApp(await Hub.restore(await openDb({})), { local: true, webDir }));
  const res = await apps.get(webDir)!.fetch(new Request(`http://x${path}`, { method }));
  return { status: res.status, type: res.headers.get('content-type') ?? '', text: await res.text(), headers: res.headers };
}
const code = (text: string): string => (JSON.parse(text) as { code: string }).code;

/** A build in a fresh temp dir: index.html and assets/app.css, plus outside.txt beside it (not in the build). */
function tempBuild(): { tmp: string; web: string } {
  const tmp = mkdtempSync(join(tmpdir(), 'jm-web-test-'));
  const web = join(tmp, 'web');
  mkdirSync(join(web, 'assets'), { recursive: true });
  writeFileSync(join(web, 'index.html'), INDEX);
  writeFileSync(join(web, 'assets', 'app.css'), 'body { margin: 0; }\n');
  writeFileSync(join(tmp, 'outside.txt'), 'SECRET\n');
  return { tmp, web };
}

test('a client route gets index.html, and a file of the build is served as itself', async () => {
  for (const path of ['/', '/pod/pod_1', '/invite/abc?x=1', '/assets', '/assets/missing.js']) {
    const r = await get(dir, path);
    expect(`${path} ${r.status} ${r.type.split(';')[0]} ${r.text}`).toBe(`${path} 200 text/html ${INDEX}`);
  }
  const css = await get(dir, '/assets/app.css');
  expect(`${css.status} ${css.type.split(';')[0]} ${css.text}`).toBe('200 text/css body { margin: 0; }\n');
});

test('index.html is never cached, a hashed asset always is, and nothing is sniffed', async () => {
  for (const path of ['/', '/pod/pod_1', '/assets/missing.js']) {
    const r = await get(dir, path);
    expect(`${path} ${r.headers.get('cache-control')} ${r.headers.get('x-content-type-options')}`).toBe(`${path} no-cache nosniff`);
  }
  const css = await get(dir, '/assets/app.css');
  expect(`${css.headers.get('cache-control')} ${css.headers.get('x-content-type-options')}`).toBe('public, max-age=31536000, immutable nosniff');
});

test('nothing outside the build is served, and the API paths keep their JSON 404', async () => {
  for (const path of ['/%2e%2e/web.test.ts', '/..%2fweb.test.ts', '/..%2fweb-fixture-sibling/secret.txt', '/%2e%2e%2f%2e%2e/src/app.ts', '/%E0%A4%A']) {
    const r = await get(dir, path);
    expect(`${path} ${r.status} ${r.text}`).toBe(`${path} 200 ${INDEX}`);
  }
  for (const path of ['/v1', '/v1/nope', '/v1/pods/pod_1/nope', '/__test/nope', '/mcp']) {
    const r = await get(dir, path);
    expect(`${path} ${r.status} ${code(r.text)}`).toBe(`${path} 404 not_found`);
  }
  expect((await get(dir, '/healthz')).text).toBe('ok');
  expect((await get(dir, '/pod/pod_1', 'POST')).status).toBe(404);
});

test('a NUL byte in the path is index.html, never a server error', async () => {
  for (const path of ['/%00', '/assets/app.css%00', '/a%00b/c']) {
    const r = await get(dir, path);
    expect(`${path} ${r.status} ${r.text}`).toBe(`${path} 200 ${INDEX}`);
  }
});

test('a symlink inside the build that points outside it is not followed', async () => {
  const { tmp, web } = tempBuild();
  try {
    symlinkSync('../outside.txt', join(web, 'link.txt'));
    symlinkSync('..', join(web, 'up'));
    for (const path of ['/link.txt', '/up/outside.txt']) {
      const r = await get(web, path);
      expect(`${path} ${r.status} ${r.text}`).toBe(`${path} 200 ${INDEX}`);
    }
    expect((await get(web, '/assets/app.css')).text).toBe('body { margin: 0; }\n');
  } finally { rmSync(tmp, { recursive: true, force: true }); }
});

test('a JM_WEB_DIR with no index.html is refused at startup', async () => {
  const { tmp, web } = tempBuild();
  try {
    for (const bad of [join(tmp, 'nowhere'), join(web, 'assets'), join(web, 'index.html')]) {
      expect(startupRefusal({ ENV: 'local', JM_WEB_DIR: bad }) ?? 'no refusal').toBe(`JM_WEB_DIR has no index.html: build the web client into it first (${bad}).`);
      const hub = await Hub.restore(await openDb({}));
      expect(() => buildApp(hub, { local: true, webDir: bad })).toThrow();
    }
    expect(startupRefusal({ ENV: 'local', JM_WEB_DIR: web }) === undefined).toBe(true);
  } finally { rmSync(tmp, { recursive: true, force: true }); }
});

test('a build that loses its index.html after startup answers the JSON 404, never a missing file', async () => {
  const { tmp, web } = tempBuild();
  try {
    expect((await get(web, '/pod/pod_1')).text).toBe(INDEX);
    rmSync(join(web, 'index.html'), { recursive: false, force: true });
    for (const path of ['/', '/pod/pod_1', '/index.html', '/assets/missing.js']) {
      const r = await get(web, path);
      expect(`${path} ${r.status} ${code(r.text)}`).toBe(`${path} 404 not_found`);
    }
    expect((await get(web, '/assets/app.css')).text).toBe('body { margin: 0; }\n');
  } finally { rmSync(tmp, { recursive: true, force: true }); }
});

test('with no JM_WEB_DIR the server serves no client', async () => {
  const r = await get(undefined, '/pod/pod_1');
  expect(`${r.status} ${code(r.text)}`).toBe('404 not_found');
  expect(appOptions({ JM_WEB_DIR: '/srv/web' }).webDir).toBe('/srv/web');
  expect(appOptions({ JM_WEB_DIR: '' }).webDir).toBe(undefined);
  expect(appOptions({}).webDir).toBe(undefined);
});

test('every reply says no-referrer, so the one-time code in a /device or /invite URL is never sent on', async () => {
  for (const path of ['/device?code=ABCD-EFGH', '/invite/tok_secret', '/assets/app.css', '/v1/nope', '/healthz']) {
    const r = await get(dir, path);
    expect(`${path} ${r.headers.get('referrer-policy')}`).toBe(`${path} no-referrer`);
  }
});

test('a route that throws still says no-referrer on the onError reply', async () => {
  const app = buildApp(await Hub.restore(await openDb({})), { local: true });
  app.get('/boom', () => { throw new Error('boom'); }); // registered on the test's own instance, after the middleware
  const errors = console.error;
  console.error = () => undefined; // onError logs the throw
  try {
    const res = await app.fetch(new Request('http://x/boom'));
    expect(`${res.status} ${res.headers.get('referrer-policy')}`).toBe('500 no-referrer');
    expect(code(await res.text())).toBe('internal');
  } finally {
    console.error = errors;
  }
});
