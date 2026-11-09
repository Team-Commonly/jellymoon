// The pod operations of schema/operations.json over HTTP. Rules live in the core; this file maps requests to commands.
// Not served here: evidence (404).
import type { Hono } from 'hono';
import { TaskStatusValues } from '../../gen/ts/protocol';
import type { Harness, Message, PostInput, ReadOutput, TaskUpdateStatus, WhoamiOutput } from '../../gen/ts/protocol';
import { overlayDecisions } from '../../podcore/src/index';
import type { Hub } from './hub';
import { newId, newWakeSeed } from './ids';
import { bad, fail, memberRoute, pick, respond } from './kit';
import type { Call } from './kit';
import { viaMcp } from './mcp';
import { eventsAfter } from './store';
import { taskView, tasksOf } from './views';

/** A run's read is an operation and renews its claims; a user's read needs only membership. Null means go on. */
export async function gate(k: Call): Promise<Response | null> {
  if (k.caller.kind === 'run') {
    const r = await k.pod.exec({ type: 'op', runId: k.caller.runId });
    return r?.ok ? null : respond(k.c, r);
  }
  return k.pod.isMember(k.caller.userId) ? null : fail(k.c, 'forbidden', 'Only members of this pod may do this.');
}

const whoami = async (k: Call): Promise<Response> => {
  const blocked = await gate(k);
  if (blocked) return blocked;
  const out: WhoamiOutput = { podId: k.pod.id };
  if (k.caller.kind === 'run') Object.assign(out, { agent: k.pod.state.runs[k.caller.runId].agent, runId: k.caller.runId });
  return k.c.json(out);
};

const taskList = async (k: Call): Promise<Response> => {
  const status = k.c.req.query('status');
  if (status !== undefined && pick(status, TaskStatusValues) === undefined) return bad(k.c, 'Unknown status.');
  const blocked = await gate(k);
  return blocked ?? k.c.json({ tasks: tasksOf(k.pod.state, status) });
};

const intQuery = (v: string | undefined, dflt: number, min: number, max: number): number | null => {
  const n = v === undefined ? dflt : Number(v);
  return Number.isInteger(n) && n >= min && n <= max ? n : null;
};

const feed = (hub: Hub) => async (k: Call): Promise<Response> => {
  const since = intQuery(k.c.req.query('sinceSeq'), 0, 0, Number.MAX_SAFE_INTEGER);
  const limit = intQuery(k.c.req.query('limit'), 20, 1, 50);
  if (since === null || limit === null) return bad(k.c, 'sinceSeq and limit must be integers within the schema limits.');
  const blocked = await gate(k);
  if (blocked) return blocked;
  const withBody = k.c.req.query('withBody') === 'true';
  const events = await eventsAfter(hub.db, k.pod.id, since, 500);
  const messages = overlayDecisions(k.pod.state, events.flatMap((e) => (e.type === 'message.created' ? [e.data as Message] : [])).slice(0, limit))
    .map(({ body, ...rest }) => (withBody && body !== undefined ? { ...rest, body } : rest));
  const ids = new Set(events.flatMap((e) => (e.type === 'task.created' || e.type === 'task.updated' ? [e.data.id] : [])));
  const out: ReadOutput = { messages, tasks: [...ids].map((id) => taskView(k.pod.state, id)), headSeq: k.pod.state.seq };
  return k.c.json(out);
};

const claim = async (k: Call): Promise<Response> => {
  if (k.caller.kind !== 'run') return fail(k.c, 'forbidden', 'Only a run claims a task.');
  return respond(k.c, await k.pod.exec({ type: 'claim', runId: k.caller.runId, taskId: k.c.req.param('task') ?? '' }));
};

const update = async (k: Call): Promise<Response> => {
  return respond(k.c, await k.pod.exec({ type: 'task_update', caller: k.caller, taskId: k.c.req.param('task') ?? '', status: k.body.status as TaskUpdateStatus, wakeId: newWakeSeed() }));
};

const release = async (k: Call): Promise<Response> =>
  respond(k.c, await k.pod.exec({ type: 'task_release', caller: k.caller, taskId: k.c.req.param('task') ?? '', reason: k.body.reason as string | undefined }));

const stop = async (k: Call): Promise<Response> =>
  respond(k.c, await k.pod.exec({ type: 'run_stop', caller: k.caller, runId: k.c.req.param('run') ?? '', reason: k.body.reason as string | undefined }));

/** A post is the core's `post` command (podcore/src/post.ts): the receipt, the ask and the decision are the core's. The harness is the adapter's record of where the agent runs; viaMcp says the post came in on /mcp, whose carrying call the receipt leaves out. */
const post = async (k: Call): Promise<Response> => {
  const agentId = k.caller.kind === 'run' ? k.pod.state.runs[k.caller.runId]?.agent.id : undefined;
  const harness = agentId === undefined ? undefined : (k.pod.meta.harness[agentId] as Harness | undefined);
  const messageId = newId('msg_');
  const reply = await k.pod.exec({ type: 'post', caller: k.caller, messageId, decisionId: newId('dec_'), wakeId: newWakeSeed(), harness, viaMcp: viaMcp(k.c.req.raw), input: k.body as unknown as PostInput });
  return respond(k.c, reply, (m) => ({ messageId, seq: (m as Message).seq, ...((m as Message).decision === undefined ? {} : { decisionId: (m as Message).decision?.id }) }));
};

/**
 * Every route names the request definition in gen/ its body is checked against (and the fields the URL supplies instead), or
 * null for a read, whose input is the query string. memberRoute checks before the handler runs, so a handler sees only a valid
 * body and a new write route cannot skip the check without writing `null` here.
 */
export function opRoutes(app: Hono, hub: Hub): void {
  app.get('/v1/pods/:pod/me', memberRoute(hub, null, [], whoami));
  app.get('/v1/pods/:pod/feed', memberRoute(hub, null, [], feed(hub)));
  app.get('/v1/pods/:pod/tasks', memberRoute(hub, null, [], taskList));
  app.post('/v1/pods/:pod/tasks/:task/claim', memberRoute(hub, 'TaskClaimInput', ['taskId'], claim));
  app.patch('/v1/pods/:pod/tasks/:task', memberRoute(hub, 'TaskUpdateInput', ['taskId'], update));
  app.post('/v1/pods/:pod/tasks/:task/release', memberRoute(hub, 'TaskReleaseInput', ['taskId'], release));
  app.post('/v1/pods/:pod/runs/:run/stop', memberRoute(hub, 'RunStopInput', ['runId'], stop));
  app.post('/v1/pods/:pod/messages', memberRoute(hub, 'PostInput', [], post, ['summary']));
  app.get('/v1/pods/:pod/runs/:run/tool-calls', (c) => fail(c, 'not_found', 'Evidence is not served by this server.'));
}
