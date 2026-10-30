// Wakes and run start. A wake is written only for an addressed change; one agent has at most one pending wake.
import type { ComputerId, WakeReason, WakeRefs } from '../../gen/ts/protocol';
import { RUN_CAP_MS } from './claims';
import { fail, ok, push } from './emit';
import { isoOf } from './time';
import type { Cmd, Ctx, Wake } from './types';

type Of<T extends Cmd['type']> = Extract<Cmd, { type: T }>;

/** An un-acked wake is re-sent on a heartbeat once it is this old (>=, not >). */
export const WAKE_RESEND_MS = 60_000;

const REASON: Record<Of<'change'>['kind'], WakeReason> = { task_assigned: 'task_assigned', mention: 'mention', ruling: 'decision_ruled' };
const REF_KEYS = ['taskId', 'messageId', 'decisionId'] as const;

/** The schema holds one id per kind of ref, so on a clash the earlier ref stays. */
function mergeRefs(into: WakeRefs, from: WakeRefs): void {
  for (const k of REF_KEYS) if (into[k] === undefined && from[k] !== undefined) into[k] = from[k];
}

function send(c: Ctx, computerId: ComputerId, w: Wake): void {
  w.sentAt = c.now;
  push(c, computerId, 'run.wake', { wakeId: w.id, agentId: w.agentId, reason: w.reason, refs: w.refs });
}

/** An unregistered agent has no computer to wake, so its change is dropped, not held (ADR 0003). */
export function changeCmd(c: Ctx, cmd: Of<'change'>): void {
  const agent = c.s.agents[cmd.agentId];
  if (!Object.hasOwn(REASON, cmd.kind) || agent === undefined) return ok(c);
  const pending = c.s.wakes[cmd.agentId];
  if (pending !== undefined) mergeRefs(pending.refs, cmd.refs);
  else {
    const refs: WakeRefs = {};
    mergeRefs(refs, cmd.refs);
    const w: Wake = { id: cmd.wakeId, agentId: cmd.agentId, reason: REASON[cmd.kind], refs, acked: false, sentAt: c.now };
    c.s.wakes[cmd.agentId] = w;
    send(c, agent.computerId, w);
  }
  ok(c);
}

const wakeById = (c: Ctx, wakeId: string): Wake | undefined => Object.values(c.s.wakes).find((w) => w.id === wakeId);

export function wakeAckCmd(c: Ctx, cmd: Of<'wake_ack'>): void {
  const w = wakeById(c, cmd.wakeId);
  if (w !== undefined && c.s.agents[w.agentId].computerId === cmd.computerId) w.acked = true;
  ok(c);
}

/** On a heartbeat, the hosting computer gets every un-acked wake that is old enough. */
export function resendWakes(c: Ctx, computerId: ComputerId): void {
  for (const w of Object.values(c.s.wakes)) {
    if (!w.acked && c.s.agents[w.agentId].computerId === computerId && c.now - w.sentAt >= WAKE_RESEND_MS) send(c, computerId, w);
  }
}

export function runClaimCmd(c: Ctx, cmd: Of<'run_claim'>): void {
  const w = wakeById(c, cmd.wakeId);
  if (w === undefined) return fail(c, 'not_found', 'No such wake, or it was already claimed.', cmd.wakeId);
  const agent = c.s.agents[w.agentId];
  if (agent.computerId !== cmd.computerId) return fail(c, 'forbidden', 'This computer does not host that agent.', w.agentId);
  c.s.runs[cmd.runId] = { id: cmd.runId, agent: agent.actor, owner: agent.owner, computerId: cmd.computerId, startedAt: c.now, status: 'active', wakeReason: w.reason };
  delete c.s.wakes[w.agentId];
  ok(c, { runId: cmd.runId, expiresAt: isoOf(c.now + RUN_CAP_MS), reason: w.reason, refs: w.refs });
}

export function agentRegisterCmd(c: Ctx, cmd: Of<'agent_register'>): void {
  const old = c.s.agents[cmd.agentId];
  c.s.agents[cmd.agentId] = {
    actor: { kind: 'agent', id: cmd.agentId, displayName: cmd.agentName ?? old?.actor.displayName ?? cmd.agentId },
    owner: { kind: 'human', id: cmd.ownerId, displayName: cmd.ownerName ?? old?.owner.displayName ?? cmd.ownerId },
    computerId: cmd.computerId,
  };
  ok(c);
}
