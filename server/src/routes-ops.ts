// The pod operations of schema/operations.json over HTTP. Rules live in the core; this file maps requests to commands.
// Not served here: evidence (404). A request field the core has no place for is refused with invalid_input, never dropped: a
// post's kind (other than fyi), status (other than done), needs, options, context, taskId, links and caveat, and a task
// update's note.
import type { Hono } from 'hono';
import { TaskStatusValues } from '../../gen/ts/protocol';
import type { Message, ReadOutput, TaskUpdateStatus, WhoamiOutput } from '../../gen/ts/protocol';
import type { Hub } from './hub';
import { newId, newWakeSeed } from './ids';
import { bad, fail, memberRoute, pick, respond } from './kit';
import type { Call } from './kit';
import { eventsAfter } from './store';
import { taskView, tasksOf } from './views';

/** A run's read is an operation and renews its claims; a user's read needs only membership. Null means go on. */
async function gate(k: Call): Promise<Response | null> {
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
  const messages = events.flatMap((e) => (e.type === 'message.created' ? [e.data as Message] : [])).slice(0, limit)
    .map(({ body, ...rest }) => (withBody && body !== undefined ? { ...rest, body } : rest));
  const ids = new Set(events.flatMap((e) => (e.type === 'task.created' || e.type === 'task.updated' ? [e.data.id] : [])));
  const out: ReadOutput = { messages, tasks: [...ids].map((id) => taskView(k.pod.state, id)), headSeq: k.pod.state.seq };
  return k.c.json(out);
};

const claim = async (k: Call): Promise<Response> => {
  if (k.caller.kind !== 'run') return fail(k.c, 'forbidden', 'Only a run claims a task.');
  return respond(k.c, await k.pod.exec({ type: 'claim', runId: k.caller.runId, taskId: k.c.req.param('task') ?? '' }));
};

/** Schema fields the core command has no place for. A request carrying one is refused, never silently dropped. */
const refuse = (k: Call, fields: readonly string[]): Response | null => {
  const hit = fields.find((f) => k.body[f] !== undefined);
  return hit === undefined ? null : bad(k.c, `body.${hit}: this server does not store ${hit} yet`);
};

const update = async (k: Call): Promise<Response> => {
  // The core's task_update takes a status and an assignee; the schema's note has nowhere to go.
  const refused = refuse(k, ['note']);
  if (refused) return refused;
  return respond(k.c, await k.pod.exec({ type: 'task_update', caller: k.caller, taskId: k.c.req.param('task') ?? '', status: k.body.status as TaskUpdateStatus, wakeId: newWakeSeed() }));
};

const release = async (k: Call): Promise<Response> =>
  respond(k.c, await k.pod.exec({ type: 'task_release', caller: k.caller, taskId: k.c.req.param('task') ?? '', reason: k.body.reason as string | undefined }));

const stop = async (k: Call): Promise<Response> =>
  respond(k.c, await k.pod.exec({ type: 'run_stop', caller: k.caller, runId: k.c.req.param('run') ?? '', reason: k.body.reason as string | undefined }));

/** The core stores every post as kind fyi, status done, with none of the ask fields (podcore/src/board.ts messagePostCmd). */
const post = async (k: Call): Promise<Response> => {
  const refused = refuse(k, ['needs', 'options', 'context', 'taskId', 'links', 'caveat']);
  if (refused) return refused;
  if (k.body.kind !== 'fyi') return bad(k.c, 'body.kind: this server stores only fyi posts yet');
  if (k.body.status !== 'done') return bad(k.c, 'body.status: this server stores only done posts yet');
  const summary = k.body.summary as string;
  const body = k.body.body as string | undefined;
  const messageId = newId('msg_');
  const reply = await k.pod.exec({ type: 'message_post', caller: k.caller, messageId, wakeId: newWakeSeed(), summary, text: body === undefined || body === '' ? summary : `${summary}\n${body}` });
  return respond(k.c, reply, (m) => ({ messageId, seq: (m as Message).seq }));
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
  app.post('/v1/pods/:pod/messages', memberRoute(hub, 'PostInput', [], post));
  app.get('/v1/pods/:pod/runs/:run/tool-calls', (c) => fail(c, 'not_found', 'Evidence is not served by this server.'));
}
