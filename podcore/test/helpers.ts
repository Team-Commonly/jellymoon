import type { Actor } from '../../gen/ts/protocol';
import { apply } from '../src';
import type { Cmd, Out, PodState } from '../src';

const id = (prefix: string, n: number): string => `${prefix}_${String(n).padStart(26, '0')}`;
const actor = (kind: Actor['kind'], prefix: string, n: number, displayName: string): Actor => ({ kind, id: id(prefix, n), displayName });

const POD = id('pod', 1);
export const U_OWNER = actor('human', 'usr', 1, 'Pod Owner');
export const U_AGENT_OWNER = actor('human', 'usr', 2, 'Agent Owner');
export const U_STRANGER = actor('human', 'usr', 3, 'Stranger');
export const RUN_A = id('run', 1);
export const RUN_B = id('run', 2);
export const RUN_C = id('run', 3);
export const CMP_1 = id('cmp', 1);
export const CMP_2 = id('cmp', 2);
export const AG1 = id('agt', 1);
export const AG3 = id('agt', 3);
export const AG_UNREG = id('agt', 9);
export const wak = (n: number): string => id('wak', n);
export const runN = (n: number): string => id('run', 100 + n);
export const TASKS = [id('tsk', 1), id('tsk', 2), id('tsk', 3)];

/** Three runs: A and B on computer 1 (owned by the agent owner), C on computer 2. Three open tasks. */
export function world(): PodState {
  const run = (rid: string, n: number, computerId: string) => ({
    id: rid, agent: actor('agent', 'agt', n, `Agent ${n}`), owner: U_AGENT_OWNER, computerId, startedAt: 0, status: 'active' as const,
  });
  const task = (tid: string, n: number) => ({
    id: tid, podId: POD, title: `Task ${n}`, kind: 'chore' as const, status: 'open' as const, assignee: id('agt', 1),
    createdBy: U_OWNER, createdAt: '2026-01-01T00:00:00.000Z', updatedAt: '2026-01-01T00:00:00.000Z',
  });
  return {
    podId: POD, seq: 0, owners: [U_OWNER],
    runs: { [RUN_A]: run(RUN_A, 1, CMP_1), [RUN_B]: run(RUN_B, 2, CMP_1), [RUN_C]: run(RUN_C, 3, CMP_2) },
    tasks: Object.fromEntries(TASKS.map((t, i) => [t, task(t, i + 1)])),
    claims: {}, lost: {}, wakes: {},
    agents: Object.fromEntries([[1, CMP_1], [2, CMP_1], [3, CMP_2]].map(([n, cmp]) => [id('agt', n as number), { actor: actor('agent', 'agt', n as number, `Agent ${n}`), owner: U_AGENT_OWNER, computerId: cmp as string }])),
  };
}

/** Applies commands in order; each is [cmd, now]. Returns the final state and all output. */
export function play(steps: Array<[Cmd, number]>, from: PodState = world()): { state: PodState; out: Out[] } {
  let state = from;
  const out: Out[] = [];
  for (const [cmd, now] of steps) {
    const r = apply(state, cmd, now);
    state = r.state;
    out.push(...r.out);
  }
  return { state, out };
}

export const events = (out: Out[]) => out.flatMap((o) => (o.kind === 'event' ? [o.event] : []));
export const types = (out: Out[]) => events(out).map((e) => e.type);
export const lastReply = (out: Out[]) => [...out].reverse().find((o) => o.kind === 'reply');
export const errCode = (out: Out[]) => {
  const r = lastReply(out);
  return r !== undefined && r.kind === 'reply' && !r.ok ? r.error.code : null;
};
export const toComputer = (out: Out[]) => out.flatMap((o) => (o.kind === 'to-computer' ? [o] : []));
export const released = (out: Out[]) => events(out).flatMap((e) => (e.type === 'claim.released' ? [e.data] : []));

/** mulberry32: a seeded PRNG for the properties (Math.random is banned in src, not here, but seeds replay). */
export function prng(seed: number): () => number {
  let a = seed;
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
