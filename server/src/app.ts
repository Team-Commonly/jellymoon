// The Hono app: HTTP routes plus the pod socket, and the /__test routes when the build asks for them.
import { Hono } from 'hono';
import type { Context } from 'hono';
import { authRoutes, ownBase } from './auth';
import type { GitHub } from './auth';
import type { Hub } from './hub';
import { sameSecret } from './ids';
import { bad, fail, ownOrigin, parse, str } from './kit';
import { mcpRoute } from './mcp';
import { opRoutes } from './routes-ops';
import { setupAll } from './routes-setup';
import { wsRoute } from './ws';
import { connectorRoutes } from './connectors';
import { rulingOpts } from './connector-call';
import type { Vault } from './vault';
import { systemResolve } from './egress';
import type { Resolve } from './egress';

/**
 * `publicUrl` is the base of an invite's join link and a run's mcpUrl; a request's own origin stands in when unset.
 * Test routes (the Worker's rule, kept): ENV=local opens them all, including the clock. Otherwise they exist only when a
 * bearer is configured, each needs `Authorization: Bearer <bearer>`, and there is no clock route. With neither, the build has
 * no /__test handler at all.
 */
/** `resolve` is the connector egress resolver (egress.ts); tests inject one, the server uses the system's. */
export interface AppOpts { publicUrl?: string; local: boolean; testBearer?: string; github?: GitHub | null; vault?: Vault; resolve?: Resolve }

function testRoutes(app: Hono, hub: Hub, local: boolean, bearer: string | undefined): void {
  if (!local) {
    app.use('/__test/*', async (c, next) => (sameSecret(c.req.header('Authorization') ?? '', `Bearer ${bearer}`) ? next() : fail(c, 'unauthorized', 'Send the test bearer.')));
  }
  app.post('/__test/session', async (c) => {
    const name = str((await parse(c))?.name, 1, 80);
    return name === undefined ? bad(c, 'name is required.') : c.json(await hub.createUser(name));
  });
  app.post('/__test/evict/:pod', async (c: Context) => {
    const pod = hub.pods.get(c.req.param('pod') ?? '');
    if (pod === undefined) return fail(c, 'not_found', 'No such pod.');
    await pod.evict();
    return c.json({ evicted: true });
  });
  if (!local) return;
  app.post('/__test/clock', async (c) => {
    const now = (await parse(c))?.now;
    if (typeof now !== 'number' || !Number.isInteger(now) || now < 0) return bad(c, 'now is unix milliseconds.');
    await hub.setClock(now);
    return c.json({ now });
  });
  app.get('/__test/clock', (c) => c.json({ now: hub.now() }));
}

export function buildApp(hub: Hub, opts: AppOpts): Hono {
  const app = new Hono();
  app.use('*', async (c, next) => { ownOrigin.set(c.req.raw, new URL(ownBase(c, opts.publicUrl, opts.local)).origin); await next(); });
  app.get('/healthz', (c) => c.text('ok'));
  setupAll(app, hub, opts.publicUrl);
  authRoutes(app, hub, opts.github ?? null, opts.publicUrl, opts.local);
  opRoutes(app, hub);
  const kit = { hub, vault: opts.vault, local: opts.local, resolve: opts.resolve ?? systemResolve };
  connectorRoutes(app, kit); // ADR 0009
  wsRoute(app, hub, (pod, cmd) => rulingOpts(kit, pod, cmd));
  mcpRoute(app, hub); // twin of the Worker's POST /mcp (worker/src/mcp.ts)
  if (opts.local || (opts.testBearer !== undefined && opts.testBearer !== '')) testRoutes(app, hub, opts.local, opts.testBearer);
  app.notFound((c) => fail(c, 'not_found', 'No such route.'));
  app.onError((e, c) => { console.error('request failed', e); return fail(c, 'internal', 'The server failed.'); });
  return app;
}
