import { expect, test } from 'bun:test';
import { readdirSync, readFileSync } from 'node:fs';

const BANNED = ['Date', 'setTimeout', 'setInterval', 'Math.random', 'crypto', 'fetch', 'process', 'performance', 'globalThis', 'await'];

test('podcore/src reads no clock, timer, randomness, network or platform global', () => {
  const dir = new URL('../src/', import.meta.url);
  const files = readdirSync(dir).filter((f: string) => f.endsWith('.ts'));
  expect(files.length).toBeGreaterThan(0);
  for (const f of files) {
    const text = readFileSync(new URL(f, dir), 'utf8');
    for (const word of BANNED) {
      const hit = new RegExp(`(?<![A-Za-z0-9_$])${word.replace('.', '\\.')}(?![A-Za-z0-9_$])`).exec(text);
      expect(hit === null ? null : `${f}: ${word}`).toBeNull();
    }
  }
});
