// harnessHome gives each recording its own config directory and carries over the sign-in file only.
import { expect, test } from 'bun:test';
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { harnessHome, removeHarnessHome } from './record-acp-env.mjs';

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

test('a missing sign-in file fails before anything is made, naming the harness', () => {
  const { fakeHome, tmp } = setup('.codex', {});
  expect(() => harnessHome('codex', tmp, { HOME: fakeHome })).toThrow(/codex.*auth\.json/s);
  expect(existsSync(join(tmp, 'home-codex'))).toBe(false);
  const claude = setup('.claude', { 'CLAUDE.md': 'x' });
  expect(() => harnessHome('claude-code', claude.tmp, { HOME: claude.fakeHome })).toThrow(/claude-code.*CLAUDE_CONFIG_DIR.*--no-isolate/s);
});

test('the copied sign-in file is 0600 whatever the source mode was', () => {
  const { fakeHome, tmp } = setup('.codex', { 'auth.json': 'signed-in' });
  chmodSync(join(fakeHome, '.codex', 'auth.json'), 0o644);
  const env = harnessHome('codex', tmp, { HOME: fakeHome });
  expect(statSync(join(env.CODEX_HOME, 'auth.json')).mode & 0o777).toBe(0o600);
});

test('the opt-out uses the real config: no new directory, an already-set variable passes through', () => {
  const { fakeHome, tmp } = setup('.codex', {});
  expect(harnessHome('codex', tmp, { HOME: fakeHome }, { isolate: false })).toEqual({});
  expect(existsSync(join(tmp, 'home-codex'))).toBe(false);
  expect(harnessHome('codex', tmp, { HOME: fakeHome, CODEX_HOME: '/real/codex' }, { isolate: false })).toEqual({ CODEX_HOME: '/real/codex' });
});

test('removeHarnessHome deletes the copied sign-in folder and is safe to call twice', () => {
  const { fakeHome, tmp } = setup('.codex', { 'auth.json': 'signed-in' });
  const env = harnessHome('codex', tmp, { HOME: fakeHome });
  expect(existsSync(join(env.CODEX_HOME, 'auth.json'))).toBe(true);
  removeHarnessHome('codex', tmp);
  expect(existsSync(env.CODEX_HOME)).toBe(false);
  expect(existsSync(tmp)).toBe(true);
  removeHarnessHome('codex', tmp);
});

test('an unknown harness gets nothing', () => {
  const { fakeHome, tmp } = setup('.codex', {});
  expect(harnessHome('other', tmp, { HOME: fakeHome })).toEqual({});
});
