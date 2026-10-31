// The pod operations of schema/operations.json over HTTP, as the pod object serves them. Rules live in the core;
// this file maps requests to commands. The Worker has already authenticated and checked the token kind and pod.
// Not served: evidence (the Worker answers 404). A request field the core has no place for is refused with invalid_input, never
// dropped: a task update's note.
import { TaskStatusValues } from '../../gen/ts/protocol';
import type { Harness, Message, PostInput, ReadOutput, TaskUpdateStatus, WhoamiOutput } from '../../gen/ts/protocol';
import type { Caller } from '../../podcore/src/index';
import { newId, newWakeSeed } from './ids';
import { taskView, tasksOf } from './views';
import { check } from './validate';
import { bad, fail, parse, pick, respond } from './wire';
import type { Body, PodPort, Who } from './wire';

interface Call { pod: PodPort; who: Who; caller: Caller; req: Request; url: URL; params: string[]; body: Body }
type Handler = (k: Call) => Response;

const intQuery = (v: string | null, dflt: number, min: number, max: number): number | null => {
  const n = v === null ? dflt : Number(v);
  return Number.isInteger(n) && n >= min && n <= max ? n : null;
};
const isMember = (pod: PodPort, userId: string): boolean => pod.state.members.some((m) => m.id === userId);

/** A run's read is an operation and renews its claims; a user's read needs only membership. Null means go on. */
function gate(k: Call): Response | null {
  if (k.caller.kind === 'run') {
    const r = k.pod.exec({ type: 'op', runId: k.caller.runId });
    return r?.ok ? null : respond(r);
  }
  return isMember(k.pod, k.caller.userId) ? null : fail('forbidden', 'Only members of this pod may do this.');
}

const whoami: Handler = (k) => {
  const blocked = gate(k);
  if (blocked) return blocked;
  const out: WhoamiOutput = { podId: k.pod.id };
  if (k.caller.kind === 'run') Object.assign(out, { agent: k.pod.state.runs[k.caller.runId].agent, runId: k.caller.runId });
  return Response.json(out);
};

const taskList: Handler = (k) => {
  const status = k.url.searchParams.get('status') ?? undefined;
  if (status !== undefined && pick(status, TaskStatusValues) === undefined) return bad('Unknown status.');
  return gate(k) ?? Response.json({ tasks: tasksOf(k.pod.state, status) });
};

const feed: Handler = (k) => {
  const q = k.url.searchParams;
  const since = intQuery(q.get('sinceSeq'), 0, 0, Number.MAX_SAFE_INTEGER);
  const limit = intQuery(q.get('limit'), 20, 1, 50);
  if (since === null || limit === null) return bad('sinceSeq and limit must be integers within the schema limits.');
  const blocked = gate(k);
  if (blocked) return blocked;
  const withBody = q.get('withBody') === 'true';
  const events = k.pod.eventsAfter(since, 500);
  const messages = events.flatMap((e) => (e.type === 'message.created' ? [e.data as Message] : [])).slice(0, limit)
    .map(({ body, ...rest }) => (withBody && body !== undefined ? { ...rest, body } : rest));
  const ids = new Set(events.flatMap((e) => (e.type === 'task.created' || e.type === 'task.updated' ? [e.data.id] : [])));
  const out: ReadOutput = { messages, tasks: [...ids].map((id) => taskView(k.pod.state, id)), headSeq: k.pod.state.seq };
  return Response.json(out);
};

const claim: Handler = (k) => (k.caller.kind !== 'run' ? fail('forbidden', 'Only a run claims a task.')
  : respond(k.pod.exec({ type: 'claim', runId: k.caller.runId, taskId: k.params[1] })));

/** Schema fields the core command has no place for. A request carrying one is refused, never silently dropped. */
const refuse = (k: Call, fields: readonly string[]): Response | null => {
  const hit = fields.find((f) => k.body[f] !== undefined);
  return hit === undefined ? null : bad(`body.${hit}: this server does not store ${hit} yet`);
};

const update: Handler = (k) => {
  // The core's task_update takes a status and an assignee; the schema's note has nowhere to go.
  const refused = refuse(k, ['note']);
  if (refused) return refused;
  return respond(k.pod.exec({ type: 'task_update', caller: k.caller, taskId: k.params[1], status: k.body.status as TaskUpdateStatus, wakeId: newWakeSeed() }));
};

const release: Handler = (k) => respond(k.pod.exec({ type: 'task_release', caller: k.caller, taskId: k.params[1], reason: k.body.reason as string | undefined }));

