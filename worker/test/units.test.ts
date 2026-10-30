import { expect, test } from 'bun:test';
import { parseFrame } from '../src/frames';
import { nthWakeId } from '../../podcore/src/index';
import { base62, hashToken, newId, newToken, newWakeSeed } from '../src/ids';
import { decodeWho, encodeWho, intIn, parse, pick, str } from '../src/wire';
import type { Who } from '../src/wire';

const user: Who = { kind: 'user', actor: { kind: 'human', id: 'usr_01ARZ3NDEKTSV4RRFFQ69G5FAV', displayName: 'olivia' } };
const computer: Who = { kind: 'computer', computerId: 'cmp_1', owner: { kind: 'human', id: 'usr_01ARZ3NDEKTSV4RRFFQ69G5FAV', displayName: 'ben' } };
const frame = (type: string, data: unknown, extra: object = {}): string => JSON.stringify({ v: 1, type, podId: 'pod_1', data, ...extra });

test('ids sort in creation order and tokens hash to 64 hex characters', async () => {
  const ids = Array.from({ length: 50 }, () => newId('tsk_'));
  expect([...ids].sort()).toEqual(ids);
  expect(/^[0-9A-Za-z]{40}$/.test(base62(40))).toBe(true);
  expect(/^jmu_[0-9A-Za-z]{43}$/.test(newToken('jmu_'))).toBe(true);
  expect(/^[0-9a-f]{64}$/.test(await hashToken('x'))).toBe(true);
});

test('a wake seed is a wak_ ULID the core can count up from, and two seeds never sit next to each other', () => {
  const seeds = Array.from({ length: 50 }, () => newWakeSeed());
  for (const s of seeds) expect(/^wak_[0-9A-HJKMNP-TV-Z]{26}$/.test(s) && nthWakeId(s, 1000) !== null).toBe(true);
  const taken = new Set(seeds.flatMap((s) => [0, 1, 2].map((k) => nthWakeId(s, k))));
  expect(taken.size).toBe(seeds.length * 3);
});

test('the identity header round-trips and refuses garbage', () => {
  expect(decodeWho(encodeWho(user))).toEqual(user);
  expect(decodeWho(null)).toBeNull();
  expect(decodeWho('%7Bnot json')).toBeNull();
});

test('validators', async () => {
  expect(str('abc', 1, 3)).toBe('abc');
  expect(str('abcd', 1, 3)).toBe(undefined);
  expect(pick('a', ['a', 'b'] as const)).toBe('a');
  expect(intIn(undefined, 5, 1, 9)).toBe(5);
  expect(intIn(10, 5, 1, 9)).toBeNull();
  expect(await parse(new Request('http://x', { method: 'POST', body: '[1]' }))).toBeNull();
  expect(await parse(new Request('http://x', { method: 'POST', body: '' }))).toEqual({});
});

test('socket frames: who may send what, and the envelope rules', () => {
  expect(parseFrame('pod_1', user, frame('subscribe', { sinceSeq: 3 }))).toEqual({ subscribe: 3 });
  expect(parseFrame('pod_1', user, frame('subscribe', { sinceSeq: -1 }))).toMatchObject({ code: 'invalid_input' });
  expect(parseFrame('pod_1', user, frame('presence.heartbeat', {}))).toMatchObject({ code: 'forbidden' });
  expect(parseFrame('pod_1', computer, frame('message.post', { text: 'hi' }))).toMatchObject({ code: 'forbidden' });
  expect(parseFrame('pod_1', computer, frame('presence.heartbeat', {}))).toEqual({ cmd: { type: 'heartbeat', computerId: 'cmp_1' } });
  expect(parseFrame('pod_1', computer, frame('presence.heartbeat', { computerId: 'cmp_2' }))).toMatchObject({ code: 'forbidden' });
  expect(parseFrame('pod_2', user, frame('subscribe', { sinceSeq: 0 }))).toMatchObject({ code: 'invalid_input' });
  expect(parseFrame('pod_1', user, frame('subscribe', { sinceSeq: 0 }, { seq: 1 }))).toMatchObject({ code: 'invalid_input' });
  expect(parseFrame('pod_1', user, 'not json')).toMatchObject({ code: 'invalid_input' });
  expect(parseFrame('pod_1', user, frame('task.create', { title: 't', kind: 'chore' }))).toMatchObject({ cmd: { type: 'task_create' } });
});
