// A post: the headline contract, the shape of an ask, and the fields only the pod sets (the receipt and the decision).
// The poster's input is copied field by field, never spread, so nothing the poster sends can reach a server-set field.
// Lengths and patterns are the schema's, checked by the adapter before the command; the rules here are the ones the schema
// cannot say: a headline that is only blank space, an ask's required parts, and the facts of the run.
import { HarnessValues } from '../../gen/ts/protocol';
import type { Harness, Message, PostInput, Receipt } from '../../gen/ts/protocol';
import { who, own, isObj } from './claims';
import { emit, fail, ok } from './emit';
import { mentioned } from './board';
import { isoOf } from './time';
import { seedFits, writeWake } from './wakes';
import type { Cmd, Ctx, Run } from './types';

type Of<T extends Cmd['type']> = Extract<Cmd, { type: T }>;

/** Every Message field the pod sets. A post that names one is refused, not ignored. */
const SERVER_SET = ['id', 'seq', 'podId', 'author', 'createdAt', 'runId', 'threadOf', 'decision', 'receipt'];
const ASK_ONLY = ['needs', 'options', 'context'] as const;
/** The fields a poster may write, copied in this order. */
const COPIED = ['summary', 'kind', 'status', 'needs', 'options', 'context', 'taskId', 'links', 'caveat', 'body'] as const;

/** Why the input is not a well-formed post, or null. Only what the schema cannot say. */
function shapeProblem(input: PostInput): string | null {
  const stray = SERVER_SET.find((k) => Object.hasOwn(input, k));
  if (stray !== undefined) return `${stray} is set by the pod; do not send it.`;
  const ask = input.kind === 'ask';
  for (const k of ASK_ONLY) {
    if (ask && input[k] === undefined) return `An ask needs ${k}.`;
    if (!ask && input[k] !== undefined) return `${k} is only for an ask.`;
  }
  const ids = (input.options ?? []).map((o) => o.id);
  if (new Set(ids).size !== ids.length) return 'Option ids must be unique.';
  if (input.context !== undefined && !ids.includes(input.context.recommendation.option)) return 'The recommendation must name one of the options.';
  return null;
}

/** The receipt of a run's post: what the pod recorded of the run, plus the harness its adapter knows the agent runs on. */
function receiptOf(c: Ctx, run: Run, harness: Harness | undefined, taskId: string | undefined): Receipt | null {
  if (harness === undefined || !(HarnessValues as readonly string[]).includes(harness) || run.wakeReason === undefined) return null;
  const r: Receipt = {
    runId: run.id, agentId: run.agent.id, computerId: run.computerId, harness, startedAt: isoOf(run.startedAt), postedAt: isoOf(c.now),
    toolCalls: run.toolCalls ?? 0, wakeReason: run.wakeReason,
  };
  if (taskId !== undefined) r.taskId = taskId;
  return r;
}

export function postCmd(c: Ctx, cmd: Of<'post'>): void {
  const w = who(c, cmd);
  if (w === null) return;
  const input = cmd.input;
  if (!isObj(input)) return fail(c, 'invalid_input', 'The post must be an object.');
  if (typeof input.summary !== 'string' || input.summary.trim() === '') return fail(c, 'summary_required', 'A post needs a summary: the headline people read.');
  const problem = shapeProblem(input);
  if (problem !== null) return fail(c, 'invalid_input', problem);
  const receipt = w.run === undefined ? undefined : receiptOf(c, w.run, cmd.harness, input.taskId);
  if (receipt === null) return fail(c, 'internal', 'The run record is missing a fact the receipt needs.');
  const ids = mentioned(c, `${input.summary}\n${input.body ?? ''}`).filter((id) => id !== w.actor.id);
  if (!seedFits(c, cmd.wakeId, ids.length)) return;
  const msg = { id: cmd.messageId, seq: c.s.seq + 1, podId: c.s.podId, author: w.actor, createdAt: isoOf(c.now) } as Message;
  Object.assign(msg, Object.fromEntries(COPIED.filter((k) => input[k] !== undefined).map((k) => [k, input[k]])));
  if (w.run !== undefined) msg.runId = w.run.id;
  if (receipt !== undefined) msg.receipt = receipt;
  if (input.kind === 'ask') msg.decision = { id: cmd.decisionId, state: 'open' };
  emit(c, 'message.created', msg);
  ids.forEach((id) => writeWake(c, id, 'mention', { messageId: cmd.messageId }, cmd.wakeId));
  ok(c, msg);
}

/** The pod saw one more MCP call on this run's token. An ended or unknown run counts nothing: its token is dead. */
export function toolCallCmd(c: Ctx, cmd: Of<'tool_call'>): void {
  const run = own(c.s.runs, cmd.runId);
  if (run?.status === 'active') run.toolCalls = (run.toolCalls ?? 0) + 1;
  ok(c);
}