const stop: Handler = (k) => respond(k.pod.exec({ type: 'run_stop', caller: k.caller, runId: k.params[1], reason: k.body.reason as string | undefined }));

/**
 * The poster's input goes to the core whole; the adapter adds only what the core cannot know: fresh ids, and the harness this
 * agent was registered with (the daemon said where it runs; the post never does).
 */
const post: Handler = (k) => {
  const messageId = newId('msg_');
  const agentId = k.caller.kind === 'run' ? k.pod.state.runs[k.caller.runId]?.agent.id : undefined;
  const harness = agentId === undefined ? undefined : (k.pod.meta.harness[agentId] as Harness | undefined);
  const reply = k.pod.exec({ type: 'post', caller: k.caller, messageId, decisionId: newId('dec_'), wakeId: newWakeSeed(), harness, input: k.body as unknown as PostInput });
  return respond(reply, (m) => ({ messageId, seq: (m as Message).seq, ...((m as Message).decision === undefined ? {} : { decisionId: (m as Message).decision?.id }) }));
};

const agent: Handler = (k) => {
  if (k.who.kind !== 'computer') return fail('forbidden', 'Only a computer registers an agent.');
  const w = k.who;
  const harness = k.body.harness as string;
  const displayName = k.body.displayName as string;
  const agentId = newId('agt_');
  const reply = k.pod.exec(
    { type: 'agent_register', agentId, ownerId: w.owner.id, computerId: w.computerId, agentName: displayName, ownerName: w.owner.displayName },
    {
      pre: (s) => (s.members.some((m) => m.id === w.owner.id) ? null : { ok: false, error: { code: 'forbidden', message: 'The computer\'s user is not in this pod.' } }),
      meta: (m) => { m.harness[agentId] = harness; },
    },
  );
  return respond(reply, () => ({ agentId }));
};

/**
 * Every route names the request definition in gen/ its body is checked against (and the fields the URL supplies instead),
 * or null for a read, whose input is the query string. podRoute checks before the handler runs, so a handler sees only
 * a valid body and a new write route cannot skip the check without writing `null` here.
 */
interface Route { method: string; re: RegExp; fn: Handler; def: string | null; fromPath?: string[]; optional?: string[] }
const ROUTES: Route[] = [
  { method: 'GET', re: /^\/v1\/pods\/([^/]+)\/me$/, fn: whoami, def: null },
  { method: 'GET', re: /^\/v1\/pods\/([^/]+)\/feed$/, fn: feed, def: null },
  { method: 'GET', re: /^\/v1\/pods\/([^/]+)\/tasks$/, fn: taskList, def: null },
  { method: 'POST', re: /^\/v1\/pods\/([^/]+)\/tasks\/([^/]+)\/claim$/, fn: claim, def: 'TaskClaimInput', fromPath: ['taskId'] },
  { method: 'PATCH', re: /^\/v1\/pods\/([^/]+)\/tasks\/([^/]+)$/, fn: update, def: 'TaskUpdateInput', fromPath: ['taskId'] },
  { method: 'POST', re: /^\/v1\/pods\/([^/]+)\/tasks\/([^/]+)\/release$/, fn: release, def: 'TaskReleaseInput', fromPath: ['taskId'] },
  { method: 'POST', re: /^\/v1\/pods\/([^/]+)\/runs\/([^/]+)\/stop$/, fn: stop, def: 'RunStopInput', fromPath: ['runId'] },
  { method: 'POST', re: /^\/v1\/pods\/([^/]+)\/messages$/, fn: post, def: 'PostInput', optional: ['summary'] },
  { method: 'POST', re: /^\/v1\/pods\/([^/]+)\/agents$/, fn: agent, def: 'RegisterAgentRequest' },
];

/** Null when no route matches (the socket upgrade and the internal routes are handled elsewhere). */
export async function podRoute(pod: PodPort, req: Request, who: Who): Promise<Response | null> {
  const url = new URL(req.url);
  for (const { method, re, fn, def, fromPath, optional } of ROUTES) {
    const m = method === req.method ? re.exec(url.pathname) : null;
    if (m === null) continue;
    const body = req.method === 'GET' ? {} : await parse(req);
    if (body === null) return bad('The body must be a JSON object.');
    const problem = def === null ? null : check(def, body, fromPath, optional);
    if (problem !== null) return bad(problem);
    const caller: Caller = who.kind === 'run' ? { kind: 'run', runId: who.runId } : { kind: 'user', userId: who.kind === 'user' ? who.actor.id : '' };
    return fn({ pod, who, caller, req, url, params: m.slice(1), body });
  }
  return null;
}
