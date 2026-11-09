import { describe, expect, test } from 'bun:test';
import { cut, digestOf } from '../src/evidence';

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
