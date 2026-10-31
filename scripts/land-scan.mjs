// The range secret scan, in one place so land (scripts/land.mjs) and the fixtures (check-scan-*.mjs, through scan-fixture.mjs) run the same
// code. Make is not on this path: land runs gitleaks itself with an argv built here from constants, because a text pin on the Makefile cannot
// stop a main Makefile from changing what the scan reads while passing the pin (a built variable name, $(eval), include, SHELL, an exported
// variable). The Makefile's secrets-range stays for people; check-review-gate only checks that its recipe is the same command
// (docs/development/landing.md).
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { checkScanConfig, foldName, scanEnv, scanResult, textCommits } from './land-decide.mjs';

export const GITLEAKS_MODULE = 'github.com/zricethezav/gitleaks/v8@v8.30.1';
// `go run <module>@<version> git ...`: the argument list is the whole command. `.` is the checkout (the cwd), --exit-code 3 makes a finding
// exit 3 (go run prints "exit status 3"), --ignore-gitleaks-allow so a marker in history hides nothing. --config names land's own config
// file, so gitleaks looks for none in the checkout (no `.gitleaks.toml`, and none of the `.gitleaks.<ext>` it would load beside one), and
// --gitleaks-ignore-path an empty directory; both are outside the checkout (scanRange).
export const scanArgv = (range, config, ignore) => ['run', GITLEAKS_MODULE, 'git', `--log-opts=-m ${range}`, '--ignore-gitleaks-allow', '--exit-code', '3', '--redact', '--no-banner', '--config', config, '--gitleaks-ignore-path', ignore, '.'];
// The config land passes: gitleaks' built-in rules and nothing else. `useDefault` loads the same embedded default config that gitleaks
// reads when it finds no config at all (gitleaks v8.30.1 config/config.go extendDefault; measured: the same findings on the same history).
export const SCAN_CONFIG = '[extend]\nuseDefault = true\n';
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
  for (const [commit, where] of [[range.split('..')[0], 'main'], ...commits.map((c) => [c, `commit ${c.slice(0, 8)} in the range`])]) {
    const r = checkScanConfig(git(['ls-tree', '-r', '-z', '--full-tree', commit]), undefined, where);
    if (!r.ok) return r;
  }
  return { ok: true };
}
// The go land runs, checked on the filesystem (an ignored file counts). `<top>/.tools` and `<top>/.tools/bin` must be real directories, not
// symlinks, and go must resolve either outside the checkout (a shared toolchain cache: .tools/bin holds absolute links into it) or under
// `<top>/.tools` (scripts/tools.sh's own download: .tools/bin/go -> ../go-<version>/bin/go). A tracked .tools is refused with the trees, so no
// tracked file can place one. Returns the resolved go and its GOROOT (the directory above its bin/), and the scan runs that go with that
// GOROOT: go started through the link looks for its GOROOT two and three levels above the link, and three levels above .tools/bin/go is the
// checkout, so a tracked `pkg/tool/` at the top would make the checkout the GOROOT (measured: its go.env is read, and its toolchain run).
// How to install it. scripts/tools.sh installs into .tools only when no go of the pinned version is on PATH, or with a shared cache, so the advice
// gives it the system PATH alone (no go there on macOS); ./init.sh runs it with the operator's PATH and may install nothing.
const INSTALL = 'run `env PATH=/usr/bin:/bin:/usr/sbin:/sbin scripts/tools.sh` in the land checkout (`JM_TOOLCHAIN_CACHE=<dir> scripts/tools.sh` links a shared toolchain cache instead)';
function findGo(top) {
  const go = path.join(top, '.tools', 'bin', 'go');
  const missing = { reason: `${go} is missing or not executable; land runs gitleaks with the checkout's own go, so ${INSTALL}` };
  for (const dir of [path.join(top, '.tools'), path.join(top, '.tools', 'bin')]) {
    let st;
    try { st = fs.lstatSync(dir); } catch { return missing; }
    if (st.isSymbolicLink() || !st.isDirectory()) return { reason: `${dir} is ${st.isSymbolicLink() ? 'a symbolic link' : 'not a directory'}; land runs go only from a real .tools directory in the checkout, so remove it and ${INSTALL}` };
  }
  let real;
  try { real = fs.realpathSync.native(go); fs.accessSync(real, fs.constants.X_OK); } catch { return missing; }
  const within = (p, dir) => p.startsWith(`${dir}${path.sep}`);
  if (within(real, fs.realpathSync.native(top)) && !within(real, fs.realpathSync.native(path.join(top, '.tools')))) return { reason: `${go} resolves to ${real}, inside the checkout but outside .tools; land runs only an installed toolchain, so remove it and ${INSTALL}` };
  return { go: real, goroot: path.dirname(path.dirname(real)) };
}
// gitleaks reads `<source>/.gitleaksignore` whatever its flags say (--gitleaks-ignore-path only adds another), and a line `file:rule:line`
// there drops that finding in every commit. So one at the top of the checkout, tracked or ignored, in any case, is refused on the filesystem.
const ignoreFileIn = (top) => fs.readdirSync(top).find((n) => foldName(n) === '.gitleaksignore');
// git opens `<dir>/.gitattributes` in the checkout for every directory of a path it diffs, tracked or not, and land refuses every one (owner
// ruling; checkScanConfig refuses them in the trees). So the checkout is listed on the filesystem, every entry at every depth, ignored ones
// included, names compared folded, and the first .gitattributes found is returned, relative to the top. Two top-level entries are skipped:
// `.git` (git's own) and `.tools`, which no checked tree may carry a path under, so git diffs nothing there and opens no attributes file in
// it (and an installed Go ships one in its source tree). Symbolic links are not followed.
const attributesIn = (top, rel = '') => {
  for (const e of fs.readdirSync(path.join(top, rel), { withFileTypes: true })) {
    const name = foldName(e.name);
    const at = rel ? `${rel}/${e.name}` : e.name;
    if (name === '.gitattributes') return at;
    const found = e.isDirectory() && !(rel === '' && (name === '.git' || name === '.tools')) && attributesIn(top, at);
    if (found) return found;
  }
  return undefined;
};
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
  const ignored = ignoreFileIn(top);
  if (ignored) return { result: 'refused', reason: `${path.join(top, ignored)} exists; gitleaks would drop the findings it lists, so remove it` };
  const attributes = attributesIn(top);
  if (attributes) return { result: 'refused', reason: `${path.join(top, attributes)} exists; git reads a .gitattributes while it diffs, and land refuses every one in the checkout, tracked or not, so remove it` };
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'jm-scan-')); // the operator's TMPDIR, outside the checkout
  try {
    const [config, ignore] = [path.join(dir, 'gitleaks.toml'), path.join(dir, 'empty')];
    fs.writeFileSync(config, SCAN_CONFIG);
    fs.mkdirSync(ignore);
    const r = run(tools.go, scanArgv(range, config, ignore), { cwd: top, env: { ...scanEnv(env), GOROOT: tools.goroot } });
    return { result: scanResult({ status: r.status, stderr: r.stderr, expected }), expected, revlist: all.length, stderr: r.stderr };
  } finally { fs.rmSync(dir, { recursive: true, force: true }); }
}
