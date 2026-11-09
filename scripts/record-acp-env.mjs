// The per-recording config home for the harness being recorded (docs/development/record-acp.md). Without it the agent reads the
// recorder's own Codex or Claude Code configuration, global instructions included, and a recording keeps whatever that told the model.
// Pure apart from creating the directory and copying one sign-in file.
import { chmodSync, copyFileSync, existsSync, mkdirSync, rmSync } from 'node:fs';
import { join } from 'node:path';

// harness -> the variable that moves the harness's config directory, the real directory's usual place under HOME, and the one file
// in it the sign-in needs. Only that file is copied in, never the directory.
const CONFIG_HOMES = {
  codex: { variable: 'CODEX_HOME', dir: '.codex', signIn: 'auth.json' },
  'claude-code': { variable: 'CLAUDE_CONFIG_DIR', dir: '.claude', signIn: '.credentials.json' },
};

/**
 * Makes <tmp>/home-<harness>/ and returns { [variable]: that path }, with the sign-in file copied in at mode 0600.
 * A missing sign-in file throws before anything is made: an unauthenticated run could overwrite a committed recording with a failure.
 * { isolate: false } is the opt-out: nothing is made and the real configuration is used (an already-set variable passes through).
 */
export function harnessHome(harness, tmp, realEnv, { isolate = true } = {}) {
  const c = CONFIG_HOMES[harness];
  if (!c) return {};
  if (!isolate) return realEnv[c.variable] ? { [c.variable]: realEnv[c.variable] } : {};
  const real = realEnv[c.variable] || (realEnv.HOME ? join(realEnv.HOME, c.dir) : null);
  const file = real && join(real, c.signIn);
  if (!file || !existsSync(file)) {
    throw new Error(`${harness}: an isolated recording needs a file sign-in, and ${c.signIn} was not found in ${real ?? `the real ${c.dir} directory`}. `
      + `Point ${c.variable} at a directory that holds ${c.signIn}`
      + `${harness === 'claude-code' ? ' (a macOS keychain sign-in cannot be carried into a fresh config directory)' : ''}, `
      + 'or pass --no-isolate (RECORD_ACP_NO_ISOLATE=1) to record with the real configuration, which leaks this machine\'s settings into the recording.');
  }
  const home = join(tmp, `home-${harness}`);
  mkdirSync(home, { recursive: true, mode: 0o700 });
  const copy = join(home, c.signIn);
  copyFileSync(file, copy);
  chmodSync(copy, 0o600);
  return { [c.variable]: home };
}

/** Removes <tmp>/home-<harness>/, the copied sign-in with it. Safe when it is not there. */
export function removeHarnessHome(harness, tmp) {
  rmSync(join(tmp, `home-${harness}`), { recursive: true, force: true });
}
