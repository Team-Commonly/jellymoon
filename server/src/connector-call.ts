// The agent's connector_call and the write gate (ADR 0009). The run's agent's grant and the pinned class of the operation
// decide: a read is called at once; a write is parked as the agent's own ask, with its exact arguments, and released at most
// once by a member's ruling of approve. Every call that names a connection leaves a server-stamped receipt row.
import type { Actor, ConnectorCallInput, ConnectorReceipt, ConnectorVendor, Harness, PostInput } from '../../gen/ts/protocol';
import { isoOf } from '../../podcore/src/index';
import type { Cmd } from '../../podcore/src/index';
import { argsHash, opOf, pinOf, scrub } from './catalogue';
import type { PinOp } from './catalogue';
import { aadOf, agentOf, noVault } from './connector-kit';
import type { Kit } from './connector-kit';
import { newId, newWakeSeed } from './ids';
import { bad, fail, respond } from './kit';
import type { Call } from './kit';
import { viaMcp } from './mcp';
import type { Opts, PodRuntime } from './pod';
import { gate } from './routes-ops';
import { callsOf, claimParked, connectionsOf, findConnection, findGrant, finishCall, saveCall } from './store';
import type { CallRow, ConnRow } from './store';
import { adapterOf } from './upstream';

/** A parked write expires 24 hours after it was parked: its decision becomes expired and nothing is released. */
const PARK_MS = 24 * 60 * 60 * 1000;
const MAX_BODY = 16384;

/** The connection the call names: exactly one of connectionId or vendor (the pod's one connection to that vendor). */
async function connectionFor(kit: Kit, k: Call, b: ConnectorCallInput): Promise<ConnRow | Response> {
  if ((b.connectionId === undefined) === (b.vendor === undefined)) return bad(k.c, 'Name exactly one of connectionId or vendor.');
  if (b.connectionId !== undefined) return (await findConnection(kit.hub.db, k.pod.id, b.connectionId)) ?? fail(k.c, 'not_found', 'No such connection in this pod.', b.connectionId);
  const all = (await connectionsOf(kit.hub.db, k.pod.id)).filter((r) => r.vendor === b.vendor);
  if (all.length > 1) return bad(k.c, `This pod has ${all.length} ${b.vendor} connections; name the connectionId.`);
  return all[0] ?? fail(k.c, 'not_found', `This pod has no ${b.vendor} connection.`);
}

