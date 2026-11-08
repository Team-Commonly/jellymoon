// Routes only the Worker calls, under /_in/ on the pod object (a public request never reaches them: the Worker has no such path).
// They exist because some steps start outside the pod (a run is claimed by a wake id that names no pod; an invite is found by code).
import { RunOutcomeValues } from '../../gen/ts/protocol';
import { isoOf } from '../../podcore/src/index';
import type { RunClaimed } from '../../podcore/src/index';
import { bad, fail, intIn, parse, pick, respond, str } from './wire';
import type { Body, PodPort } from './wire';

export interface Starter extends PodPort { begin(podId: string): void }

const podView = (p: PodPort) => ({ id: p.id, name: p.meta.name, createdAt: p.meta.createdAt });
const isMember = (p: PodPort, userId: string): boolean => p.state.members.some((m) => m.id === userId);

function create(pod: Starter, b: Body): Response {
  const podId = str(b.podId, 1, 64);
  const name = str(b.name, 1, 80);
  const ownerId = str(b.ownerId, 1, 64);
  const ownerName = str(b.ownerName, 1, 80);
  if (podId === undefined || name === undefined || ownerId === undefined || ownerName === undefined) return bad('podId, name, ownerId and ownerName are required.');
  pod.begin(podId);
  return respond(pod.exec({ type: 'pod_create', ownerId, ownerName }, { meta: (m) => { m.name = name; m.createdAt = isoOf(pod.now()); } }), () => podView(pod));
}

/** The Worker has validated the request and hashed the code; the pod stores the hash and returns the code once, in this reply. */
function invite(pod: PodPort, b: Body): Response {
  const code = str(b.code, 1, 64);
  const hash = str(b.hash, 64, 64);
  const userId = str(b.userId, 1, 64);
  const base = str(b.base, 1, 2048);
  const uses = intIn(b.maxUses, 1, 1, 100);
  const ttl = intIn(b.ttlSeconds, 604800, 60, 2592000);
  if (code === undefined || hash === undefined || userId === undefined || base === undefined || uses === null || ttl === null) return bad('code, hash, userId and base are required.');
  if (!isMember(pod, userId)) return fail('forbidden', 'Only a member can invite.');
  const expiresAt = pod.now() + ttl * 1000;
  pod.sql.exec('insert into invites (hash, uses_left, expires_at) values (?, ?, ?)', hash, uses, expiresAt);
  return Response.json({ code, url: `${base.replace(/\/+$/, '')}/join/${code}`, expiresAt: isoOf(expiresAt), usesLeft: uses });
}

function accept(pod: PodPort, b: Body): Response {
  const hash = str(b.hash, 64, 64);
  const userId = str(b.userId, 1, 64);
  const name = str(b.name, 1, 80);
  if (hash === undefined || userId === undefined || name === undefined) return bad('hash, userId and name are required.');
  const reply = pod.exec({ type: 'member_add', userId, name }, {
    pre: (s) => {
      if (s.members.some((m) => m.id === userId)) return { ok: true, result: null };
      const [inv] = pod.sql.exec<{ uses_left: number; expires_at: number }>('select uses_left, expires_at from invites where hash = ?', hash).toArray();
      return inv !== undefined && inv.uses_left > 0 && inv.expires_at > pod.now() ? null : { ok: false, error: { code: 'gone', message: 'This invite is used or expired.' } };
    },
    extra: () => { pod.sql.exec('update invites set uses_left = uses_left - 1 where hash = ?', hash); },
  });
  return respond(reply, () => ({ pod: podView(pod) }));
}

/** Wake ids are per pod, so the Worker asks every pod where the computer hosts an agent whether the wake is its own. */
function hasWake(pod: PodPort, b: Body): Response {
  const has = Object.values(pod.state.wakes).some((w) => w.id === b.wakeId && pod.state.agents[w.agentId]?.computerId === b.computerId);
  return Response.json({ has });
}

function brief(pod: PodPort, runId: string, r: RunClaimed): string {
  const refs = Object.entries(r.refs).map(([k, v]) => `${k} ${v}`).join(', ');
  return `You are ${pod.state.runs[runId].agent.displayName}, woken in the pod "${pod.meta.name}" (${r.reason}${refs ? `: ${refs}` : ''}). Call jm_whoami, then jm_task_list, claim a task before working on it and report with jm_post.`;
}

function runClaim(pod: PodPort, b: Body): Response {
  const wakeId = str(b.wakeId, 1, 64);
  const computerId = str(b.computerId, 1, 64);
  const runId = str(b.runId, 1, 64);
  if (wakeId === undefined || computerId === undefined || runId === undefined) return bad('wakeId, computerId and runId are required.');
  return respond(pod.exec({ type: 'run_claim', wakeId, computerId, runId }), (r) => ({ runId, brief: brief(pod, runId, r as RunClaimed), expiresAt: (r as RunClaimed).expiresAt }));
}

function runEnd(pod: PodPort, b: Body): Response {
  const outcome = pick(b.outcome, RunOutcomeValues);
  const runId = str(b.runId, 1, 64);
  const computerId = str(b.computerId, 1, 64);
  if (outcome === undefined || runId === undefined || computerId === undefined) return bad('outcome must be completed, failed or cancelled.');
  return respond(pod.exec({ type: 'run_end', runId, computerId, outcome }));
}

/** The MCP adapter answered a tool call on a run's token; the pod counts it for the receipt. */
function toolCall(pod: PodPort, b: Body): Response {
  const runId = str(b.runId, 1, 64);
  return runId === undefined ? bad('runId is required.') : respond(pod.exec({ type: 'tool_call', runId }));
}

const TABLE: Record<string, (pod: Starter, b: Body) => Response> = {
  create, invite, accept, 'has-wake': hasWake, 'run-claim': runClaim, 'run-end': runEnd, 'tool-call': toolCall, ping: () => Response.json({}),
};

export async function internalRoute(pod: Starter, req: Request): Promise<Response | null> {
  const name = new URL(req.url).pathname.slice('/_in/'.length);
  const fn = Object.hasOwn(TABLE, name) ? TABLE[name] : undefined;
  if (fn === undefined || req.method !== 'POST') return null;
  const body = await parse(req);
  return body === null ? bad('The body must be a JSON object.') : fn(pod, body);
}
