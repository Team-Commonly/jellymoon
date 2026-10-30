// What the fixtures that run land's real range scan share (check-scan-history.mjs, check-scan-checkout.mjs). The scan runs the way land runs it:
// scanRange (land-scan.mjs) with the repo's own go, the allowlist env, land's run seam, and the count land derives for itself (textCommits over
// git log -m -p -U0, run in the same env) as the expected scanned count. `make secrets` (the merged tree) still goes through make, as people run it.
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { realRun } from './land.mjs';
import { scanEnv } from './land-decide.mjs';
import { scanRange } from './land-scan.mjs';

export const root = path.resolve(import.meta.dirname, '..');
export const gitEnv = { ...process.env, GIT_CONFIG_GLOBAL: '/dev/null', GIT_AUTHOR_NAME: 't', GIT_AUTHOR_EMAIL: 't@l', GIT_COMMITTER_NAME: 't', GIT_COMMITTER_EMAIL: 't@l' };
export const makeIn = (cwd, target) => spawnSync('make', ['-s', target], { cwd, encoding: 'utf8', env: scanEnv(process.env) }).status;
export const rangeIn = (cwd, range, go = path.join(root, '.tools', 'bin', 'go')) => scanRange({ range, top: cwd, go, env: process.env, run: realRun, git: (a) => spawnSync('git', a, { cwd, encoding: 'utf8', env: scanEnv(process.env), maxBuffer: 1 << 28 }).stdout });
// A token gitleaks' default rules find, built from parts so no tracked file carries it whole.
export const token = ['ghp_', 'aB3dE5gH7jK9m', 'N1pQ3sT5vW7yZ9bD2fH4jL6'].join('');
