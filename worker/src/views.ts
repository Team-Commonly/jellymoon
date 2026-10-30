// Read models derived from core state. The core exports commands and events but no read model, so the adapter derives these.
import type { Actor, AgentPresence, Harness, Hello, Task } from '../../gen/ts/protocol';
import { isoOf } from '../../podcore/src/index';
import type { PodState } from '../../podcore/src/index';
import type { PodMeta } from './wire';

export function taskView(s: PodState, id: string): Task {
  const { claim: _stored, ...task } = s.tasks[id];
  const held = s.claims[id];
  if (held === undefined) return task;
  return { ...task, claim: { by: s.runs[held.runId].agent, runId: held.runId, leaseUntil: isoOf(held.leaseUntil) } };
}

export const tasksOf = (s: PodState, status?: string): Task[] =>
  Object.keys(s.tasks).sort().map((id) => taskView(s, id)).filter((t) => status === undefined || t.status === status);

function presence(s: PodState, a: Actor, computerId: string, online: Set<string>): AgentPresence['state'] {
  if (Object.values(s.runs).some((r) => r.status === 'active' && r.agent.id === a.id)) return 'working';
  return online.has(computerId) ? 'online' : 'offline';
}

export function hello(s: PodState, meta: PodMeta, you: Actor, online: Set<string>): Hello {
  return {
    podId: s.podId,
    headSeq: s.seq,
    you,
    members: s.members.map((actor) => ({ actor, role: s.owners.some((o) => o.id === actor.id) ? 'owner' : 'member' })),
    agents: Object.values(s.agents).map((a) => ({
      agent: a.actor, harness: (meta.harness[a.actor.id] ?? 'fake') as Harness, computerId: a.computerId, state: presence(s, a.actor, a.computerId, online),
    })),
  };
}
