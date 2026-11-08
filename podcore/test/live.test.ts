import { describe, expect, test } from 'bun:test';
import { runLive } from '../src';
import { CMP_1, RUN_A, RUN_B, U_OWNER, id, play, world } from './helpers';

describe('runLive', () => {
  test('a run is live until it ends or is stopped, and a name that is no run is never live', () => {
    const s = world();
    expect(runLive(s, RUN_A)).toBe(true);
    for (const bad of ['__proto__', 'constructor', 'toString', '', id('run', 99)]) expect(runLive(s, bad)).toBe(false);
    const ended = play([[{ type: 'run_end', runId: RUN_A, computerId: CMP_1, outcome: 'completed' }, 1000]]).state;
    expect([runLive(ended, RUN_A), runLive(ended, RUN_B)]).toEqual([false, true]);
    const stopped = play([[{ type: 'run_stop', caller: { kind: 'user', userId: U_OWNER.id }, runId: RUN_A }, 1000]]).state;
    expect([runLive(stopped, RUN_A), runLive(stopped, RUN_B)]).toEqual([false, true]);
  });
});
