// The built web client (ADR 0006: the server serves it). With JM_WEB_DIR set, a GET no route answered gets the file of that
// path in the build, or the client's index.html so the client's router takes the path. The API paths keep their JSON 404.
import { statSync } from 'node:fs';
import { join, resolve, sep } from 'node:path';
import type { Hono } from 'hono';

const API = /^\/(v1|__test|mcp|healthz)(\/|$)/;

/** The file under `root` that `path` names, or null for anything that is not a file inside it. */
function fileOf(root: string, path: string): string | null {
  let rel: string;
  try { rel = decodeURIComponent(path); } catch { return null; }
  const file = resolve(join(root, rel));
  if (!file.startsWith(root + sep)) return null;
  return statSync(file, { throwIfNoEntry: false })?.isFile() ? file : null;
}

export function webRoutes(app: Hono, dir: string): void {
  const root = resolve(dir);
  const index = join(root, 'index.html');
  app.get('*', async (c, next) => {
    if (API.test(c.req.path)) return next();
    return new Response(Bun.file(fileOf(root, c.req.path) ?? index));
  });
}
