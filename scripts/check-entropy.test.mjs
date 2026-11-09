// A LOC cap grows only with a new note under docs/entropy/caps/ in the same diff (ADR 0001). Each case builds a throwaway
// git repo whose first commit is the base, edits entropy.json after it, and runs check-entropy.mjs inside that repo.
import { expect, test } from 'bun:test';
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const script = join(import.meta.dir, 'check-entropy.mjs');
const cfg = (max) => JSON.stringify({
  maxFileLines: 400, maxFunctionLines: 60, generated: [], markerScanSkip: [],
  locBudget: { core: { paths: ['src/'], max } }, goImports: {}, exceptions: { files: {}, functions: {} },
}, null, 2);

function git(dir, ...args) {
  const r = Bun.spawnSync(['git', '-C', dir, '-c', 'user.name=t', '-c', 'user.email=t@example.com', ...args]);
  if (r.exitCode !== 0) throw new Error(`git ${args.join(' ')}: ${r.stderr}`);
}

function repo() {
  const dir = mkdtempSync(join(tmpdir(), 'jmentropy-test-'));
  git(dir, 'init', '-q', '-b', 'main');
  mkdirSync(join(dir, 'src'));
  writeFileSync(join(dir, 'src', 'a.mjs'), 'export const a = 1;\n');
  writeFileSync(join(dir, 'entropy.json'), cfg(100));
  git(dir, 'add', '.');
  git(dir, 'commit', '-q', '-m', 'base');
  git(dir, 'checkout', '-q', '-b', 'topic');
  return dir;
}

function run(dir) {
  const r = Bun.spawnSync([process.execPath, script], { cwd: dir, env: { ...process.env, JM_ENTROPY_BASE: 'main' } });
  return { code: r.exitCode, out: `${r.stdout}${r.stderr}` };
}

test('a raised cap with no note fails', () => {
  const dir = repo();
  writeFileSync(join(dir, 'entropy.json'), cfg(150));
  const r = run(dir);
  expect(r.code).toBe(1);
  expect(r.out).toContain('docs/entropy/caps/');
});

test('a raised cap with a new note passes, and the total is printed', () => {
  const dir = repo();
  writeFileSync(join(dir, 'entropy.json'), cfg(150));
  mkdirSync(join(dir, 'docs', 'entropy', 'caps'), { recursive: true });
  writeFileSync(join(dir, 'docs', 'entropy', 'caps', '2026-11-09-core-100-to-150.md'), 'core 100 to 150\n');
  const r = run(dir);
  expect(r.out).toContain('caps total: 150');
  expect(r.code).toBe(0);
});

test('an unchanged cap needs no note', () => {
  const r = run(repo());
  expect(r.code).toBe(0);
});

test('an edited old note does not count as a new one', () => {
  const dir = repo();
  mkdirSync(join(dir, 'docs', 'entropy', 'caps'), { recursive: true });
  writeFileSync(join(dir, 'docs', 'entropy', 'caps', 'old.md'), 'old\n');
  git(dir, 'add', '.');
  git(dir, 'commit', '-q', '-m', 'old note');
  git(dir, 'checkout', '-q', 'main');
  git(dir, 'merge', '-q', 'topic');
  git(dir, 'checkout', '-q', '-b', 'topic2');
  writeFileSync(join(dir, 'entropy.json'), cfg(150));
  writeFileSync(join(dir, 'docs', 'entropy', 'caps', 'old.md'), 'old, edited\n');
  expect(run(dir).code).toBe(1);
});
