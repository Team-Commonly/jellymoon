import { describe, expect, test } from 'bun:test';
import type { Cmd, Out } from '../src';
import { AG1, CMP_1, CMP_2, RUN_A, RUN_B, TASKS, U_OWNER, U_STRANGER, events, id, play, runN, wak, world } from './helpers';

const [T1] = TASKS;
const CAP = 7_200_000;
const owner = { kind: 'user' as const, userId: U_OWNER.id };
const agent1 = world().agents[AG1].actor;
const runOf = (out: Out[]) => events(out).flatMap((e) => (e.type === 'run.started' || e.type === 'run.ended' ? [{ type: e.type, seq: e.seq, data: e.data }] : []));
const ended = (out: Out[]) => events(out).flatMap((e) => (e.type === 'run.ended' ? [e.data] : []));
const end = (outcome: 'completed' | 'failed' | 'cancelled', runId = RUN_A, computerId = CMP_1): Cmd => ({ type: 'run_end', runId, computerId, outcome });
const stop = (runId = RUN_A): Cmd => ({ type: 'run_stop', caller: owner, runId });
const assign: Cmd = { type: 'task_create', caller: owner, taskId: id('tsk', 11), title: 'T', kind: 'chore', assignee: AG1, wakeId: wak(1) };
const claimWake = (computerId = CMP_1): Cmd => ({ type: 'run_claim', wakeId: wak(1), computerId, runId: runN(1) });

describe('run.started', () => {
  test('a computer claiming a wake starts a run: one run.started with its id and agent, carrying the next seq', () => {
    const r = play([[assign, 0], [claimWake(), 1]]);
    const got = runOf(r.out);
    expect(got.map((e) => [e.type, e.data])).toEqual([['run.started', { runId: runN(1), agent: agent1 }]]);
    expect(got[0].seq).toBe(r.state.seq);
  });

  test('a refused run_claim starts no run and emits nothing', () => {
    expect(runOf(play([[assign, 0], [claimWake(CMP_2), 1]]).out)).toEqual([]);
    expect(runOf(play([[claimWake(), 1]]).out)).toEqual([]);
  });
});

describe('run.ended fires once, with the outcome of the path that ended it', () => {
  for (const outcome of ['completed', 'failed', 'cancelled'] as const) {
    test(`the daemon reports ${outcome}`, () => {
      expect(ended(play([[end(outcome), 10]]).out)).toEqual([{ runId: RUN_A, agent: agent1, outcome }]);
    });
  }

  test('a person stopping the run: cancelled', () => {
    expect(ended(play([[stop(), 10]]).out)).toEqual([{ runId: RUN_A, agent: agent1, outcome: 'cancelled' }]);
  });

  test('the 2 h token cap: expired, stamped with the cap, after the claim it held is released', () => {
    const r = play([[{ type: 'claim', runId: RUN_A, taskId: T1 }, CAP - 50_000], [{ type: 'tick' }, CAP + 100_000]]);
    const ev = events(r.out).filter((e) => e.type === 'claim.released' || e.type === 'run.ended');
    expect(ev.map((e) => e.type)).toEqual(['claim.released', 'run.ended', 'run.ended', 'run.ended']);
    expect(ended(r.out).find((d) => d.runId === RUN_A)).toEqual({ runId: RUN_A, agent: agent1, outcome: 'expired' });
    expect(ev[1].ts).toBe(new Date(CAP).toISOString());
  });

  test('a lease that expires ends the claim, not the run', () => {
    const r = play([[{ type: 'claim', runId: RUN_A, taskId: T1 }, 0], [{ type: 'tick' }, 70_000]]);
    expect(ended(r.out)).toEqual([]);
  });

  test('once only: stop after an end, end after a stop, a second stop, and an end after the cap emit nothing new', () => {
    const cases: Array<Array<[Cmd, number]>> = [
      [[end('completed'), 10], [stop(), 20]],
      [[stop(), 10], [end('cancelled'), 20]],
      [[stop(), 10], [stop(), 20]],
      [[end('failed'), 10], [end('completed'), 20]],
      [[{ type: 'tick' }, CAP], [stop(), CAP + 1], [end('completed'), CAP + 2]],
    ];
    for (const steps of cases) expect(ended(play(steps).out).filter((d) => d.runId === RUN_A).length).toBe(1);
  });

  test('a refused stop or end emits nothing: a stranger, a run token, the wrong computer, an unknown run', () => {
    const steps: Array<[Cmd, number]> = [
      [{ type: 'run_stop', caller: { kind: 'user', userId: U_STRANGER.id }, runId: RUN_A }, 1],
      [{ type: 'run_stop', caller: { kind: 'run', runId: RUN_B }, runId: RUN_A }, 2],
      [end('completed', RUN_A, CMP_2), 3],
      [stop(id('run', 99)), 4],
      [end('completed', id('run', 99)), 5],
    ];
    expect(ended(play(steps).out)).toEqual([]);
  });

  test('a run that never started never ends: a full start-stop cycle emits exactly started then ended', () => {
    const r = play([[assign, 0], [claimWake(), 1], [stop(runN(1)), 2], [stop(runN(1)), 3]]);
    expect(runOf(r.out).filter((e) => e.data.runId === runN(1)).map((e) => e.type)).toEqual(['run.started', 'run.ended']);
  });
});
