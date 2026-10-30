// The pod operations of schema/operations.json over HTTP, as the pod object serves them. Rules live in the core;
// this file maps requests to commands. The Worker has already authenticated and checked the token kind and pod.
// Not served: evidence (the Worker answers 404) and the `ask` message kind (a post is stored as a plain message).
import { HarnessValues, TaskStatusValues } from '../../gen/ts/protocol';
import type { Message, ReadOutput, WhoamiOutput } from '../../gen/ts/protocol';
import type { Caller } from '../../podcore/src/index';
import { newId, newWakeSeed } from './ids';
import { taskView, tasksOf } from './views';
import { bad, fail, parse, pick, respond, str } from './wire';
import type { Body, PodPort, Who } from './wire';

interface Call { pod: PodPort; who: Who; caller: Caller; req: Request; url: URL; params: string[]; body: Body }
type Handler = (k: Call) => Response;

const UPDATE_STATUS = ['in_progress', 'blocked', 'done', 'failed'] as const;
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
  const out: ReadOutput = { messages, tasks: [...ids].sort().map((id) => taskView(k.pod.state, id)), headSeq: k.pod.state.seq };
  return Response.json(out);
};

const claim: Handler = (k) => (k.caller.kind !== 'run' ? fail('forbidden', 'Only a run claims a task.')
  : respond(k.pod.exec({ type: 'claim', runId: k.caller.runId, taskId: k.params[1] })));

const update: Handler = (k) => {
  const status = pick(k.body.status, UPDATE_STATUS);
  if (status === undefined || (k.body.note !== undefined && str(k.body.note, 0, 500) === undefined)) return bad('status must be in_progress, blocked, done or failed; note is at most 500 characters.');
  return respond(k.pod.exec({ type: 'task_update', caller: k.caller, taskId: k.params[1], status, wakeId: newWakeSeed() }));
};

const reasonOf = (k: Call): string | undefined | null => (k.body.reason === undefined ? undefined : (str(k.body.reason, 0, 160) ?? null));

const release: Handler = (k) => {
  const reason = reasonOf(k);
  return reason === null ? bad('reason is at most 160 characters.') : respond(k.pod.exec({ type: 'task_release', caller: k.caller, taskId: k.params[1], reason }));
};

const stop: Handler = (k) => {
  const reason = reasonOf(k);
  return reason === null ? bad('reason is at most 160 characters.') : respond(k.pod.exec({ type: 'run_stop', caller: k.caller, runId: k.params[1], reason }));
};

const post: Handler = (k) => {
  const summary = str(k.body.summary, 1, 120);
  if (summary === undefined) return fail('summary_required', 'A post needs a summary of 1 to 120 characters.');
  const body = k.body.body === undefined ? undefined : str(k.body.body, 0, 32768);
  const messageId = newId('msg_');
  const reply = k.pod.exec({ type: 'message_post', caller: k.caller, messageId, wakeId: newWakeSeed(), text: body === undefined || body === '' ? summary : `${summary}\n${body}` });
  return respond(reply, (m) => ({ messageId, seq: (m as Message).seq }));
};

const agent: Handler = (k) => {
  if (k.who.kind !== 'computer') return fail('forbidden', 'Only a computer registers an agent.');
  const w = k.who;
  const harness = pick(k.body.harness, HarnessValues);
  const displayName = str(k.body.displayName, 1, 80);
  if (harness === undefined || displayName === undefined) return bad('harness and displayName (1 to 80 characters) are required.');
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

const ROUTES: Array<[string, RegExp, Handler]> = [
  ['GET', /^\/v1\/pods\/([^/]+)\/me$/, whoami],
  ['GET', /^\/v1\/pods\/([^/]+)\/feed$/, feed],
  ['GET', /^\/v1\/pods\/([^/]+)\/tasks$/, taskList],
  ['POST', /^\/v1\/pods\/([^/]+)\/tasks\/([^/]+)\/claim$/, claim],
  ['PATCH', /^\/v1\/pods\/([^/]+)\/tasks\/([^/]+)$/, update],
  ['POST', /^\/v1\/pods\/([^/]+)\/tasks\/([^/]+)\/release$/, release],
  ['POST', /^\/v1\/pods\/([^/]+)\/runs\/([^/]+)\/stop$/, stop],
  ['POST', /^\/v1\/pods\/([^/]+)\/messages$/, post],
  ['POST', /^\/v1\/pods\/([^/]+)\/agents$/, agent],
];

/** Null when no route matches (the socket upgrade and the internal routes are handled elsewhere). */
export async function podRoute(pod: PodPort, req: Request, who: Who): Promise<Response | null> {
  const url = new URL(req.url);
  for (const [method, re, fn] of ROUTES) {
    const m = method === req.method ? re.exec(url.pathname) : null;
    if (m === null) continue;
    const body = req.method === 'GET' ? {} : await parse(req);
    if (body === null) return bad('The body must be a JSON object.');
    const caller: Caller = who.kind === 'run' ? { kind: 'run', runId: who.runId } : { kind: 'user', userId: who.kind === 'user' ? who.actor.id : '' };
    return fn({ pod, who, caller, req, url, params: m.slice(1), body });
  }
  return null;
}
