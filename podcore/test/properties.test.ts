import { expect, test } from 'bun:test';
import { apply } from '../src';
import type { Cmd, Out, PodState } from '../src';
import { AG1, AG3, CMP_1, CMP_2, RUN_A, RUN_B, RUN_C, TASKS, U_AGENT_OWNER, U_OWNER, U_STRANGER, events, id, prng, runN, wak, world } from './helpers';

const RUNS = [RUN_A, RUN_B, RUN_C];
const AGENTS = [AG1, AG3, 'agt_00000000000000000000000002'];
const USERS = [U_OWNER.id, U_AGENT_OWNER.id, U_STRANGER.id];
const TEXTS = ['hi', '@agent-1 look', '`@agent-1` no', '@agent-3 and @agent-1', 'x @agent-2'];

/** A random but replayable command with a monotonic clock; big jumps now and then reach the 2 h cap. */
/** `claimy` keeps to claims, ops, heartbeats, releases and ticks in lease-sized steps, so lease rules are exercised hard. */
function sequence(seed: number, length = 60, claimy = false): Array<[Cmd, number]> {
  const r = prng(seed);
  const pick = <T>(xs: T[]): T => xs[Math.floor(r() * xs.length)];
  let now = 0;
  let n = 0;
  let w = 0;
  const wid = (): string => wak((w += 4));
  return Array.from({ length }, () => {
    now += !claimy && r() < 0.05 ? Math.floor(r() * 3_600_000) : Math.floor(r() * (claimy ? 40_000 : 45_000));
    const caller = () => (r() < 0.5 ? { kind: 'user' as const, userId: pick(USERS) } : { kind: 'run' as const, runId: pick(RUNS) });
    const gens: Array<() => Cmd> = [
      () => ({ type: 'claim', runId: pick(RUNS), taskId: pick(TASKS) }),
      () => ({ type: 'op', runId: pick(RUNS), taskId: pick(TASKS) }),
      () => ({ type: 'heartbeat', computerId: pick([CMP_1, CMP_2]) }),
      () => ({ type: 'task_release', caller: caller(), taskId: pick(TASKS) }),
      () => ({ type: 'run_end', runId: pick(RUNS), computerId: pick([CMP_1, CMP_2]), outcome: 'completed' }),
      () => ({ type: 'run_stop', caller: caller(), runId: pick(RUNS) }),
      () => ({ type: 'task_create', caller: caller(), taskId: id('tsk', 10 + (n += 1)), title: 't', kind: 'chore', assignee: r() < 0.7 ? pick(AGENTS) : undefined, wakeId: wid() }),
      () => ({ type: 'task_update', caller: caller(), taskId: pick(TASKS), status: 'in_progress', assignee: pick(AGENTS), wakeId: wid() }),
      () => ({ type: 'message_post', caller: caller(), messageId: id('msg', (n += 1)), text: pick(TEXTS), summary: 's', wakeId: wid() }),
      () => ({ type: 'change', kind: 'ruling', agentId: pick(AGENTS), refs: {}, wakeId: wid() }),
      () => ({ type: 'wake_ack', wakeId: wak(4 * (1 + Math.floor(r() * (w / 4 + 1)))), computerId: pick([CMP_1, CMP_2]) }),
      () => ({ type: 'run_claim', wakeId: wak(4 * (1 + Math.floor(r() * (w / 4 + 1)))), computerId: pick([CMP_1, CMP_2]), runId: runN((n += 1)) }),
      () => ({ type: 'agent_register', agentId: pick(AGENTS), ownerId: U_AGENT_OWNER.id, computerId: pick([CMP_1, CMP_2]) }),
      () => ({ type: 'tick' }),
    ];
    const cmd: Cmd = pick(claimy ? [gens[0], gens[0], gens[1], gens[1], gens[2], gens[3], gens[13]] : gens)();
    return [cmd, now];
  });
}

function run(steps: Array<[Cmd, number]>, mid?: (state: PodState, i: number) => PodState): { state: PodState; out: Out[]; per: Out[][] } {
  let state = world();
  const out: Out[] = [];
  const per: Out[][] = [];
  steps.forEach(([cmd, now], i) => {
    if (mid !== undefined) state = mid(state, i);
    const r = apply(state, cmd, now);
    state = r.state;
    out.push(...r.out);
    per.push(r.out);
  });
  return { state, out, per };
}

const SEEDS = Array.from({ length: 500 }, (_, i) => i + 1);
const PROPERTY_MS = 60_000; // 500 seeds run in ~1 s alone but took 7-19 s on a loaded shared host, past bun's 5 s default
const stamps = (out: Out[]) => out.flatMap((o) => (o.kind === 'reply' ? [] : [o.event.ts as string]));

test('restart property: a JSON round-trip of the state at a random step changes nothing', () => {
  for (const seed of SEEDS) {
    const steps = sequence(seed);
    const at = Math.floor(prng(seed * 7919)() * steps.length);
    const straight = run(steps);
    const restarted = run(steps, (s, i) => (i === at ? JSON.parse(JSON.stringify(s)) : s));
    expect(restarted.out).toEqual(straight.out);
    expect(restarted.state).toEqual(straight.state);
  }
}, PROPERTY_MS);

test('seq property: members-event seq has no gaps, and ts never decreases', () => {
  for (const seed of SEEDS) {
    const out = run(sequence(seed)).out;
    const seqs = events(out).map((e) => e.seq);
    expect(seqs).toEqual(seqs.map((_, i) => i + 1));
    const ts = stamps(out);
    expect(ts).toEqual([...ts].sort());
  }
}, PROPERTY_MS);

test('one-release property: claims started = claim.released + claims still held, per cause, against a tiny reference model', () => {
  for (const seed of SEEDS) {
    const held = new Map<string, string>();
    const tally: Record<string, number> = {};
    let started = 0;
    const { per, state } = run(sequence(seed));
    for (const e of events(per.flat())) {
      if (e.type === 'task.claimed') {
        expect(held.has(e.data.taskId)).toBe(false);
        held.set(e.data.taskId, e.data.runId);
        started += 1;
      } else if (e.type === 'claim.released') {
        expect(held.get(e.data.taskId)).toBe(e.data.runId);
        held.delete(e.data.taskId);
        tally[e.data.cause] = (tally[e.data.cause] ?? 0) + 1;
        expect(e.data.by !== undefined).toBe(e.data.cause === 'released' || e.data.cause === 'stopped');
      }
    }
    const total = Object.values(tally).reduce((a, b) => a + b, 0);
    expect(started).toBe(total + Object.keys(state.claims).length);
    expect(Object.fromEntries(held)).toEqual(Object.fromEntries(Object.entries(state.claims).map(([t, k]) => [t, k.runId])));
  }
}, PROPERTY_MS);

test('wake property: one pending wake per agent, and every run.wake goes to the computer hosting the agent', () => {
  for (const seed of SEEDS) {
    let state = world();
    for (const [cmd, now] of sequence(seed)) {
      const r = apply(state, cmd, now);
      for (const o of r.out) {
        if (o.kind === 'to-computer' && o.event.type === 'run.wake') {
          expect(o.computerId).toBe(r.state.agents[o.event.data.agentId].computerId);
          expect(r.state.wakes[o.event.data.agentId].id).toBe(o.event.data.wakeId);
        }
      }
      for (const [agentId, w] of Object.entries(r.state.wakes)) expect(w.agentId).toBe(agentId);
      state = r.state;
    }
  }
}, PROPERTY_MS);
