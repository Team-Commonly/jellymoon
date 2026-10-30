// The range secret scan, in one place so land (scripts/land.mjs) and the fixtures (check-scan-*.mjs, through scan-fixture.mjs) run the same
// code. Make is not on this path: land runs gitleaks itself with an argv built here from constants, because a text pin on the Makefile cannot
// stop a main Makefile from changing what the scan reads while passing the pin (a built variable name, $(eval), include, SHELL, an exported
// variable). The Makefile's secrets-range stays for people; check-review-gate only checks that its recipe is the same command
// (docs/development/landing.md).
import fs from 'node:fs';
import path from 'node:path';
import { checkScanConfig, scanEnv, scanResult, textCommits } from './land-decide.mjs';

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
// The trees read before the scan, by git in the scan's env: main's tip (the left end of the range) and every commit of the range, each listed
// whole (`--full-tree`, so the directory land runs from cannot narrow it) and NUL-separated. checkScanConfig refuses a tree that carries a file
// that would steer this scan or a later one.
function checkTrees(range, commits, git) {
  const blob = (id) => git(['cat-file', 'blob', id]);
  for (const [commit, where] of [[range.split('..')[0], 'main'], ...commits.map((c) => [c, `commit ${c.slice(0, 8)} in the range`])]) {
    const r = checkScanConfig(git(['ls-tree', '-r', '-z', '--full-tree', commit]), blob, undefined, where);
    if (!r.ok) return r;
  }
  return { ok: true };
}
// The go land runs, checked on the filesystem (an ignored file counts). `<top>/.tools` and `<top>/.tools/bin` must be real directories, not
// symlinks, and go must resolve either outside the checkout (a shared toolchain cache: .tools/bin holds absolute links into it) or under
// `<top>/.tools` (./init.sh's own download: .tools/bin/go -> ../go-<version>/bin/go). A tracked .tools is refused with the trees, so no
// tracked file can place one. Returns the resolved go and its GOROOT (the directory above its bin/), and the scan runs that go with that
// GOROOT: go started through the link looks for its GOROOT two and three levels above the link, and three levels above .tools/bin/go is the
// checkout, so a tracked `pkg/tool/` at the top would make the checkout the GOROOT (measured: its go.env is read, and its toolchain run).
function findGo(top) {
  const go = path.join(top, '.tools', 'bin', 'go');
  const missing = { reason: `${go} is missing or not executable; run ./init.sh once in the land checkout so land can run gitleaks` };
  for (const dir of [path.join(top, '.tools'), path.join(top, '.tools', 'bin')]) {
    let st;
    try { st = fs.lstatSync(dir); } catch { return missing; }
    if (st.isSymbolicLink() || !st.isDirectory()) return { reason: `${dir} is ${st.isSymbolicLink() ? 'a symbolic link' : 'not a directory'}; land runs go only from a real .tools directory in the checkout, so remove it and run ./init.sh` };
  }
  let real;
  try { real = fs.realpathSync.native(go); fs.accessSync(real, fs.constants.X_OK); } catch { return missing; }
  const within = (p, dir) => p.startsWith(`${dir}${path.sep}`);
  if (within(real, fs.realpathSync.native(top)) && !within(real, fs.realpathSync.native(path.join(top, '.tools')))) return { reason: `${go} resolves to ${real}, inside the checkout but outside .tools; land runs only an installed toolchain, so remove it and run ./init.sh` };
  return { go: real, goroot: path.dirname(path.dirname(real)) };
}
// Reads the range the way gitleaks will and runs the scan. git(args) returns stdout in the scan's env; run(cmd, args, opts) is { status,
// stdout, stderr }. result: 'clean' | 'hit' | 'failure' (scanResult) or 'refused' with a reason (the range or the checkout cannot be scanned).
export function scanRange({ range, top, git, run, env }) {
  const all = git(['rev-list', range]).split('\n').filter(Boolean);
  const trees = checkTrees(range, all, git);
  if (!trees.ok) return { result: 'refused', reason: trees.reason };
  const bad = backslashPaths(git(NAME_STATUS.concat(range)))[0];
  if (bad) return { result: 'refused', reason: `commit ${bad.sha.slice(0, 8)} touches the path ${JSON.stringify(bad.path)}, which ends in a backslash; gitleaks cannot parse that file name and would skip the files after it, so land refuses` };
  const { expected } = textCommits(git(['log', '-m', '-p', '-U0', '--format=%x01%H', range]), all);
  const tools = findGo(top);
  if (!tools.go) return { result: 'refused', reason: tools.reason };
  const r = run(tools.go, scanArgv(range), { cwd: top, env: { ...scanEnv(env), GOROOT: tools.goroot } });
  return { result: scanResult({ status: r.status, stderr: r.stderr, expected }), expected, revlist: all.length, stderr: r.stderr };
}
