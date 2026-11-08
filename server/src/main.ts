// The pod-state server: one Bun process serving every pod (ADR 0005).
// Env: PORT, ENV, DATABASE_URL, PUBLIC_URL, TEST_BEARER; for local runs JM_PGLITE_TEMPLATE and JM_PG_SCHEMA (see db.ts). Flag: --persist-to <dir> keeps the in-process Postgres (PGlite) there.
// Production runs against a real Postgres (DATABASE_URL). PGlite is for `make e2e` and a clean shell, so it needs ENV=local.
import { websocket } from 'hono/bun';
import { buildApp } from './app';
import { startupRefusal } from './config';
import { Hub } from './hub';

const port = Number(process.env.PORT ?? 8787);
const local = process.env.ENV === 'local';
const at = process.argv.indexOf('--persist-to');
const dir = at === -1 ? undefined : process.argv[at + 1];
const refusal = startupRefusal(process.env);
if (refusal !== undefined) {
  console.error(refusal);
  process.exit(1);
}
const hub = await Hub.open({ DATABASE_URL: process.env.DATABASE_URL, PGLITE_DIR: dir, PGLITE_TEMPLATE: process.env.JM_PGLITE_TEMPLATE, PG_SCHEMA: process.env.JM_PG_SCHEMA });
const app = buildApp(hub, { publicUrl: process.env.PUBLIC_URL, local, testBearer: process.env.TEST_BEARER });
Bun.serve({ port, fetch: app.fetch, websocket });
console.log(`jellymoon server on :${port}, ${hub.pods.size} pods restored`);
