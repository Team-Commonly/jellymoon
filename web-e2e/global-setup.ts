// Starts one pod-state server for the browser specs: ENV=local, a fresh PGlite data dir, a free port, and the built web client
// served by the server itself (ADR 0006). The specs read its address from JM_WEB_E2E_BASE; the returned function stops it.
import { spawn } from 'node:child_process';
import { existsSync, mkdtempSync, rmSync } from 'node:fs';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = fileURLToPath(new URL('..', import.meta.url));

function freePort(): Promise<number> {
  return new Promise((done, fail) => {
    const s = createServer();
    s.once('error', fail);
    s.listen(0, '127.0.0.1', () => {
      const port = (s.address() as { port: number }).port;
      s.close(() => done(port));
    });
  });
}

/** Waits on GET /healthz, failing at once if the server exits first, and after 60 s. */
async function healthy(base: string, exited: () => string | null): Promise<void> {
  for (const until = Date.now() + 60_000; Date.now() < until;) {
    const gone = exited();
    if (gone !== null) throw new Error(`the server exited before /healthz answered: ${gone}`);
    if (await fetch(`${base}/healthz`).then((r) => r.ok, () => false)) return;
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error('no /healthz 200 within 60 s');
}

export default async function globalSetup(): Promise<() => Promise<void>> {
  const web = join(repo, 'web', 'dist');
  if (!existsSync(join(web, 'index.html'))) throw new Error('web/dist/index.html is missing: build the client first (make e2e-web does)');
  const dir = mkdtempSync(join(tmpdir(), 'jm-web-e2e-'));
  const port = await freePort();
  // Never a real database: ENV=local with DATABASE_URL is refused at startup, and these specs want the in-process one.
  const env: Record<string, string | undefined> = { ...process.env, ENV: 'local', PORT: String(port), JM_WEB_DIR: web, DATABASE_URL: undefined };
  const child = spawn(process.env.BUN ?? 'bun', ['server/src/main.ts', '--persist-to', join(dir, 'state')], { cwd: repo, env, stdio: ['ignore', 'pipe', 'pipe'] });
  let log = '';
  let exit: string | null = null;
  child.stdout?.on('data', (d: unknown) => { log += String(d); });
  child.stderr?.on('data', (d: unknown) => { log += String(d); });
  const stopped = new Promise<void>((done) => child.on('exit', (code: unknown, signal: unknown) => { exit = `code ${String(code)} signal ${String(signal)}`; done(); }));
  const base = `http://127.0.0.1:${port}`;
  try {
    await healthy(base, () => (exit === null ? null : `${exit}\n${log}`));
  } catch (e) {
    child.kill('SIGKILL');
    throw e;
  }
  process.env.JM_WEB_E2E_BASE = base;
  return async () => {
    if (exit === null) child.kill('SIGTERM');
    await stopped;
    if (dir.startsWith(tmpdir())) rmSync(dir, { recursive: true, force: true });
  };
}
