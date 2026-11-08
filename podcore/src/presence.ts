// Presence (F08): which computers are online, and what each agent a computer hosts shows. A computer is online from its socket
// (or its first heartbeat after being offline) until its socket goes or 45 s pass without a sign of life on the pod's clock.
// The adapter reports sockets; the rule, the staleness bound and the events are here. Presence is current state, not history:
// its events are `live` (no seq, never replayed), and a client that connects later reads the current state from hello.
import type { AgentId, ComputerId, PresenceState } from '../../gen/ts/protocol';
import { ok } from './emit';
import { isoOf } from './time';
import type { Cmd, Ctx, PodState } from './types';

type Of<T extends Cmd['type']> = Extract<Cmd, { type: T }>;

/** Three missed 15 s heartbeats. */
export const PRESENCE_STALE_MS = 45_000;

const isOnline = (s: PodState, computerId: ComputerId): boolean => s.online !== undefined && Object.hasOwn(s.online, computerId);

/** What an agent shows: working while a run of it is active, else its computer's presence. */
export function agentPresence(s: PodState, agentId: AgentId): PresenceState {
  if (Object.values(s.runs).some((r) => r.status === 'active' && r.agent.id === agentId)) return 'working';
  return isOnline(s, s.agents[agentId].computerId) ? 'online' : 'offline';
}

function announce(c: Ctx, agentId: AgentId): void {
  const a = c.s.agents[agentId];
  const data = { actor: a.actor, state: agentPresence(c.s, agentId), computerId: a.computerId };
  c.out.push({ kind: 'live', event: { v: 1, type: 'presence.changed', podId: c.s.podId, ts: isoOf(c.now), data } });
}

const hosted = (s: PodState, computerId: ComputerId): AgentId[] => Object.keys(s.agents).filter((id) => s.agents[id].computerId === computerId);

/** Sets a computer online (stamped now) or offline, and tells the pod about each hosted agent whose presence changed. */
function set(c: Ctx, computerId: ComputerId, online: boolean): void {
  const ids = hosted(c.s, computerId);
  const before = ids.map((id) => agentPresence(c.s, id));
  if (online) (c.s.online ??= {})[computerId] = c.now;
  else if (c.s.online !== undefined) delete c.s.online[computerId];
  ids.forEach((id, i) => { if (agentPresence(c.s, id) !== before[i]) announce(c, id); });
}

/** Any sign of life from a computer: its socket opened, or a heartbeat. */
export const computerSeen = (c: Ctx, computerId: ComputerId): void => set(c, computerId, true);
export const computerGone = (c: Ctx, computerId: ComputerId): void => set(c, computerId, false);

export function computerOnlineCmd(c: Ctx, cmd: Of<'computer_online'>): void {
  computerSeen(c, cmd.computerId);
  ok(c);
}

export function computerOfflineCmd(c: Ctx, cmd: Of<'computer_offline'>): void {
  computerGone(c, cmd.computerId);
  ok(c);
}

/** A newly registered (or moved) agent tells the pod what it shows, so members learn of it without reloading. */
export const announceAgent = announce;

/** When each online computer goes stale: its last sign of life plus 45 s. */
export const staleAt = (s: { online?: Record<ComputerId, number> }): Array<[ComputerId, number]> => Object.entries(s.online ?? {}).map(([id, seen]) => [id, seen + PRESENCE_STALE_MS]);
