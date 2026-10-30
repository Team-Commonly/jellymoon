import { expect, test } from 'bun:test';
import { readdirSync, readFileSync } from 'node:fs';
import { isoOf } from '../src';

const BARE = ['Date', 'crypto', 'fetch', 'process', 'performance', 'globalThis', 'Intl', 'Atomics', 'Temporal', 'WeakRef', 'FinalizationRegistry'];
// Math and Bun stay usable by name (Math.floor); they are only banned for bracket access, Math.random and Bun.
const GLOBALS = [...BARE, 'Math', 'Bun'];
const BANNED = [...BARE, 'setTimeout', 'setInterval', 'Math.random', 'await', 'eval', 'queueMicrotask', 'Function(', 'Bun.', '.then('];
const escape = (w: string): string => w.replace(/[.()[\]]/g, '\\$&');
const edge = (w: string, pre: boolean): string => (/[A-Za-z]/.test(pre ? w[0] : w[w.length - 1]) ? (pre ? '(?<![A-Za-z0-9_$])' : '(?![A-Za-z0-9_$])') : '');
const rules = [
  ...BANNED.map((w) => new RegExp(`${edge(w, true)}${escape(w)}${edge(w, false)}`)),
  ...GLOBALS.map((g) => new RegExp(`(?<![A-Za-z0-9_$.])${g}\\s*\\[`)),
];

test('podcore/src, read recursively, uses no clock, timer, randomness, network, platform global, eval or bracket access to one', () => {
  const dir = new URL('../src/', import.meta.url);
  const files = readdirSync(dir, { recursive: true }).filter((f: string) => f.endsWith('.ts'));
  expect(files.length).toBeGreaterThan(0);
  for (const f of files) {
    const text = readFileSync(new URL(f, dir), 'utf8');
    for (const re of rules) expect(re.test(text) ? `${f}: ${re.source}` : null).toBeNull();
  }
});

test('isoOf rejects a non-integer now', () => {
  expect(() => isoOf(1.5)).toThrow();
  expect(() => isoOf(Number.NaN)).toThrow();
});
