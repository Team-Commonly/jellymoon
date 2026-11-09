// Pods, invites, computers, agents and run start and end. Each route names the request definition its body is checked against.
import type { Context, Hono } from 'hono';
import { isoOf } from '../../podcore/src/index';
import type { RunClaimed } from '../../podcore/src/index';
import type { Hub } from './hub';
import { hashToken, newId, newInviteCode, newToken } from './ids';
import { bad, fail, intIn, podRoute, respond, setup } from './kit';
import type { Call } from './kit';
import type { PodRuntime } from './pod';
import { findInvite, saveInvite, saveToken, useInvite } from './store';

const podView = (p: PodRuntime) => ({ id: p.id, name: p.meta.name, createdAt: p.meta.createdAt });

/** The public base URL: the run's mcpUrl and an invite's join link. An empty value counts as unset, and a trailing slash is dropped. A request's own origin stands in when none is configured. */
const baseOf = (publicUrl: string | undefined, c: Context): string => (publicUrl === undefined || publicUrl === '' ? new URL(c.req.url).origin : publicUrl).replace(/\/+$/, '');

function inviteRoutes(app: Hono, hub: Hub, publicUrl: string | undefined): void {
  const create = async (k: Call): Promise<Response> => {
    const uses = intIn(k.body.maxUses, 1, 1, 100);
    const ttl = intIn(k.body.ttlSeconds, 604800, 60, 2592000);
    if (!k.pod.isMember(k.caller.kind === 'user' ? k.caller.userId : '')) return fail(k.c, 'forbidden', 'Only a member can invite.');
    if (uses === null || ttl === null) return bad(k.c, 'maxUses is 1 to 100 and ttlSeconds is 60 to 2592000.');
    // The code is shown once, in this reply. Only its SHA-256 is stored, as with a token.
    const code = newInviteCode();
    const inv = { hash: hashToken(code), pod: k.pod.id, usesLeft: uses, expiresAt: hub.now() + ttl * 1000 };
    await saveInvite(hub.db, inv);
    return k.c.json({ code, url: `${baseOf(publicUrl, k.c)}/invite/${code}`, expiresAt: isoOf(inv.expiresAt), usesLeft: uses });
  };
  app.post('/v1/pods/:pod/invites', podRoute(hub, ['user'] as const, 'CreateInviteRequest', [], create));
  app.post('/v1/invites/:code/accept', setup(hub, ['user'] as const, null, async (c, who) => {
    const hash = hashToken(c.req.param('code') ?? '');
    const found = await findInvite(hub.db, hash);
    const pod = found === undefined ? undefined : hub.pods.get(found.pod);
    if (pod === undefined) return fail(c, 'not_found', 'No such invite.');
    const reply = await pod.exec({ type: 'member_add', userId: who.actor.id, name: who.actor.displayName }, {
      pre: async (s) => {
        if (s.members.some((m) => m.id === who.actor.id)) return { ok: true, result: null };
        const now = await findInvite(hub.db, hash);
        return now !== undefined && now.usesLeft > 0 && now.expiresAt > hub.now() ? null : { ok: false, error: { code: 'gone', message: 'This invite is used or expired.' } };
      },
      extra: async (t) => { await useInvite(t, hash); },
    });
    return respond(c, reply, () => ({ pod: podView(pod) }));
  }));
}

function agentRoute(app: Hono, hub: Hub): void {
  app.post('/v1/pods/:pod/agents', podRoute(hub, ['computer'] as const, 'RegisterAgentRequest', [], async (k) => {
    if (k.who.kind !== 'computer') return fail(k.c, 'forbidden', 'Only a computer registers an agent.');
    const { computer, owner } = k.who;
    const harness = k.body.harness as string;
    const displayName = k.body.displayName as string;
    const agentId = newId('agt_');
    const reply = await k.pod.exec(
      { type: 'agent_register', agentId, ownerId: owner.id, computerId: computer.id, agentName: displayName, ownerName: owner.displayName },
      {
        pre: async (s) => (s.members.some((m) => m.id === owner.id) ? null : { ok: false, error: { code: 'forbidden', message: 'The computer\'s user is not in this pod.' } }),
        meta: (m) => { m.harness[agentId] = harness; },
      },
    );
    if (reply?.ok) hub.indexPod(k.pod);
    return respond(k.c, reply, () => ({ agentId }));
  }));
}

