import { expect, test } from 'bun:test';
import { apply } from '../src';
import { own } from '../src/claims';
import type { Cmd } from '../src';
import { AG1, CMP_1, RUN_A, RUN_B, TASKS, U_OWNER, errCode, id, wak, world } from './helpers';

const BAD = ['__proto__', 'constructor', 'toString', 'hasOwnProperty', 'valueOf'];
const user = { kind: 'user' as const, userId: U_OWNER.id };
const run = { kind: 'run' as const, runId: RUN_A };
const SAMPLES: Cmd[] = [
  { type: 'claim', runId: RUN_A, taskId: TASKS[0] }, { type: 'op', runId: RUN_A, taskId: TASKS[0] }, { type: 'heartbeat', computerId: CMP_1 },
  { type: 'task_release', caller: user, taskId: TASKS[0] }, { type: 'task_release', caller: run, taskId: TASKS[0] },
  { type: 'run_end', runId: RUN_A, computerId: CMP_1, outcome: 'completed' }, { type: 'run_stop', caller: user, runId: RUN_A }, { type: 'run_stop', caller: run, runId: RUN_B },
  { type: 'pod_create', ownerId: U_OWNER.id, ownerName: 'x' }, { type: 'member_add', userId: id('usr', 9), name: 'x' },
  { type: 'task_create', caller: user, taskId: id('tsk', 20), title: 't', kind: 'chore', assignee: AG1, wakeId: wak(1) },
  { type: 'task_update', caller: user, taskId: TASKS[0], status: 'done', assignee: AG1, wakeId: wak(1) },
  { type: 'message_post', caller: run, messageId: id('msg', 1), threadOf: id('msg', 2), text: '@agent-1', wakeId: wak(1) },
  { type: 'change', kind: 'ruling', agentId: AG1, refs: { taskId: TASKS[0], messageId: id('msg', 1) }, wakeId: wak(1) },
  { type: 'wake_ack', wakeId: wak(1), computerId: CMP_1 }, { type: 'run_claim', wakeId: wak(1), computerId: CMP_1, runId: id('run', 50) },
  { type: 'agent_register', agentId: AG1, ownerId: U_OWNER.id, computerId: CMP_1 },
];

/** Every copy of `v` with one id-like string field (name ending in Id, or assignee), at any depth, replaced by `bad`. */
function variants(v: unknown, bad: string): unknown[] {
  if (typeof v !== 'object' || v === null) return [];
  return Object.entries(v).flatMap(([k, x]) => {
    const swap = typeof x === 'string' && /Id$|^assignee$/.test(k) ? [{ ...v, [k]: bad }] : [];
    return [...swap, ...variants(x, bad).map((y) => ({ ...v, [k]: y }))];
  });
}

const protoState = () => {
  const names = Object.getOwnPropertyNames(Object.prototype);
  return { names, values: names.map((n) => Object.getOwnPropertyDescriptor(Object.prototype, n)?.value) };
};
const TOUCHED = ['status', 'updatedAt', 'assignee', 'title', 'id', 'runId', 'leaseUntil', 'computerId', 'agent', 'owner'];

test('an id equal to an Object.prototype key is not_found: no throw, state unchanged, Object.prototype untouched', () => {
  const base = world();
  const before = protoState();
  let tried = 0;
  for (const cmd of SAMPLES) {
    for (const bad of BAD) {
      for (const v of variants(cmd, bad)) {
        const r = apply(base, v as Cmd, 5);
        tried += 1;
        expect([JSON.stringify(v), errCode(r.out), r.out.length, r.state]).toEqual([JSON.stringify(v), 'not_found', 1, base]);
        expect(protoState()).toEqual(before);
        for (const k of TOUCHED) expect((Object.prototype as Record<string, unknown>)[k]).toBe(undefined);
      }
    }
  }
  expect(tried).toBeGreaterThan(150);
});

test('own() finds only own keys, so even a lookup that skipped the refusal cannot reach a prototype member', () => {
  const rec: Record<string, number> = { a: 1 };
  expect([own(rec, 'a'), own(rec, 'constructor'), own(rec, '__proto__'), own(rec, 'toString')]).toEqual([1, undefined, undefined, undefined]);
});
