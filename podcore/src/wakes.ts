// Wakes and run start. A wake is written only for an addressed change; one agent has at most one pending wake.
import type { AgentId, ComputerId, WakeId, WakeReason, WakeRefs } from '../../gen/ts/protocol';
import { RUN_CAP_MS, own } from './claims';
import { nthWakeId } from './wakeid';
import { fail, ok, push } from './emit';
import { isoOf } from './time';
import type { Cmd, Ctx, Wake } from './types';

type Of<T extends Cmd['type']> = Extract<Cmd, { type: T }>;

/** A wake with no run_claim is re-sent on a heartbeat once its last send is this old (>=, not >). */
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
 * A pending wake absorbs the change and `seed` is ignored; otherwise a new wake is written under
 * `nthWakeId(seed, k)`, k counting the new wakes this command has opened. The caller checked `seedFits` first.
 * A run never wakes its own agent: `by` is the caller's actor id.
 */
export function writeWake(c: Ctx, agentId: AgentId, reason: WakeReason, refs: WakeRefs, seed: WakeId, by?: string): void {
  const agent = own(c.s.agents, agentId);
  if (agent === undefined || agentId === by) return;
  const pending = own(c.s.wakes, agentId);
  if (pending !== undefined) return mergeRefs(pending.refs, refs);
  const mine: WakeRefs = {};
  mergeRefs(mine, refs);
  const w: Wake = { id: nthWakeId(seed, c.opened) as WakeId, agentId, reason, refs: mine, sentAt: c.now };
  c.opened += 1;
  c.s.wakes[agentId] = w;
  send(c, agent.computerId, w);
}

/** Refuses (invalid_input) a malformed seed, or one whose random part cannot hold `n` wakes. Call before any effect. */
export function seedFits(c: Ctx, seed: WakeId, n: number): boolean {
  const taken = Array.from({ length: Math.max(n, 1) }, (_, k) => nthWakeId(seed, k));
  if (Object.values(c.s.wakes).some((w) => taken.includes(w.id))) {
    fail(c, 'invalid_input', 'That wakeId is already a pending wake; a retry needs a fresh seed.', seed);
    return false;
  }
  if (nthWakeId(seed, Math.max(n - 1, 0)) !== null) return true;
  fail(c, 'invalid_input', 'wakeId must be a fresh wak_ ULID with room for the wakes this command opens.', seed);
  return false;
}

/** A ruling on an agent's own ask. Assignments and mentions are derived by the task and message commands. */
export function changeCmd(c: Ctx, cmd: Of<'change'>): void {
  if (!seedFits(c, cmd.wakeId, 1)) return;
  if (cmd.kind === 'ruling') writeWake(c, cmd.agentId, 'decision_ruled', cmd.refs, cmd.wakeId);
  ok(c);
}

const wakeById = (c: Ctx, wakeId: string): Wake | undefined => Object.values(c.s.wakes).find((w) => w.id === wakeId);

/** An ack changes nothing: a pending wake is re-sent until `run_claim` consumes it, acked or not (ADR 0003). */
export const wakeAckCmd = (c: Ctx): void => ok(c);

/** On a heartbeat, the hosting computer gets every pending wake that is old enough. */
export function resendWakes(c: Ctx, computerId: ComputerId): void {
  for (const w of Object.values(c.s.wakes)) {
    if (c.s.agents[w.agentId].computerId === computerId && c.now - w.sentAt >= WAKE_RESEND_MS) send(c, computerId, w);
  }
}

export function runClaimCmd(c: Ctx, cmd: Of<'run_claim'>): void {
  const w = wakeById(c, cmd.wakeId);
  if (w === undefined) return fail(c, 'not_found', 'No such wake, or it was already claimed.', cmd.wakeId);
  const agent = c.s.agents[w.agentId];
  if (agent.computerId !== cmd.computerId) return fail(c, 'forbidden', 'This computer does not host that agent.', w.agentId);
  if (own(c.s.runs, cmd.runId) !== undefined) return fail(c, 'invalid_input', 'That run id is taken.', cmd.runId);
  c.s.runs[cmd.runId] = { id: cmd.runId, agent: agent.actor, owner: agent.owner, computerId: cmd.computerId, startedAt: c.now, status: 'active' };
  delete c.s.wakes[w.agentId];
  ok(c, { runId: cmd.runId, expiresAt: isoOf(c.now + RUN_CAP_MS), reason: w.reason, refs: w.refs });
}

export function agentRegisterCmd(c: Ctx, cmd: Of<'agent_register'>): void {
  const old = own(c.s.agents, cmd.agentId);
  c.s.agents[cmd.agentId] = {
    actor: { kind: 'agent', id: cmd.agentId, displayName: cmd.agentName ?? old?.actor.displayName ?? cmd.agentId },
    owner: { kind: 'human', id: cmd.ownerId, displayName: cmd.ownerName ?? old?.owner.displayName ?? cmd.ownerId },
    computerId: cmd.computerId,
  };
  const pending = own(c.s.wakes, cmd.agentId);
  if (pending !== undefined && old?.computerId !== cmd.computerId) send(c, cmd.computerId, pending);
  ok(c);
}