/** The ask a parked write becomes: the agent's own, options a approve and b reject, the exact arguments in a fenced block. */
function cardOf(vendor: string, op: string, args: Record<string, unknown>, hash: string): PostInput | null {
  const json = JSON.stringify(args, null, 2);
  const fence = '`'.repeat(Math.max(3, ...(json.match(/`+/g) ?? []).map((s) => s.length + 1)));
  const body = `The agent asked the pod to call ${op} on ${vendor}. Approve makes exactly this one call, with these arguments as written:\n\n${fence}json\n${json}\n${fence}\n\nargsHash \`${hash}\`. Reject, or no ruling within 24 hours, sends nothing.`;
  if (body.length > MAX_BODY) return null;
  const claim = (text: string) => ({ text });
  return {
    summary: `Approve ${vendor} ${op}?`, kind: 'ask', status: 'blocked', needs: { to: { role: 'owner' }, action: 'approve' }, body,
    options: [
      { id: 'a', label: 'Approve', consequence: `The pod makes this one ${op} call on ${vendor} with exactly the arguments shown.` },
      { id: 'b', label: 'Reject', consequence: `Nothing is sent to ${vendor}.` },
    ],
    context: {
      happened: [claim(`The agent asked to call ${op} on ${vendor}; the arguments are in the body.`)],
      checked: [claim(`${op} is pinned as a write, and the agent holds a write-with-confirm grant on this connection.`)],
      stakes: claim(`Approving sends one write to ${vendor} with the credential the pod holds.`),
      recommendation: { option: 'b', why: claim('Reject unless the arguments shown are exactly what should be sent.') },
    },
  };
}

async function read(kit: Kit, k: Call, conn: ConnRow, op: PinOp, row: CallRow, args: Record<string, unknown>): Promise<Response> {
  const token = await kit.vault!.open(conn, aadOf(conn));
  const res = scrub(await adapterOf(pinOf(conn.vendor as ConnectorVendor)).call(conn.url, token, op.name, args), token);
  await saveCall(kit.hub.db, { ...row, outcome: res.ok ? 'ok' : 'failed', upstreamStatus: res.status, settledAt: kit.hub.now() });
  if (!res.ok) return fail(k.c, res.code ?? 'upstream_failed', `${conn.vendor} ${op.name} failed (upstream status ${res.status}): ${res.error ?? ''}`.slice(0, 400), row.id);
  return k.c.json({ state: 'done', receiptId: row.id, result: res.result ?? {} });
}

async function park(kit: Kit, k: Call, conn: ConnRow, op: PinOp, row: CallRow, args: Record<string, unknown>): Promise<Response> {
  const input = cardOf(conn.vendor, op.name, args, row.argsHash);
  if (input === null) return bad(k.c, 'The arguments are too large to show on an ask.');
  const decisionId = newId('dec_');
  const expiresAt = kit.hub.now() + PARK_MS;
  const harness = k.pod.meta.harness[row.agentId] as Harness | undefined;
  const reply = await k.pod.exec(
    { type: 'post', caller: k.caller, messageId: newId('msg_'), decisionId, wakeId: newWakeSeed(), harness, viaMcp: viaMcp(k.c.req.raw), expiresAt, input },
    { extra: async (t) => { await saveCall(t, { ...row, outcome: 'parked', decisionId, expiresAt }); } },
  );
  return reply?.ok ? k.c.json({ state: 'parked', receiptId: row.id, decisionId }) : respond(k.c, reply);
}

/** POST /v1/pods/:pod/connectors/call, run tokens only. */
export const connectorCall = (kit: Kit) => async (k: Call): Promise<Response> => {
  const blocked = await gate(k);
  if (blocked) return blocked;
  if (kit.vault === undefined) return noVault(k);
  const b = k.body as unknown as ConnectorCallInput;
  const conn = await connectionFor(kit, k, b);
  if (conn instanceof Response) return conn;
  const agentId = agentOf(k) as string;
  const args = b.args as Record<string, unknown>;
  const row: CallRow = {
    id: newId('crc_'), pod: k.pod.id, connection: conn.id, operation: b.operation, args: JSON.stringify(args), argsHash: argsHash(args), runId: (k.caller as { runId: string }).runId,
    agentId, outcome: 'refused', upstreamStatus: null, decisionId: null, approver: null, result: null, createdAt: kit.hub.now(), settledAt: null, expiresAt: null,
  };
  const refuse = async (code: 'not_found' | 'forbidden', why: string): Promise<Response> => {
    await saveCall(kit.hub.db, { ...row, settledAt: kit.hub.now() });
    return fail(k.c, code, why, row.id);
  };
  const op = opOf(pinOf(conn.vendor as ConnectorVendor), b.operation);
  if (op === undefined) return refuse('not_found', `${b.operation} is not in the pinned ${conn.vendor} catalogue.`);
  const g = await findGrant(kit.hub.db, conn.id, agentId);
  if (g === undefined) return refuse('forbidden', 'Your agent holds no grant on this connection.');
  if (op.class === 'read') return read(kit, k, conn, op, row, args);
  if (g.access !== 'write-with-confirm') return refuse('forbidden', `${op.name} is a write, and your agent's grant is read only.`);
  return park(kit, k, conn, op, row, args);
};

/** The one upstream call a ruling of approve released. Its answer is recorded; it never throws. */
async function release(kit: Kit, row: CallRow): Promise<void> {
  const conn = await findConnection(kit.hub.db, row.pod, row.connection);
  if (conn === undefined || kit.vault === undefined) return void (await finishCall(kit.hub.db, row.id, 'failed', null, null));
  const token = await kit.vault.open(conn, aadOf(conn));
  const res = scrub(await adapterOf(pinOf(conn.vendor as ConnectorVendor)).call(conn.url, token, row.operation, JSON.parse(row.args) as Record<string, unknown>), token);
  await finishCall(kit.hub.db, row.id, res.ok ? 'released' : 'failed', res.status, res.ok ? JSON.stringify(res.result ?? {}) : null);
}

/**
 * What a ruling does to a parked write (ws.ts passes these with every `rule`). Only a pod owner, the role that connects and
 * grants, rules on a connector write's ask; anyone else is refused forbidden before the core sees the ruling. In the ruling's
 * own transaction the parked row is claimed, released (option a) or rejected (option b); after it commits and before members
 * hear of the ruling, a released row makes its one call. A decision with no connector row claims nothing.
 */
export function rulingOpts(kit: Kit, pod: PodRuntime, cmd: Extract<Cmd, { type: 'rule' }>): Opts {
  const caller = cmd.caller;
  if (caller.kind !== 'user') return {};
  const approve = cmd.option === 'a';
  const by: Actor | undefined = pod.state.members.find((m) => m.id === caller.userId);
  let claimed: CallRow | undefined;
  return {
    pre: async (s) => {
      if (s.owners.some((o) => o.id === caller.userId) || (await callsOf(kit.hub.db, pod.id, cmd.decisionId, 1)).length === 0) return null;
      return { ok: false, error: { code: 'forbidden', message: 'Only a pod owner rules on a connector write.' } };
    },
    extra: async (t) => { claimed = await claimParked(t, cmd.decisionId, approve ? 'released' : 'rejected', approve && by ? JSON.stringify(by) : null, kit.hub.now()); },
    after: async () => { if (claimed !== undefined && approve) await release(kit, claimed); },
  };
}

/** A stored call as its receipt. A parked write past its expiry reads as expired, settled at its expiry. */
export function receiptView(r: CallRow, now: number): ConnectorReceipt {
  const expired = r.outcome === 'parked' && r.expiresAt !== null && r.expiresAt <= now;
  const out: ConnectorReceipt = {
    id: r.id, podId: r.pod, connectionId: r.connection, operation: r.operation, argsHash: r.argsHash, runId: r.runId, agentId: r.agentId,
    outcome: (expired ? 'expired' : r.outcome) as ConnectorReceipt['outcome'], createdAt: isoOf(Number(r.createdAt)),
  };
  if (r.upstreamStatus !== null) out.upstreamStatus = Number(r.upstreamStatus);
  if (r.decisionId !== null) out.decisionId = r.decisionId;
  if (r.approver !== null) out.approver = JSON.parse(r.approver) as Actor;
  if (r.result !== null) out.result = JSON.parse(r.result) as Record<string, unknown>;
  const settled = expired ? r.expiresAt : r.settledAt;
  if (settled !== null) out.settledAt = isoOf(Number(settled));
  if (r.expiresAt !== null) out.expiresAt = isoOf(Number(r.expiresAt));
  return out;
}
