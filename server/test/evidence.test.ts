import { describe, expect, test } from 'bun:test';
import type { AnyEvent } from '../../gen/ts/protocol';
import type { PodState } from '../../podcore/src/index';
import { openDb } from '../src/db';
import { cut, digestOf } from '../src/evidence';
import { messagesById, migrate, savePod, saveToolCall, toolCallsById } from '../src/store';

// The text of a record (ADR 0006, "Evidence"): a name and a digest are cut by code point and never hold a NUL.
describe('what a record keeps of a call', () => {
  test('cut counts code points, not UTF-16 units, and drops NUL', () => {
    expect(cut('abc', 2)).toBe('ab');
    expect(cut('a\0b\0c', 3)).toBe('abc');
    expect(cut('😀😀😀', 2)).toBe('😀😀');
    expect(cut('', 5)).toBe('');
  });

  test('a digest is at most 600 characters whatever the input and output, and starts with the tool', () => {
    const big = 'x'.repeat(5_000_000);
    const d = digestOf('jm_read', { sinceSeq: 0 }, big);
    expect([...d].length).toBe(600);
    expect(d.startsWith('jm_read {"sinceSeq":0} -> xxx')).toBe(true);
  });

  test('prototype keys and odd inputs are plain text, never looked up', () => {
    const input = JSON.parse('{"__proto__":{"polluted":true},"constructor":1}') as unknown;
    const d = digestOf('__proto__', input, '');
    expect(d.startsWith('__proto__ ')).toBe(true);
    expect(d.includes('constructor')).toBe(true);
    expect(({} as { polluted?: boolean }).polluted).toBe(undefined);
    expect(digestOf('n', undefined, 'o')).toBe('n  -> o');
    expect(digestOf('n', 'a\0b', 'c\0')).toBe('n "a\\u0000b" -> c');
  });
});

// What an ask may cite (F25): the adapter hands the core only records of the asking pod, whatever ids the ask names.
describe('the cited records the server looks up', () => {
  test('a tool call or message of another pod is not found, and neither is an id that is no record', async () => {
    const db = await openDb({});
    await migrate(db);
    const row = (id: string, pod: string) => ({ id, pod, run: 'run_A', n: Number(id.slice(-1)), name: 'jm_whoami', status: 'ok' as const, startedAt: 1, endedAt: 2, digest: `d ${id}` });
    await saveToolCall(db, row('tc_1', 'pod_A'));
    await saveToolCall(db, row('tc_2', 'pod_B'));
    expect(await toolCallsById(db, 'pod_A', ['tc_1', 'tc_2', 'tc_9', '__proto__'])).toEqual([{ id: 'tc_1', run: 'run_A', digest: 'd tc_1' }]);
    expect(await toolCallsById(db, 'pod_A', [])).toEqual([]);
    const msg = (pod: string, id: string) => ({ v: 1, type: 'message.created', seq: 1, podId: pod, ts: '', data: { id, summary: `s ${id}`, body: 'b' } }) as unknown as AnyEvent;
    const meta = { name: '', createdAt: '', harness: {} };
    await db.tx((t) => savePod(t, { id: 'pod_A', state: {} as PodState, meta }, [msg('pod_A', 'msg_1')]));
    await db.tx((t) => savePod(t, { id: 'pod_B', state: {} as PodState, meta }, [msg('pod_B', 'msg_2')]));
    expect(await messagesById(db, 'pod_A', ['msg_1', 'msg_2', "x' or '1'='1"])).toEqual([{ id: 'msg_1', summary: 's msg_1', body: 'b' }]);
  });
});
