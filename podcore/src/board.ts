// Creating the pod, its members, tasks and messages. A task or message that addresses a registered agent writes its wake.
import type { Message, MessageId, Task } from '../../gen/ts/protocol';
import { taskView, who, acquire } from './claims';
import { emit, fail, ok } from './emit';
import { isoOf } from './time';
import { prose } from './mentions';
import { writeWake } from './wakes';
import type { Cmd, Ctx } from './types';

type Of<T extends Cmd['type']> = Extract<Cmd, { type: T }>;

export function podCreateCmd(c: Ctx, cmd: Of<'pod_create'>): void {
  if (c.s.members.length > 0) return fail(c, 'invalid_input', 'This pod already exists.');
  const owner = { kind: 'human' as const, id: cmd.ownerId, displayName: cmd.ownerName };
  c.s.members = [owner];
  c.s.owners = [owner];
  ok(c);
}

/** The adapter calls this once an invite is accepted. Adding a member twice changes nothing. */
export function memberAddCmd(c: Ctx, cmd: Of<'member_add'>): void {
  if (!c.s.members.some((m) => m.id === cmd.userId)) c.s.members.push({ kind: 'human', id: cmd.userId, displayName: cmd.name });
  ok(c);
}

export function taskCreateCmd(c: Ctx, cmd: Of<'task_create'>): void {
  const w = who(c, cmd.caller);
  if (w === null) return;
  if (c.s.tasks[cmd.taskId] !== undefined) return fail(c, 'invalid_input', 'That task id is taken.', cmd.taskId);
  const at = isoOf(c.now);
  const task: Task = { id: cmd.taskId, podId: c.s.podId, title: cmd.title, kind: cmd.kind, status: 'open', createdBy: w.actor, createdAt: at, updatedAt: at };
  if (cmd.body !== undefined) task.body = cmd.body;
  if (cmd.assignee !== undefined) task.assignee = cmd.assignee;
  c.s.tasks[cmd.taskId] = task;
  emit(c, 'task.created', task);
  if (cmd.assignee !== undefined) writeWake(c, cmd.assignee, 'task_assigned', { taskId: cmd.taskId }, cmd.wakeId, w.actor.id);
  ok(c, task);
}

export function taskUpdateCmd(c: Ctx, cmd: Of<'task_update'>): void {
  const w = who(c, cmd.caller);
  if (w === null) return;
  if ((cmd.status as string | undefined) === 'open') return fail(c, 'invalid_input', 'A task cannot be set back to open.', cmd.taskId);
  const task = c.s.tasks[cmd.taskId];
  if (task === undefined) return fail(c, 'not_found', 'No such task.', cmd.taskId);
  if (w.run !== undefined && acquire(c, w.run, cmd.taskId, false)) return;
  const reassigned = cmd.assignee !== undefined && cmd.assignee !== task.assignee;
  if (cmd.status !== undefined) task.status = cmd.status;
  if (cmd.assignee !== undefined) task.assignee = cmd.assignee;
  task.updatedAt = isoOf(c.now);
  emit(c, 'task.updated', taskView(c, cmd.taskId));
  if (reassigned && cmd.assignee !== undefined) writeWake(c, cmd.assignee, 'task_assigned', { taskId: cmd.taskId }, cmd.wakeId, w.actor.id);
  ok(c, taskView(c, cmd.taskId));
}

/** `@name` tokens outside code spans and fenced blocks, in text order, matched exactly against registered agent names. */
function mentioned(c: Ctx, text: string): string[] {
  const names = Array.from(prose(text).matchAll(/(?<![\w@])@([\w.-]+)/g), (m) => m[1].replace(/[.-]+$/, ''));
  const ids = names.flatMap((n) => Object.values(c.s.agents).filter((a) => a.actor.displayName === n).map((a) => a.actor.id));
  return ids.filter((id, i) => ids.indexOf(id) === i);
}

/** The first line that is not blank (CR, LF or CRLF), trimmed and cut to 120 code points, so it validates as a Message summary. */
const summaryOf = (text: string): string => {
  const line = (text.split(/\r\n|\r|\n/).map((l) => l.trim()).find((l) => l !== '') ?? '(no text)');
  return Array.from(line).slice(0, 120).join('');
};

export function messagePostCmd(c: Ctx, cmd: Of<'message_post'>): void {
  const w = who(c, cmd.caller);
  if (w === null) return;
  const ids = mentioned(c, cmd.text).filter((id) => id !== w.actor.id);
  if (ids.length > cmd.wakeIds.length) throw new Error('message_post: wakeIds needs one id per mentioned agent');
  const msg: Message = {
    id: cmd.messageId, seq: c.s.seq + 1, podId: c.s.podId, author: w.actor, createdAt: isoOf(c.now), summary: summaryOf(cmd.text),
    kind: 'fyi', status: 'done', body: cmd.text,
  };
  if (w.run !== undefined) msg.runId = w.run.id;
  if (cmd.threadOf !== undefined) msg.threadOf = cmd.threadOf;
  emit(c, 'message.created', msg);
  // One adapter-minted wakeId per mentioned agent, in text order; a run's message never wakes its own agent.
  ids.forEach((id, i) => writeWake(c, id, 'mention', { messageId: cmd.messageId as MessageId }, cmd.wakeIds[i]));
  ok(c, msg);
}
