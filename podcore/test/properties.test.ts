import { expect, test } from 'bun:test';
import { apply } from '../src';
import type { Cmd, Out, PodState } from '../src';
import { CMP_1, CMP_2, RUN_A, RUN_B, RUN_C, TASKS, U_AGENT_OWNER, U_OWNER, U_STRANGER, events, prng, world } from './helpers';

const RUNS = [RUN_A, RUN_B, RUN_C];
const USERS = [U_OWNER.id, U_AGENT_OWNER.id, U_STRANGER.id];

/** A random but replayable command with a monotonic clock; big jumps now and then reach the 2 h cap. */
function sequence(seed: number, length = 60): Array<[Cmd, number]> {
  const r = prng(seed);
  const pick = <T>(xs: T[]): T => xs[Math.floor(r() * xs.length)];
  let now = 0;
  return Array.from({ length }, () => {
    now += r() < 0.05 ? Math.floor(r() * 3_600_000) : Math.floor(r() * 45_000);
    const caller = () => (r() < 0.5 ? { kind: 'user' as const, userId: pick(USERS) } : { kind: 'run' as const, runId: pick(RUNS) });
    const cmd: Cmd = pick<() => Cmd>([
      () => ({ type: 'claim', runId: pick(RUNS), taskId: pick(TASKS) }),
      () => ({ type: 'op', runId: pick(RUNS), taskId: pick(TASKS) }),
      () => ({ type: 'heartbeat', computerId: pick([CMP_1, CMP_2]) }),
      () => ({ type: 'task_release', caller: caller(), taskId: pick(TASKS) }),
      () => ({ type: 'run_end', runId: pick(RUNS), outcome: 'completed' }),
      () => ({ type: 'run_stop', caller: caller(), runId: pick(RUNS) }),
      () => ({ type: 'tick' }),
    ])();
    return [cmd, now];
  });
}

function run(steps: Array<[Cmd, number]>, roundTripAt = -1): { state: PodState; out: Out[]; per: Out[][] } {
  let state = world();
  const out: Out[] = [];
  const per: Out[][] = [];
  steps.forEach(([cmd, now], i) => {
    if (i === roundTripAt) state = JSON.parse(JSON.stringify(state));
    const r = apply(state, cmd, now);
    state = r.state;
    out.push(...r.out);
    per.push(r.out);
  });
  return { state, out, per };
}

const SEEDS = Array.from({ length: 500 }, (_, i) => i + 1);

test('restart property: a JSON round-trip of the state at a random step changes nothing', () => {
  for (const seed of SEEDS) {
    const steps = sequence(seed);
    const at = Math.floor(prng(seed * 7919)() * steps.length);
    const straight = run(steps);
    const restarted = run(steps, at);
    expect(restarted.out).toEqual(straight.out);
    expect(restarted.state).toEqual(straight.state);
  }
});

test('seq property: members-event seq has no gaps and no duplicates', () => {
  for (const seed of SEEDS) {
    const seqs = events(run(sequence(seed)).out).map((e) => e.seq);
    expect(seqs).toEqual(seqs.map((_, i) => i + 1));
  }
});

test('exclusivity property: at no step do two runs hold one task, and every claim that ends says so once', () => {
  for (const seed of SEEDS) {
    const holder = new Map<string, string>();
    let ended = 0;
    let started = 0;
    for (const per of run(sequence(seed)).per) {
      for (const e of events(per)) {
        if (e.type === 'task.claimed') {
          expect(holder.has(e.data.taskId)).toBe(false);
          holder.set(e.data.taskId, e.data.runId);
          started += 1;
        } else if (e.type === 'claim.released') {
          expect(holder.get(e.data.taskId)).toBe(e.data.runId);
          holder.delete(e.data.taskId);
          ended += 1;
        }
      }
    }
    const final = run(sequence(seed)).state;
    expect(started - ended).toBe(Object.keys(final.claims).length);
    expect(Object.fromEntries(holder)).toEqual(Object.fromEntries(Object.entries(final.claims).map(([t, k]) => [t, k.runId])));
  }
});
