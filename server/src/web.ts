// The built web client (ADR 0006: the server serves it). With JM_WEB_DIR set, a GET no route answered gets the file of that
// path in the build, or the client's index.html so the client's router takes the path. The API paths keep their JSON 404, and
// so does every path while the build has no index.html: Bun is never handed a file that is not there.
import { realpathSync, statSync } from 'node:fs';
import { join, resolve, sep } from 'node:path';
import type { Hono } from 'hono';

const API = /^\/(v1|__test|mcp|healthz)(\/|$)/;
const HEADERS = { index: 'no-cache', asset: 'public, max-age=31536000, immutable' };

/** A regular file there; an ENOTDIR or any other stat failure is not one. */
function isFile(path: string): boolean {
  try { return statSync(path, { throwIfNoEntry: false })?.isFile() === true; } catch { return false; }
}

/** Why `dir` cannot be served (the startup refusal), or undefined. */
export function webDirProblem(dir: string): string | undefined {
  return isFile(join(dir, 'index.html')) ? undefined : `JM_WEB_DIR has no index.html: build the web client into it first (${dir}).`;
}

/** The regular file under `root` (a real path) that `path` names, symlinks resolved, or null for anything else. */
function fileOf(root: string, path: string): string | null {
  let rel: string;
  try { rel = decodeURIComponent(path); } catch { return null; }
  if (rel.includes('\0')) return null;
  const file = resolve(join(root, rel));
  if (!file.startsWith(root + sep)) return null;
  let real: string;
  try { real = realpathSync(file); } catch { return null; }
  return real.startsWith(root + sep) && isFile(real) ? real : null;
}

export function webRoutes(app: Hono, dir: string): void {
  const problem = webDirProblem(dir);
  if (problem !== undefined) throw new Error(problem);
  const root = realpathSync(resolve(dir));
  app.get('*', async (c, next) => {
    if (API.test(c.req.path)) return next();
    const file = fileOf(root, c.req.path) ?? fileOf(root, '/index.html');
    if (file === null) return next();
    const cache = file === join(root, 'index.html') ? HEADERS.index : c.req.path.startsWith('/assets/') ? HEADERS.asset : undefined;
    return new Response(Bun.file(file), { headers: { 'x-content-type-options': 'nosniff', ...(cache === undefined ? {} : { 'cache-control': cache }) } });
  });
}
