import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import Ajv2020 from 'ajv/dist/2020.js';
import { LEASE_MS, apply, emptyPod, isoOf } from '../src';
import type { Cmd } from '../src';
import { CMP_1, POD_ID, RUN_A, TASKS, U_OWNER, U_STRANGER, errCode, events, id, lastReply, play } from './helpers';

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
  test('before pod_create a malformed command from anyone is invalid_input, not forbidden (there are no members to judge by)', () => {
    const bad = { type: 'task_release', caller: { kind: 'user', userId: U_STRANGER.id }, taskId: '__proto__' } as Cmd;
    expect(errCode(play([[bad, 0]], emptyPod(POD_ID)).out)).toBe('not_found');
    expect(errCode(play([[{ ...bad, taskId: 7 } as unknown as Cmd, 0]], emptyPod(POD_ID)).out)).toBe('invalid_input');
  });
  test('member_add admits a person once; a stranger is forbidden before not_found', () => {
    const probe: Cmd = { type: 'task_release', caller: { kind: 'user', userId: U_STRANGER.id }, taskId: id('tsk', 99) };
    expect(errCode(play([[probe, 1]]).out)).toBe('forbidden');
    const stranger = { kind: 'user', userId: U_STRANGER.id };
    const malformed = [{ ...probe, taskId: '__proto__' }, { type: 'run_stop', caller: stranger, runId: 'toString' }, { type: 'task_update', caller: stranger, taskId: T1 },
      { type: 'task_create', caller: stranger, taskId: 7 }, { type: 'message_post', caller: stranger, messageId: 'constructor' }] as unknown as Cmd[];
    for (const cmd of malformed) expect([JSON.stringify(cmd), errCode(play([[cmd, 1]]).out)]).toEqual([JSON.stringify(cmd), 'forbidden']);
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
  const post = (text: string) => events(play([[{ type: 'message_post', caller: owner, messageId: id('msg', 1), text, summary: 's', wakeId: id('wak', 1) }, 0]]).out)[0].data as { summary: string };
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
  const claimed = play([[{ type: 'claim', runId: RUN_A, taskId: T1 }, 0]]).state;
  test('boot with staleMs 0 is a plain settle: on a created pod it equals a tick', () => {
    for (const now of [30_000, 70_000]) expect(apply(claimed, { type: 'boot', staleMs: 0 }, now)).toEqual(apply(claimed, { type: 'tick' }, now));
  });
  const STALES: unknown[] = [1, 15_000, LEASE_MS, -1, 1.5, Number.NaN, undefined, '0', null, false];
  test('boot with any staleMs other than 0 is invalid_input and changes nothing (no restart clause in ADR 0002 yet)', () => {
    for (const staleMs of STALES) {
      const r = apply(claimed, { type: 'boot', staleMs } as unknown as Cmd, 30_000);
      expect([staleMs, errCode(r.out), r.state]).toEqual([staleMs, 'invalid_input', claimed]);
    }
  });
  test('a refused boot changes nothing beyond the settle: at a now where a lease is due it equals a tick plus the refusal', () => {
    const tick = apply(claimed, { type: 'tick' }, 70_000);
    for (const staleMs of STALES) {
      const r = apply(claimed, { type: 'boot', staleMs } as unknown as Cmd, 70_000);
      expect([staleMs, errCode(r.out), r.state]).toEqual([staleMs, 'invalid_input', tick.state]);
      expect(r.out.filter((o) => o.kind !== 'reply')).toEqual(tick.out);
    }
  });
  test('boot on an uncreated pod answers nothing (the exemption); tick answers not_found', () => {
    expect(apply(emptyPod(POD_ID), { type: 'boot', staleMs: 0 }, 5_000).out).toEqual([]);
    expect(errCode(apply(emptyPod(POD_ID), { type: 'tick' }, 5_000).out)).toBe('not_found');
  });
});

describe('ErrorBody.ref', () => {
  const bundle = JSON.parse(readFileSync(new URL('../../gen/ts/schemas.json', import.meta.url), 'utf8'));
  const ajv = new Ajv2020({ allErrors: true, strict: true });
  ajv.addSchema(bundle);
  const valid = ajv.getSchema(`${bundle.$id}#/$defs/ErrorBody`);
  test('a 200-character id or a numeric id still gives a schema-valid ErrorBody, with ref left out', () => {
    for (const cmd of [{ type: 'claim', runId: RUN_A, taskId: 't'.repeat(200) }, { type: 'run_claim', wakeId: 'w'.repeat(200), computerId: CMP_1, runId: id('run', 60) }, { type: 'claim', runId: RUN_A, taskId: 5 }]) {
      const e = (lastReply(play([[cmd as Cmd, 5]]).out) as { error: { ref?: string } }).error;
      expect([JSON.stringify(cmd).slice(0, 60), valid?.(e), e.ref]).toEqual([JSON.stringify(cmd).slice(0, 60), true, undefined]);
    }
  });
});
