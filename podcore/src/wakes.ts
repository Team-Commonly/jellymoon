// Wakes and run start. A wake is written only for an addressed change; one agent has at most one pending wake.
import type { AgentId, ComputerId, WakeId, WakeReason, WakeRefs } from '../../gen/ts/protocol';
import { RUN_CAP_MS } from './claims';
import { fail, ok, push } from './emit';
import { isoOf } from './time';
import type { Cmd, Ctx, Wake } from './types';

type Of<T extends Cmd['type']> = Extract<Cmd, { type: T }>;

/** An un-acked wake is re-sent on a heartbeat once it is this old (>=, not >). */
export const WAKE_RESEND_MS = 60_000;

const REF_KEYS = ['taskId', 'messageId', 'decisionId'] as const;

/** The schema holds one id per kind of ref, so on a clash the earlier ref stays. */
function mergeRefs(into: WakeRefs, from: WakeRefs): void {
  for (const k of REF_KEYS) if (into[k] === undefined && from[k] !== undefined) into[k] = from[k];
}

function send(c: Ctx, computerId: ComputerId, w: Wake): void {
  w.sentAt = c.now;
  push(c, computerId, 'run.wake', { wakeId: w.id, agentId: w.agentId, reason: w.reason, refs: w.refs });
}

/**
 * The one wake writer. An unregistered agent has no computer to wake, so its change is dropped, not held (ADR 0003).
 * A pending wake absorbs the change; otherwise a new one is written under `wakeId`, or a counter-derived id.
 */
export function writeWake(c: Ctx, agentId: AgentId, reason: WakeReason, refs: WakeRefs, wakeId?: WakeId): void {
  const agent = c.s.agents[agentId];
  if (agent === undefined) return;
  const pending = c.s.wakes[agentId];
  if (pending !== undefined) return mergeRefs(pending.refs, refs);
  const own: WakeRefs = {};
  mergeRefs(own, refs);
  c.s.wakeN += 1;
  const id = wakeId ?? `wak_${String(c.s.wakeN).padStart(26, '0')}`;
  const w: Wake = { id, agentId, reason, refs: own, acked: false, sentAt: c.now };
  c.s.wakes[agentId] = w;
  send(c, agent.computerId, w);
}

/** A ruling on an agent's own ask. Assignments and mentions are derived by the task and message commands. */
export function changeCmd(c: Ctx, cmd: Of<'change'>): void {
  if (cmd.kind === 'ruling') writeWake(c, cmd.agentId, 'decision_ruled', cmd.refs, cmd.wakeId);
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
