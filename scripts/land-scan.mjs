// The range secret scan, in one place so land (scripts/land.mjs) and the fixtures (check-review-gate.mjs) run the same code. Make is not
// on this path: land runs gitleaks itself with an argv built here from constants, because a text pin on the Makefile cannot stop a main
// Makefile from changing what the scan reads while passing the pin (a built variable name, $(eval), include, SHELL, an exported variable).
// The Makefile's secrets-range stays for people; check-review-gate only checks that its recipe is the same command (docs/development/landing.md).
import fs from 'node:fs';
import path from 'node:path';
import { scanEnv, scanResult, textCommits } from './land-decide.mjs';

export const GITLEAKS_MODULE = 'github.com/zricethezav/gitleaks/v8@v8.30.1';
// `go run <module>@<version> git ...`: the argument list is the whole command. `.` is the checkout (the cwd), --exit-code 3 makes a finding
// exit 3 (go run prints "exit status 3"), --ignore-gitleaks-allow so a marker in history hides nothing.
export const scanArgv = (range) => ['run', GITLEAKS_MODULE, 'git', `--log-opts=-m ${range}`, '--ignore-gitleaks-allow', '--exit-code', '3', '--redact', '--no-banner', '.'];
// Every path the range touches, listed by git itself (`git log -m --no-renames --name-status -z`, each commit announced by \x01<sha>): added,
// modified and deleted paths, so both sides of a rename or copy appear, and every parent of a merge (-m). It reads NUL-separated records by
// position (a status, then its path; a rename or copy status would carry two), so a path that looks like a header is never taken for one.
const NAME_STATUS = ['log', '-m', '--no-renames', '--format=%x01%H', '--name-status', '-z'];
export function touchedPaths(listing) {
  const found = [];
  let sha = '';
  let owe = 0; // paths still owed by the last status
  for (const tok of String(listing).split('\0')) {
    if (owe) { found.push({ sha, path: tok }); owe -= 1; continue; }
    const head = /^\u0001([0-9a-f]{40})$/.exec(tok);
    const status = tok.replace(/^\n/, '');
    if (head) sha = head[1];
    else if (status) owe = /^[RC]/.test(status) ? 2 : 1;
  }
  return found;
}
// gitleaks' diff parser (go-gitdiff parseQuotedName) ends a quoted file name at the first quote that does not follow a backslash, so a path
// whose last character is a backslash (git quotes it "b\\") fails to parse. The parser then drops that file AND the "commit" preamble before
// it, and the files after it are attributed to the previous commit. The count cannot see this (the numbers can agree over different
// commits), so land refuses any range that touches such a path.
export const backslashPaths = (listing) => touchedPaths(listing).filter((p) => p.path.endsWith('\\'));
// Reads the range the way gitleaks will and runs the scan. git(args) returns stdout in the scan's env; run(cmd, args, opts) is { status,
// stdout, stderr }. result: 'clean' | 'hit' | 'failure' (scanResult) or 'refused' with a reason (the range or the checkout cannot be scanned).
export function scanRange({ range, top, git, run, env, go = path.join(top, '.tools', 'bin', 'go') }) {
  const bad = backslashPaths(git(NAME_STATUS.concat(range)))[0];
  if (bad) return { result: 'refused', reason: `commit ${bad.sha.slice(0, 8)} touches the path ${JSON.stringify(bad.path)}, which ends in a backslash; gitleaks cannot parse that file name and would skip the files after it, so land refuses` };
  const all = git(['rev-list', range]).split('\n').filter(Boolean);
  const { expected } = textCommits(git(['log', '-m', '-p', '-U0', '--format=%x01%H', range]), all);
  try { fs.accessSync(go, fs.constants.X_OK); } catch { return { result: 'refused', reason: `${go} is missing or not executable; run ./init.sh once in the land checkout so land can run gitleaks` }; }
  const r = run(go, scanArgv(range), { cwd: top, env: scanEnv(env) });
  return { result: scanResult({ status: r.status, stderr: r.stderr, expected }), expected, revlist: all.length, stderr: r.stderr };
}