/** The pod where `computerId` hosts the agent a pending wake is for. Wake ids are per pod, so this is where they are resolved. */
function wakePod(hub: Hub, computerId: string, wakeId: string): PodRuntime | undefined {
  for (const id of hub.computerPods.get(computerId) ?? []) {
    const pod = hub.pods.get(id);
    if (pod && Object.values(pod.state.wakes).some((w) => w.id === wakeId && pod.state.agents[w.agentId]?.computerId === computerId)) return pod;
  }
  return undefined;
}

function brief(pod: PodRuntime, runId: string, r: RunClaimed): string {
  const who = pod.state.runs[runId].agent.displayName;
  const refs = Object.entries(r.refs).map(([k, v]) => `${k} ${v}`).join(', ');
  return `You are ${who}, woken in the pod "${pod.meta.name}" (${r.reason}${refs ? `: ${refs}` : ''}). Call jm_whoami, then jm_task_list, claim a task before working on it and report with jm_post.`;
}

function runRoutes(app: Hono, hub: Hub, publicUrl: string | undefined): void {
  app.post('/v1/runs', setup(hub, ['computer'] as const, 'ClaimRunRequest', async (c, who, body) => {
    const wakeId = body.wakeId as string;
    const pod = wakePod(hub, who.computer.id, wakeId);
    if (pod === undefined) return fail(c, 'not_found', 'No such wake, or it was already claimed.', wakeId);
    const runId = newId('run_');
    const runToken = newToken('jmr_');
    const reply = await pod.exec({ type: 'run_claim', wakeId, computerId: who.computer.id, runId }, {
      extra: async (t) => { await saveToken(t, hashToken(runToken), 'run', runId, pod.id); },
    });
    if (reply?.ok) hub.runPod.set(runId, pod.id);
    return respond(c, reply, (r) => ({ runId, runToken, brief: brief(pod, runId, r as RunClaimed), mcpUrl: `${baseOf(publicUrl, c)}/mcp`, expiresAt: (r as RunClaimed).expiresAt }));
  }));
  app.post('/v1/runs/:run/end', setup(hub, ['computer'] as const, 'EndRunRequest', async (c, who, body) => {
    // The core's run_end has no place for the harness version, so it is refused rather than dropped.
    if (body.harnessVersion !== undefined) return bad(c, 'body.harnessVersion: this server does not store a harness version yet');
    const runId = c.req.param('run') ?? '';
    const pod = hub.pods.get(hub.runPod.get(runId) ?? '');
    if (pod === undefined) return fail(c, 'not_found', 'No such run.', runId);
    return respond(c, await pod.exec({ type: 'run_end', runId, computerId: who.computer.id, outcome: body.outcome as 'completed' | 'failed' | 'cancelled' }));
  }));
}

function setupRoutes(app: Hono, hub: Hub): void {
  app.post('/v1/pods', setup(hub, ['user'] as const, 'CreatePodRequest', async (c, who, body) => {
    const { pod, reply } = await hub.createPod(who.actor, body.name as string);
    return respond(c, reply, () => podView(pod));
  }));
  app.post('/v1/computers', setup(hub, ['user'] as const, 'RegisterComputerRequest', async (c, who, body) =>
    c.json(await hub.createComputer(who.actor.id, body.name as string, body.os as string, body.arch as string))));
}

export function setupAll(app: Hono, hub: Hub, publicUrl: string | undefined): void {
  setupRoutes(app, hub);
  inviteRoutes(app, hub, publicUrl);
  agentRoute(app, hub);
  runRoutes(app, hub, publicUrl);
}
