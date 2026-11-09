// harnessHome gives each recording its own config directory and carries over the sign-in file only.
import { expect, test } from 'bun:test';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { harnessHome } from './record-acp-env.mjs';

function setup(dir, files) {
  const fakeHome = mkdtempSync(join(tmpdir(), 'jmrec-env-home-'));
  const tmp = mkdtempSync(join(tmpdir(), 'jmrec-env-tmp-'));
  mkdirSync(join(fakeHome, dir));
  for (const [name, text] of Object.entries(files)) writeFileSync(join(fakeHome, dir, name), text);
  return { fakeHome, tmp };
}

test('codex: a fresh CODEX_HOME holds the sign-in file and nothing else', () => {
  const { fakeHome, tmp } = setup('.codex', { 'auth.json': 'signed-in', 'AGENTS.md': 'global rules', 'config.toml': 'x = 1' });
  const env = harnessHome('codex', tmp, { HOME: fakeHome });
  expect(Object.keys(env)).toEqual(['CODEX_HOME']);
  expect(env.CODEX_HOME.startsWith(tmp)).toBe(true);
  expect(readFileSync(join(env.CODEX_HOME, 'auth.json'), 'utf8')).toBe('signed-in');
  expect(existsSync(join(env.CODEX_HOME, 'AGENTS.md'))).toBe(false);
  expect(existsSync(join(env.CODEX_HOME, 'config.toml'))).toBe(false);
});

test('claude-code: a fresh CLAUDE_CONFIG_DIR, and an existing CODEX_HOME or CLAUDE_CONFIG_DIR names the source', () => {
  const { fakeHome, tmp } = setup('elsewhere', { '.credentials.json': 'c', 'CLAUDE.md': 'global rules' });
  const env = harnessHome('claude-code', tmp, { HOME: fakeHome, CLAUDE_CONFIG_DIR: join(fakeHome, 'elsewhere') });
  expect(env.CLAUDE_CONFIG_DIR.startsWith(tmp)).toBe(true);
  expect(existsSync(join(env.CLAUDE_CONFIG_DIR, '.credentials.json'))).toBe(true);
  expect(existsSync(join(env.CLAUDE_CONFIG_DIR, 'CLAUDE.md'))).toBe(false);
});

test('no sign-in file is fine, and an unknown harness gets nothing', () => {
  const { fakeHome, tmp } = setup('.codex', {});
  const env = harnessHome('codex', tmp, { HOME: fakeHome });
  expect(existsSync(join(env.CODEX_HOME, 'auth.json'))).toBe(false);
  expect(harnessHome('other', tmp, { HOME: fakeHome })).toEqual({});
});
