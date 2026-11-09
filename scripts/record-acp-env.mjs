// The per-recording config home for the harness being recorded (docs/development/record-acp.md). Without it the agent reads the
// recorder's own Codex or Claude Code configuration, global instructions included, and a recording keeps whatever that told the model.
// Pure apart from creating the directory and copying one sign-in file.
import { copyFileSync, existsSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';

// harness -> the variable that moves the harness's config directory, the real directory's usual place under HOME, and the one file
// in it the sign-in needs. Only that file is copied in, never the directory.
const CONFIG_HOMES = {
  codex: { variable: 'CODEX_HOME', dir: '.codex', signIn: 'auth.json' },
  'claude-code': { variable: 'CLAUDE_CONFIG_DIR', dir: '.claude', signIn: '.credentials.json' },
};

/** Makes <tmp>/home-<harness>/ and returns { [variable]: that path }. Copies the sign-in file when the real directory has one. */
export function harnessHome(harness, tmp, realEnv) {
  const c = CONFIG_HOMES[harness];
  if (!c) return {};
  const home = join(tmp, `home-${harness}`);
  mkdirSync(home, { recursive: true, mode: 0o700 });
  const real = realEnv[c.variable] || (realEnv.HOME ? join(realEnv.HOME, c.dir) : null);
  const file = real && join(real, c.signIn);
  if (file && existsSync(file)) copyFileSync(file, join(home, c.signIn));
  return { [c.variable]: home };
}
