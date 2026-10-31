// What the fixtures that run land's real range scan share (check-scan-history.mjs, check-scan-checkout.mjs). The scan runs the way land runs it:
// scanRange (land-scan.mjs) with the fixture checkout's own .tools, the allowlist env, land's run seam, and the count land derives for itself
// (textCommits over git log -m -p -U0, run in the same env) as the expected scanned count. `make secrets` (the merged tree) still goes through
// make, as people run it.
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { realRun } from './land.mjs';
import { scanEnv } from './land-decide.mjs';
import { scanRange } from './land-scan.mjs';

export const root = path.resolve(import.meta.dirname, '..');
export const gitEnv = { ...process.env, GIT_CONFIG_GLOBAL: '/dev/null', GIT_AUTHOR_NAME: 't', GIT_AUTHOR_EMAIL: 't@l', GIT_COMMITTER_NAME: 't', GIT_COMMITTER_EMAIL: 't@l' };
export const makeIn = (cwd, target) => spawnSync('make', ['-s', target], { cwd, encoding: 'utf8', env: scanEnv(process.env) }).status;
// The go the checks run fixtures with, resolved to its real path: `<top>/.tools/bin/go` when it exists (scripts/tools.sh installed or linked
// it), else the first go on PATH (CI has no .tools: setup-go puts go on PATH, and tools.sh installs nothing when a matching go is there).
// Neither fails, naming both. Checks only: land itself runs the checkout's own .tools/bin/go and nothing else (findGo, land-scan.mjs).
export const fixtureGo = ({ top = root, PATH = process.env.PATH ?? '' } = {}) => {
  const local = path.join(top, '.tools', 'bin', 'go');
  const runs = (p) => { try { fs.accessSync(p, fs.constants.X_OK); return fs.statSync(p).isFile(); } catch { return false; } };
  const go = fs.existsSync(local) ? local : PATH.split(path.delimiter).filter(Boolean).map((d) => path.resolve(d, 'go')).find(runs);
  if (!go) throw new Error(`no go to run the fixtures with: ${local} does not exist and there is no go on PATH`);
  return fs.realpathSync.native(go);
};
// The two layouts scripts/tools.sh makes, built in a fixture checkout from the go above. With a shared toolchain cache:
// real .tools and .tools/bin directories, and .tools/bin/go an absolute link to a go outside the checkout.
export const cacheTools = (top) => {
  fs.mkdirSync(path.join(top, '.tools', 'bin'), { recursive: true });
  fs.symlinkSync(fixtureGo(), path.join(top, '.tools', 'bin', 'go'));
};
// Without one: the toolchain in .tools/go-<version>/ and .tools/bin/go -> ../go-<version>/bin/go, so go resolves under the checkout's .tools.
// The fixture's toolchain directory links each entry of the installed GOROOT except bin/, whose go is a copy (a clone where APFS allows).
export const ownTools = (top) => {
  const goroot = path.dirname(path.dirname(fixtureGo()));
  const mine = path.join(top, '.tools', path.basename(goroot));
  fs.mkdirSync(path.join(mine, 'bin'), { recursive: true });
  fs.mkdirSync(path.join(top, '.tools', 'bin'));
  for (const e of fs.readdirSync(goroot)) if (e !== 'bin') fs.symlinkSync(path.join(goroot, e), path.join(mine, e));
  fs.copyFileSync(fixtureGo(), path.join(mine, 'bin', 'go'), fs.constants.COPYFILE_FICLONE);
  fs.chmodSync(path.join(mine, 'bin', 'go'), 0o755);
  fs.symlinkSync(path.join('..', path.basename(goroot), 'bin', 'go'), path.join(top, '.tools', 'bin', 'go'));
};
const has = (p) => { try { fs.lstatSync(p); return true; } catch { return false; } };
// Land's own scan in a fixture checkout. A checkout without a .tools gets the cache layout first; one that brings its own keeps it, and that is
// the .tools the scan runs (land never takes a go from anywhere else).
export const rangeIn = (cwd, range) => {
  if (!has(path.join(cwd, '.tools'))) cacheTools(cwd);
  return scanRange({ range, top: cwd, env: process.env, run: realRun, git: (a) => spawnSync('git', a, { cwd, encoding: 'utf8', env: scanEnv(process.env), maxBuffer: 1 << 28 }).stdout });
};
// A token gitleaks' default rules find, built from parts so no tracked file carries it whole.
export const token = ['ghp_', 'aB3dE5gH7jK9m', 'N1pQ3sT5vW7yZ9bD2fH4jL6'].join('');
