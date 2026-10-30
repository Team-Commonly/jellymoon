import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import Ajv2020 from 'ajv/dist/2020.js';
import { LEASE_MS, apply, emptyPod, isoOf, nextDeadline } from '../src';
import type { Cmd } from '../src';
import { CMP_1, POD_ID, RUN_A, RUN_C, TASKS, U_OWNER, U_STRANGER, errCode, events, id, lastReply, play, released, types, world } from './helpers';

const [T1] = TASKS;
const owner = { kind: 'user' as const, userId: U_OWNER.id };
const create = (c: Cmd = { type: 'pod_create', ownerId: U_OWNER.id, ownerName: 'Pod Owner' }) => play([[c, 0]], emptyPod(POD_ID));

describe('pod_create, members and the gate', () => {
  test('before pod_create every command gets not_found; pod_create makes the owner a member and owner', () => {
    for (const cmd of [{ type: 'tick' }, { type: 'heartbeat', computerId: CMP_1 }, { type: 'member_add', userId: U_OWNER.id, name: 'x' }] as Cmd[]) {
      expect(errCode(play([[cmd, 0]], emptyPod(POD_ID)).out)).toBe('not_found');
    }
    const r = create();
    expect(r.state.members).toEqual([U_OWNER]);
    expect(r.state.owners).toEqual([U_OWNER]);
    expect(errCode(play([[{ type: 'pod_create', ownerId: U_STRANGER.id, ownerName: 'x' }, 1]], r.state).out)).toBe('invalid_input');
  });
  test('member_add admits a person once; a stranger is forbidden before not_found', () => {
    const probe: Cmd = { type: 'task_release', caller: { kind: 'user', userId: U_STRANGER.id }, taskId: id('tsk', 99) };
    expect(errCode(play([[probe, 1]]).out)).toBe('forbidden');
    const add: Cmd = { type: 'member_add', userId: U_STRANGER.id, name: U_STRANGER.displayName };
    const r = play([[add, 1], [add, 2], [probe, 3]]);
    expect(r.state.members.filter((m) => m.id === U_STRANGER.id)).toEqual([U_STRANGER]);
    expect(errCode(r.out)).toBe('not_found');
  });
  test('task_create: members and live runs only; emits task.created open; a taken id is refused', () => {
    const mk = (caller: Cmd extends infer _ ? { kind: 'user'; userId: string } | { kind: 'run'; runId: string } : never, taskId = id('tsk', 20)): Cmd => ({ type: 'task_create', caller, taskId, title: 'New', kind: 'fix', body: 'b', wakeId: id('wak', 1) });
    const r = play([[mk(owner), 5]]);
    expect(events(r.out)[0].data).toMatchObject({ id: id('tsk', 20), status: 'open', createdBy: U_OWNER, kind: 'fix', body: 'b', createdAt: isoOf(5) });
    expect(errCode(play([[mk({ kind: 'user', userId: U_STRANGER.id }), 5]]).out)).toBe('forbidden');
    expect(errCode(play([[mk({ kind: 'run', runId: RUN_A }), 5]]).out)).toBeNull();
    expect(errCode(play([[mk(owner, T1), 5]]).out)).toBe('invalid_input');
  });
});

describe('message_post summary', () => {
  const bundle = JSON.parse(readFileSync(new URL('../../gen/ts/schemas.json', import.meta.url), 'utf8'));
  const ajv = new Ajv2020({ allErrors: true, strict: true });
  ajv.addSchema(bundle);
  const valid = ajv.getSchema(`${bundle.$id}#/$defs/Message`);
  const post = (text: string) => events(play([[{ type: 'message_post', caller: owner, messageId: id('msg', 1), text, wakeIds: [] }, 0]]).out)[0].data as { summary: string };
  const cases: Array<[string, string]> = [['\nhello', 'hello'], ['line one\r\nline two', 'line one'], ['a\rb', 'a'], ['   ', '(no text)'], ['', '(no text)'], ['\u{1F600}'.repeat(200), '\u{1F600}'.repeat(120)], ['hello', 'hello'], ['  \n  padded  \nnext', 'padded']];
  test('the first non-blank line, trimmed and cut to 120 code points, validates as a Message for every text', () => {
    for (const [text, summary] of cases) {
      const m = post(text);
      expect([text, m.summary]).toEqual([text, summary]);
      expect([text, valid?.(m)]).toEqual([text, true]);
    }
  });
});

describe('boot', () => {
  test('the verifier\'s boot sequence: a restored timeline holds the same claim at 130 000 as the straight one', () => {
    const s0 = play([[{ type: 'claim', runId: RUN_C, taskId: TASKS[1] }, 0]]).state;
    const rest: Array<[Cmd, number]> = [[{ type: 'claim', runId: RUN_C, taskId: TASKS[2] }, 62_000], [{ type: 'claim', runId: RUN_A, taskId: T1 }, 63_000], [{ type: 'claim', runId: RUN_A, taskId: TASKS[1] }, 100_000], [{ type: 'tick' }, 130_000]];
    const straight = play(rest, s0);
    const restored = play(rest, apply(JSON.parse(JSON.stringify(s0)), { type: 'boot', staleMs: 15_000 }, 10_000).state);
    expect(straight.state.claims[T1]).toEqual({ runId: RUN_A, leaseUntil: 160_000 });
    expect(restored.state.claims[T1]).toEqual(straight.state.claims[T1]);
    expect(released(restored.out).filter((x) => x.taskId === T1)).toEqual([]);
  });
  const claimed = play([[{ type: 'claim', runId: RUN_A, taskId: T1 }, 0]]).state;
  const boot = (staleMs: number, now: number) => apply(claimed, { type: 'boot', staleMs }, now);
  test('a saved lease within staleMs of the restart is still claimed', () => {
    expect(boot(15_000, 70_000).state.claims[T1].leaseUntil).toBe(75_000);
    expect(boot(15_000, 70_000).out).toEqual([]);
  });
  test('staleMs 0 equals a plain tick', () => {
    for (const now of [30_000, 70_000]) expect(boot(0, now)).toEqual(apply(claimed, { type: 'tick' }, now));
  });
  test('boot never shortens a lease and never touches the run cap', () => {
    const early = boot(LEASE_MS, 0);
    expect(early.state.claims[T1].leaseUntil).toBe(60_000);
    expect(early.state.runs).toEqual(claimed.runs);
    expect(nextDeadline(boot(15_000, 10_000).state)).toBe(70_000);
  });
  test('a lease whose extended deadline has passed is released by the settle inside boot, stamped with it', () => {
    const r = boot(30_000, 200_000);
    expect(released(r.out)).toEqual([{ taskId: T1, runId: RUN_A, cause: 'expired' }]);
    expect(events(r.out).map((e) => e.ts)).toEqual([isoOf(90_000), isoOf(90_000)]);
    expect(types(r.out)).toEqual(['task.updated', 'claim.released']);
    expect(lastReply(r.out)).toBe(undefined);
  });
  test('an out-of-range staleMs is a plain Error', () => {
    for (const bad of [-1, LEASE_MS + 1, 1.5]) expect(() => apply(world(), { type: 'boot', staleMs: bad }, 0)).toThrow();
  });
});
