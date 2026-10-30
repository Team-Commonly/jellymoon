import { expect, test } from 'bun:test';
import { readdirSync, readFileSync } from 'node:fs';
import { isoOf } from '../src';

// A lexical backstop, not a proof (ADR 0003): it reads text, so it catches the plain and the aliased spellings of
// a banned name, and the `podcore-is-pure` import rule does the rest. Math is allowed only as these three calls.
const BANNED = [
  'Date', 'crypto', 'fetch', 'process', 'performance', 'globalThis', 'self', 'global', 'window', 'console', 'Reflect', 'Intl', 'Atomics', 'Temporal',
  'WeakRef', 'FinalizationRegistry', 'setTimeout', 'setInterval', 'setImmediate', 'WebAssembly', 'navigator', 'MessageChannel', 'SharedArrayBuffer', 'await', 'eval', 'queueMicrotask', 'require', 'Bun',
];
const MATH_OK = ['floor', 'min', 'max'];
const rules: RegExp[] = [
  ...BANNED.map((w) => new RegExp(`(?<![A-Za-z0-9_$])${w}(?![A-Za-z0-9_$])`)),
  new RegExp(`(?<![A-Za-z0-9_$])Math(?![A-Za-z0-9_$])(?!\\s*\\.\\s*(${MATH_OK.join('|')})(?![A-Za-z0-9_$]))`),
  /Function\s*\(/, /\\u[0-9a-fA-F{]/, /\.\s*constructor(?![A-Za-z0-9_$])/, /\[\s*(['"`])constructor\1\s*\]/, /\bimport\s*\(/, /\.then\s*\(/,
];
const violations = (text: string): string[] => rules.filter((re) => re.test(text)).map((re) => re.source);

test('podcore/src, read recursively, uses no clock, timer, randomness, network, platform global, eval or alias of one', () => {
  const dir = new URL('../src/', import.meta.url);
  const files = readdirSync(dir, { recursive: true }).filter((f: string) => f.endsWith('.ts'));
  expect(files.length).toBeGreaterThan(0);
  for (const f of files) expect([f, violations(readFileSync(new URL(f, dir), 'utf8'))]).toEqual([f, []]);
});

test('the guard fails each aliasing form, and passes the three allowed Math calls, and a comment is scanned too', () => {
  const bad = [
    'const M = Math; M.random()', 'Math.random()', "Math['random']()", 'const { random } = Math', 'Math . round(1)', 'Reflect.get(Math, "random")',
    'globalThis.Date.now()', 'self.Date', 'global.process', 'window.fetch', 'console.log(1)', 'new Function ("return 1")', 'Function\n("x")', 'import("node:fs")',
    'require("fs")', '(0, eval)("1")', 'const T = setTimeout; T(f)', 'Bun.file("x")', 'p.then (f)', 'new Date()', 'Date.now()', 'const D = \\u0044ate', 'x.constructor.constructor("return 1")()', 'setImmediate(f)', "Math.min['constructor']('return Dat'+'e.now()')()", 'x["constructor"]', 'x[`constructor`]',
    'WebAssembly.compile(b)', 'navigator.userAgent', 'new MessageChannel()', 'new SharedArrayBuffer(8)',
  ];
  for (const text of bad) expect([text, violations(text).length > 0]).toEqual([text, true]);
  expect(violations('Math.floor(a) + Math.min(b, c) + Math.max(d, e)')).toEqual([]);
  expect(violations("const s = '//'; Math.random()").length).toBeGreaterThan(0);
});

test('isoOf rejects a non-integer now', () => {
  expect(() => isoOf(1.5)).toThrow();
  expect(() => isoOf(Number.NaN)).toThrow();
});
