// The connector routes (ADR 0009): a pod owner connects a vendor and grants an agent access; members and runs list the
// connections and read the receipts. The agent's call and the write gate are in connector-call.ts.
import type { Hono } from 'hono';
import type { Connection, ConnectorAccess, ConnectorVendor, ConnectorView, Grant } from '../../gen/ts/protocol';
import { isoOf } from '../../podcore/src/index';
import { driftOf, pinOf, scrub, view } from './catalogue';
import { connectorCall, receiptView } from './connector-call';
import { aadOf, agentOf, destFor, noVault } from './connector-kit';
import type { Kit } from './connector-kit';
import { newId } from './ids';
import { bad, fail, intIn, memberRoute, podRoute } from './kit';
import type { Call } from './kit';
import { gate } from './routes-ops';
import { callsOf, connectionsOf, findConnection, findGrant, saveConnection, saveGrant } from './store';
import type { ConnRow } from './store';
import { adapterOf } from './upstream';

const isOwner = (k: Call): boolean => k.caller.kind === 'user' && k.pod.state.owners.some((o) => o.id === (k.caller as { userId: string }).userId);
const notOwner = (k: Call): Response => fail(k.c, 'forbidden', 'Only a pod owner connects a vendor or grants access.');

const connectionView = (r: ConnRow): Connection => ({
  id: r.id, vendor: r.vendor as ConnectorVendor, url: r.url, createdAt: isoOf(r.createdAt), operations: pinOf(r.vendor as ConnectorVendor).operations.map(view),
});

const connect = (kit: Kit) => async (k: Call): Promise<Response> => {
  if (!isOwner(k)) return notOwner(k);
  if (kit.vault === undefined) return noVault(k);
  const { vendor, url, token } = k.body as { vendor: ConnectorVendor; url: string; token: string };
  const dest = await destFor(kit, { url });
  if (typeof dest === 'string') return bad(k.c, dest);
  const pin = pinOf(vendor);
  const listed = await adapterOf(pin).list(dest, token);
  if (!listed.ok) return fail(k.c, listed.code ?? 'upstream_failed', `Listing the upstream's operations failed (upstream status ${listed.status}): ${scrub(listed.error, token)}`.slice(0, 400));
  const drift = driftOf(pin, listed.ops);
  if (drift !== null) return fail(k.c, 'connector_drift', scrub(drift, token).slice(0, 400));
  const id = newId('con_');
  const row: ConnRow = { id, pod: k.pod.id, vendor, url, createdAt: kit.hub.now(), ...(await kit.vault.seal(token, aadOf({ pod: k.pod.id, id }))) };
  await saveConnection(kit.hub.db, row);
  return k.c.json(connectionView(row));
};

const grant = (kit: Kit) => async (k: Call): Promise<Response> => {
  if (!isOwner(k)) return notOwner(k);
  const { connectionId, agentId, access } = k.body as { connectionId: string; agentId: string; access: ConnectorAccess };
  if ((await findConnection(kit.hub.db, k.pod.id, connectionId)) === undefined) return fail(k.c, 'not_found', 'No such connection in this pod.', connectionId);
  if (!Object.hasOwn(k.pod.state.agents, agentId)) return fail(k.c, 'not_found', 'No such agent in this pod.', agentId);
  const g = { connection: connectionId, agent: agentId, access, createdAt: kit.hub.now() };
  await saveGrant(kit.hub.db, g);
  const out: Grant = { connectionId, agentId, access, createdAt: isoOf(g.createdAt) };
  return k.c.json(out);
};

/** The pinned catalogue filtered by the caller's grant: a run sees only granted connections, and only reads on a read grant. */
const list = (kit: Kit) => async (k: Call): Promise<Response> => {
  const blocked = await gate(k);
  if (blocked) return blocked;
  const agent = agentOf(k);
  const out: ConnectorView[] = [];
  for (const r of await connectionsOf(kit.hub.db, k.pod.id)) {
    const ops = pinOf(r.vendor as ConnectorVendor).operations;
    if (agent === undefined) { out.push({ connectionId: r.id, vendor: r.vendor as ConnectorVendor, operations: ops.map(view) }); continue; }
    const g = await findGrant(kit.hub.db, r.id, agent);
    if (g === undefined) continue;
    const allowed = ops.filter((o) => g.access === 'write-with-confirm' || o.class === 'read');
    out.push({ connectionId: r.id, vendor: r.vendor as ConnectorVendor, access: g.access as ConnectorAccess, operations: allowed.map(view) });
  }
  return k.c.json({ connections: out });
};

const DECISION = /^dec_[0-9A-HJKMNP-TV-Z]{26}$/;

const receipts = (kit: Kit) => async (k: Call): Promise<Response> => {
  const dec = k.c.req.query('decisionId');
  const raw = k.c.req.query('limit');
  const limit = intIn(raw === undefined ? undefined : Number(raw), 20, 1, 100);
  if ((dec !== undefined && !DECISION.test(dec)) || limit === null) return bad(k.c, 'decisionId must be a decision id and limit an integer from 1 to 100.');
  const blocked = await gate(k);
  if (blocked) return blocked;
  const rows = await callsOf(kit.hub.db, k.pod.id, dec, limit);
  return k.c.json({ receipts: rows.map((r) => receiptView(r, kit.hub.now())) });
};

export function connectorRoutes(app: Hono, kit: Kit): void {
  const { hub } = kit;
  app.post('/v1/pods/:pod/connections', podRoute(hub, ['user'] as const, 'ConnectRequest', [], connect(kit)));
  app.post('/v1/pods/:pod/grants', podRoute(hub, ['user'] as const, 'GrantRequest', [], grant(kit)));
  app.get('/v1/pods/:pod/connectors', memberRoute(hub, null, [], list(kit)));
  app.get('/v1/pods/:pod/connectors/receipts', memberRoute(hub, null, [], receipts(kit)));
  app.post('/v1/pods/:pod/connectors/call', podRoute(hub, ['run'] as const, 'ConnectorCallInput', [], connectorCall(kit)));
}
