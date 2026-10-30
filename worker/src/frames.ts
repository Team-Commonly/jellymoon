// Client frames on the pod socket: parse one, turn it into a core command (or a refusal), run it.
// The token rode in Authorization on the upgrade; hello was sent at once; the client then subscribes.
import { TaskKindValues } from '../../gen/ts/protocol';
import type { ErrorBody } from '../../gen/ts/protocol';
import type { Cmd } from '../../podcore/src/index';
import { newId, newWakeSeed } from './ids';
import { pick, str } from './wire';
import type { PodPort, Who } from './wire';

const ACTOR_ID = /^(usr|agt|sys)_[0-9A-HJKMNP-TV-Z]{26}$/;
const MESSAGE_ID = /^msg_[0-9A-HJKMNP-TV-Z]{26}$/;
const TYPES = ['subscribe', 'message.post', 'task.create', 'presence.heartbeat', 'wake.ack'];
type Data = Record<string, unknown>;

/** What the socket does with a parsed frame. `subscribe` is the socket's own business; the rest are core commands. */
export type Parsed = { subscribe: number } | { cmd: Cmd } | ErrorBody;

function userCommand(type: string, d: Data, who: Extract<Who, { kind: 'user' }>): Cmd | ErrorBody {
  const caller = { kind: 'user', userId: who.actor.id } as const;
  if (type === 'message.post') {
    const text = str(d.text, 1, 16384);
    const threadOf = d.threadOf === undefined ? undefined : str(d.threadOf, 30, 30);
    if (text === undefined || (d.threadOf !== undefined && (threadOf === undefined || !MESSAGE_ID.test(threadOf)))) return { code: 'invalid_input', message: 'message.post needs text (1 to 16384 characters) and, optionally, a threadOf message id.' };
    return { type: 'message_post', caller, messageId: newId('msg_'), wakeId: newWakeSeed(), text, ...(threadOf ? { threadOf } : {}) };
  }
  const title = str(d.title, 1, 120);
  const kind = pick(d.kind, TaskKindValues);
  const body = d.body === undefined ? undefined : str(d.body, 0, 16384);
  const assignee = d.assignee === undefined ? undefined : str(d.assignee, 30, 30);
  if (title === undefined || kind === undefined || (d.body !== undefined && body === undefined) || (d.assignee !== undefined && !ACTOR_ID.test(assignee ?? ''))) return { code: 'invalid_input', message: 'task.create needs a title (1 to 120 characters) and a kind; assignee is an actor id and body at most 16384 characters.' };
  return { type: 'task_create', caller, taskId: newId('tsk_'), wakeId: newWakeSeed(), title, kind, ...(assignee ? { assignee } : {}), ...(body === undefined ? {} : { body }) };
}

function computerCommand(type: string, d: Data, w: Extract<Who, { kind: 'computer' }>): Cmd | ErrorBody {
  if (type === 'presence.heartbeat') return d.computerId !== undefined && d.computerId !== w.computerId ? { code: 'forbidden', message: 'This socket belongs to another computer.' } : { type: 'heartbeat', computerId: w.computerId };
  const wakeId = str(d.wakeId, 1, 64);
  return wakeId === undefined ? { code: 'invalid_input', message: 'wake.ack needs a wakeId.' } : { type: 'wake_ack', wakeId, computerId: w.computerId };
}

export function parseFrame(podId: string, who: Who, text: string): Parsed {
  let f: Record<string, unknown> | null = null;
  try { f = JSON.parse(text) as Record<string, unknown>; } catch { /* refused below */ }
  const d = f?.data ?? {};
  if (f === null || typeof f !== 'object' || f.v !== 1 || f.podId !== podId || 'seq' in f || 'ts' in f || typeof d !== 'object' || d === null) return { code: 'invalid_input', message: 'A frame is {v: 1, type, podId, data} for this pod, with no seq or ts.' };
  const type = typeof f.type === 'string' ? f.type : '';
  if (!TYPES.includes(type)) return { code: 'invalid_input', message: `${type || 'This frame'} is not served by this server.` };
  if (type === 'subscribe') {
    const since = (d as Data).sinceSeq;
    return typeof since === 'number' && Number.isInteger(since) && since >= 0 ? { subscribe: since } : { code: 'invalid_input', message: 'subscribe needs sinceSeq, an integer from 0.' };
  }
  const posts = type === 'message.post' || type === 'task.create';
  if (posts && who.kind !== 'user') return { code: 'forbidden', message: 'Only a person posts or creates tasks.' };
  if (!posts && who.kind !== 'computer') return { code: 'forbidden', message: 'Only a computer sends this.' };
  const cmd = who.kind === 'user' ? userCommand(type, d as Data, who) : computerCommand(type, d as Data, who as Extract<Who, { kind: 'computer' }>);
  return 'code' in cmd ? cmd : { cmd };
}

/** Runs a parsed command and returns the refusal to send back, if any. */
export function runFrame(pod: PodPort, cmd: Cmd): ErrorBody | null {
  const reply = pod.exec(cmd);
  return reply && !reply.ok ? reply.error : null;
}
