#!/usr/bin/env bun
// End-to-end land cases: main() from land.mjs runs against a scripted fake of git, gh, gitleaks (go run), the pod and the preflight child. The fake
// answers by argv pattern and records every call, so each case asserts the refusal AND the argv that proves which guard fired.
import { EventEmitter } from 'node:events';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { main, realRun } from './land.mjs';

const [MAIN, HEAD, TREE, MERGED, OTHER] = ['1', '2', '3', '4', '5'].map((c) => c.repeat(40));
const REPO = 'Team-Commonly/jellymoon';
const HDR = 'pr\thead\tverdict\tby\twhen\tnote\n';
const rowOf = (verdict, by = 'opus-verifier', head = HEAD) => `5\t${head}\t${verdict}\t${by}\t2026-10-29T10:00:00Z\tn\n`;
const NOTE = { userId: { _id: 'id-orion', username: 'orion' }, content: `jellymoon#5 ${HEAD}\nok\nGATE: PASS`, createdAt: '2026-10-29T10:01:00Z' };
const fail = new Error('scripted failure');
// What a clean gitleaks run prints to stderr (it colours its log even into a pipe): the scanned count and "no leaks found".
const cleanScan = (n) => `\x1b[90m10:11AM\x1b[0m \x1b[32mINF\x1b[0m \x1b[1m${n} commits scanned.\x1b[0m\n10:11AM INF scanned ~6 bytes (6 bytes) in 37.9ms\n\x1b[90m10:11AM\x1b[0m \x1b[32mINF\x1b[0m \x1b[1mno leaks found\x1b[0m\n`;

// The scripted range scan: a throw is a failed scan, a return is a zero exit with that stderr. runOf turns either into { status, stdout, stderr }.
const scanCall = (over) => {
  const bad = { hit: [2, 'WRN leaks found: 1\nexit status 3\n'], killed: [null, 'exit status 3\n'], killedclean: [null, cleanScan(3)], fail: [2, 'go: no network\nexit status 1\n'] }[over.scan];
  if (bad) throw Object.assign(new Error('scan failed'), { status: bad[0], stderr: bad[1] });
  return { status: 0, stderr: over.scanErr ?? cleanScan(over.scanned ?? (over.count ?? 3) - (over.bare ?? 0)) };
};
const runOf = (f) => { try { const v = f(); return typeof v === 'object' ? { stdout: '', ...v } : { status: 0, stdout: v, stderr: '' }; } catch (e) { return { status: typeof e.status === 'number' ? e.status : null, stdout: '', stderr: String(e.stderr || '') }; } };

// The commits of <main tip>..<head>; any other range holds one.
const commitsOf = (range, n = 3) => Array.from({ length: range.endsWith(`..${HEAD}`) && !range.startsWith(HEAD) ? n : 1 }, (_, i) => `c${i}`.padEnd(40, '0'));
const ALL = [];
// What the scan's config and ignore paths held while it ran (land removes them after): read from the argv land passed.
const seenOf = (a) => { const [config, ignore] = ['--config', '--gitleaks-ignore-path'].map((f) => a[a.indexOf(f) + 1]); return { config, ignore, text: fs.readFileSync(config, 'utf8'), entries: fs.readdirSync(ignore) }; };
const seedGo = (top) => { fs.mkdirSync(path.join(top, '.tools', 'bin'), { recursive: true }); fs.writeFileSync(path.join(top, '.tools', 'bin', 'go'), '', { mode: 0o755 }); }; // the checkout's own go
async function land(over = {}) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'land-e2e-'));
  const [top, common, cwd] = ['top', 'common', 'cwd'].map((d) => { fs.mkdirSync(path.join(dir, d)); return path.join(dir, d); });
  const ledger = path.join(dir, 'ledger.tsv');
  fs.writeFileSync(ledger, over.ledger ?? HDR + rowOf('PASS'));
  fs.writeFileSync(path.join(dir, 'token'), 'jmu_FAKE-token-abc123\n');
  (over.topFiles || []).forEach((f) => fs.writeFileSync(path.join(top, f), '')); if (!over.noGo) seedGo(top);
  (over.cwdFiles || []).forEach((f) => fs.writeFileSync(path.join(cwd, f), ''));
  const w = { calls: [], sleeps: [], fetches: [], pod: [], spawns: [], views: 0, ledger, lockAtMerge: null, kills: [], seen: [], top, go: over.noGo ? null : fs.realpathSync.native(path.join(top, '.tools', 'bin', 'go')) }; // go: what land runs, the checkout's go resolved
  const base = { url: `https://github.com/${REPO}/pull/5`, number: 5, state: 'OPEN', baseRefName: 'main', headRefName: 'feat', headRefOid: HEAD, createdAt: '2026-10-29T09:00:00Z', isCrossRepository: false, isDraft: false, author: { login: 'alice' }, mergeCommit: { oid: MERGED }, ...over.pr };
  const wt = /^git -C \S+ /;
  const defaults = [
    [/^git rev-parse --git-common-dir/, common], [/^git rev-parse --show-toplevel/, top], [/^git rev-parse HEAD$/, over.checkoutHead ?? MAIN],
    [/^git remote -v/, over.remote ?? `origin\thttps://github.com/${REPO}.git (fetch)\norigin\thttps://github.com/${REPO}.git (push)`],
    [/^git config --get-regexp/, () => { throw fail; }], [/status --porcelain/, ''], [/^git ls-files -v/, 'H README.md'], [/^git diff --quiet HEAD/, ''],
    [/^git fetch --quiet origin/, (a, s) => { s.fetched = a.at(-1); s.fetches.push(s.fetched); return ''; }],
    [/^git rev-parse FETCH_HEAD/, (a, s) => (s.fetched === 'refs/heads/main' ? (over.mainTips?.[s.fetches.filter((f) => f === s.fetched).length - 1] ?? MAIN) : s.fetched.startsWith('pull/') ? (over.pullHead ?? HEAD) : MERGED)],
    [/^git rev-parse \S+\^\{tree\}/, over.mergedTree ?? TREE],
    [/^gh pr view 5 .*--json mergeCommit/, JSON.stringify({ mergeCommit: base.mergeCommit })],
    [/^gh pr view/, (a, s) => { s.views += 1; const pr = s.views > 1 ? { ...base, ...over.fresh } : base; return JSON.stringify(Object.fromEntries(a[a.indexOf('--json') + 1].split(',').filter((k) => !(over.omit || []).includes(k)).map((k) => [k, pr[k]]))); }],
    [/^gh pr merge/, (a, s) => { s.lockAtMerge = fs.existsSync(path.join(common, 'jm-land.lock')); return ''; }],
    [/^gh api .*collaborators/, (a) => ['{"login":"alice","push":true}', ...(a.includes('--paginate') ? over.page2 || [] : [])].join('\n')],
    [/^gh api .*check-runs/, over.runs ?? '{"total":1,"runs":[{"name":"ci","conclusion":"success"}]}'],
    [/^git merge-tree --write-tree/, `${TREE}\n`],
    [/^git rev-list /, (a) => commitsOf(a.at(-1), over.count).join('\n')],
    [/^git log -m -p -U0 --format=%x01%H /, (a) => commitsOf(a.at(-1), over.count).map((c, i) => `\u0001${c}\n\n${i < (over.bare ?? 0) ? '' : 'diff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -0,0 +1 @@\n+x\n'}`).join('')], // the first over.bare commits add no text
    [/\/\.tools\/bin\/go run /, (a, s) => { s.seen.push(seenOf(a)); return scanCall(over); }], [/^git log -m --no-renames --format=%x01%H --name-status -z /, over.names ?? ''],
    [/^git cat-file -e/, ''], [/^git ls-tree -r -z /, over.tree ?? ''],
    [/^git (-c \S+ )?worktree|^chmod/, ''], [new RegExp(`${wt.source}.*merge --no-ff`), ''], [new RegExp(`${wt.source}.*rev-parse HEAD\\^\\{tree\\}`), over.wtTree ?? TREE],
  ];
  const exec = (cmd, args, opts) => {
    w.calls.push({ cmd, args, opts });
    const j = [cmd, ...args].join(' ');
    const hit = [...(over.on || []), ...defaults].find(([re]) => re.test(j));
    if (!hit) throw new Error(`unscripted call: ${j}`);
    const v = typeof hit[1] === 'function' ? hit[1]([cmd, ...args], w) : hit[1];
    return v;
  };
  const run = (cmd, args, opts) => runOf(() => exec(cmd, args, opts)); // the scan's seam: status, stdout and stderr, never a throw
  const spawn = (cmd, args, opts) => {
    w.spawns.push({ cmd, args, opts });
    const c = Object.assign(new EventEmitter(), { stdout: new EventEmitter(), stderr: new EventEmitter(), pid: 2000000001 });
    setImmediate(() => { over.onSpawn?.(w); if (over.output) c.stdout.emit('data', Buffer.from(over.output)); if (over.child === 'hang') return; if (over.child === 'exit-only') c.emit('exit', 0); else c.emit('close', over.initCode ?? 0); });
    return c;
  };
  const fetch = async (url, o) => { w.pod.push({ url, o }); if (over.plan) return over.plan(w.pod.length, url); return { ok: true, status: 200, json: async () => (url.includes('before=') ? [] : (over.msgs ?? [NOTE])) }; };
  const env = { JM_REPO: REPO, JM_TRUSTED_LOGINS: 'alice', JM_LEDGER: ledger, JM_COMMONLY_API: 'https://api.commonly.me', JM_COMMONLY_TOKEN_FILE: path.join(dir, 'token'), JM_VERDICT_POD: 'pod1', JM_VERDICT_REVIEWERS: 'id-orion:Orion', ...over.env };
  const kill = (pid, sig) => { w.kills.push({ pid, sig, t: Date.now() }); };
  const [log, err] = [console.log, console.error];
  const said = [];
  console.log = (m) => said.push(m); console.error = (m) => said.push(m);
  let error = null; const listeners = process.listenerCount('exit');
  try { await main({ exec, run, fetch, spawn, sleep: async (ms) => { w.sleeps.push(ms); over.onSleep?.(w); if (w.sleeps.length > 200) throw new Error('runaway wait'); }, kill, graceMs: over.graceMs ?? 20, limitMs: over.limitMs, env, argv: over.argv ?? ['5'], nodeVersion: over.node ?? '22.0.0', cwd, now: () => over.now ?? Date.now() }); } catch (e) { error = e.message; } finally { console.log = log; console.error = err; }
  const merge = w.calls.find((c) => c.cmd === 'gh' && c.args[1] === 'merge');
  w.doneAt = Date.now(); w.exitListenersLeft = process.listenerCount('exit') - listeners;
  const res = { error, w, merge, said: said.join('\n'), ledgerNow: fs.readFileSync(ledger, 'utf8'), ledgerWas: over.ledger ?? HDR + rowOf('PASS'), lock: fs.existsSync(path.join(common, 'jm-land.lock')) };
  fs.rmSync(dir, { recursive: true, force: true });
  ALL.push(...w.calls);
  return res;
}
// Each case runs land once; `no` is: refused with this reason, nothing merged.
const no = (r, re) => !r.merge && re.test(r.error ?? '');
const scans = (r) => r.w.calls.filter((c) => c.cmd === r.w.go); // land's gitleaks runs: the checkout's own go, by its resolved absolute path
const argv = (r, cmd, sub) => r.w.calls.filter((c) => c.cmd === cmd && [cmd, ...c.args].join(' ').includes(sub)).map((c) => c.args.join(' '));
const fetched = (r) => r.w.calls.some((c) => c.args[0] === 'fetch');
const cases = [];
const t = async (name, over, f) => { cases.push([name, !!f(await land(over)), true]); };
const mergeArgs = `pr merge 5 --repo ${REPO} --merge --match-head-commit ${HEAD}`;
const dirty = [[/status --porcelain/, ' M x']];
const PO = ['5', '--preflight-only'];
await t('happy path: lands with --merge and --match-head-commit <head>; the ledger gets LANDED', {}, (r) => r.merge?.args.join(' ') === mergeArgs && r.ledgerNow.includes('\tLANDED\t') && r.lock === false && r.w.lockAtMerge === true);
await t('happy path: pr view asks for isCrossRepository; the collaborators call is paginated and unfiltered', {}, (r) => argv(r, 'gh', 'pr view')[0]?.includes('isCrossRepository') && argv(r, 'gh', 'collaborators')[0]?.includes('--paginate') && !argv(r, 'gh', 'collaborators')[0]?.includes('permission='));
await t('every main fetch names refs/heads/main; the merge disables hooks; untracked files are shown in full', {}, (r) => r.w.fetches.filter((f) => !f.startsWith('pull/') && f !== MERGED).every((f) => f === 'refs/heads/main') && argv(r, 'git', 'merge --no-ff')[0]?.includes('core.hooksPath=/dev/null') && argv(r, 'git', 'status')[0]?.includes('status.showUntrackedFiles=all'));
await t('the range scan is the checkout\'s own .tools/bin/go, resolved to its real absolute path, run with exactly this argv, cwd the checkout, no shell, and no make', {}, (r) => scans(r).length === 2 && scans(r).every((c) => JSON.stringify(c.args) === JSON.stringify(['run', 'github.com/zricethezav/gitleaks/v8@v8.30.1', 'git', `--log-opts=-m ${MAIN}..${HEAD}`, '--ignore-gitleaks-allow', '--exit-code', '3', '--redact', '--no-banner', '--config', c.args[10], '--gitleaks-ignore-path', c.args[12], '.']) && c.opts.cwd === r.w.top && !('shell' in c.opts)) && !r.w.calls.some((c) => c.cmd === 'make' || c.cmd === 'go' || c.cmd === 'sh' || c.cmd === 'bash'));
const outside = (p, top) => path.isAbsolute(p) && path.relative(top, p).startsWith('..');
await t('the scan\'s --config is a file land wrote outside the checkout, a fresh one per scan, holding only gitleaks\' built-in rules, removed after', {}, (r) => r.w.seen.length === 2 && r.w.seen[0].config !== r.w.seen[1].config && r.w.seen.every((s) => outside(s.config, r.w.top) && s.text === '[extend]\nuseDefault = true\n' && !fs.existsSync(path.dirname(s.config))));
await t('the scan\'s --gitleaks-ignore-path is an empty directory outside the checkout, removed after', {}, (r) => r.w.seen.length === 2 && r.w.seen.every((s) => outside(s.ignore, r.w.top) && s.entries.length === 0 && !fs.existsSync(s.ignore)));
await t('a .gitleaksignore at the top of the checkout (ignored, in any case) is refused before the scan, naming it', { topFiles: ['.GitleaksIgnore'] }, (r) => no(r, /\/\.GitleaksIgnore exists; gitleaks would drop the findings it lists/) && !scans(r).length && r.w.pod.length === 0);
// Every name the allowlist keeps, set to a recognisable value, plus operator variables that must not pass (and a git config the operator points at its own).
const KEPT = { PATH: '/usr/bin:/bin', HOME: '/opt/opuser', TMPDIR: '/var/tmp', GOPATH: '/opt/gopath', GOCACHE: '/opt/gocache', GOMODCACHE: '/opt/gomod' };
const MAKEENV = { ...KEPT, MAKEFLAGS: 'i', GITLEAKS_CONFIG: '/tmp/off.toml', GITLEAKS_ENABLE_RULES: 'x', GIT_DIR: '/nope', GOFLAGS: '-x', GOTOOLCHAIN: 'go9.9', GOWORK: '/nope/go.work', GIT_CONFIG_GLOBAL: '/opt/opuser/.gitconfig', GIT_CONFIG_NOSYSTEM: '0', GOENV: '/opt/opuser/go.env', GOPROXY: 'off', GOSUMDB: 'off', GONOSUMDB: '*', GOPRIVATE: '*', GOINSECURE: '*' };
const PINNED = { ...KEPT, GOTOOLCHAIN: 'local', GOWORK: 'off', GOENV: 'off', GOFLAGS: '', GOPROXY: 'https://proxy.golang.org,direct', GOSUMDB: 'sum.golang.org', GONOSUMDB: '', GOPRIVATE: '', GOINSECURE: '', GIT_CONFIG_GLOBAL: '/dev/null', GIT_CONFIG_NOSYSTEM: '1' };
const EXACT = JSON.stringify(Object.entries(PINNED).sort()); // git's env; the scan adds GOROOT, the resolved toolchain's own directory
const scanExact = (r) => JSON.stringify(Object.entries({ ...PINNED, GOROOT: path.dirname(path.dirname(r.w.go)) }).sort());
const envOf = (c) => c && JSON.stringify(Object.entries(c.opts.env).sort());
const scanEnvOf = (r) => envOf(scans(r)[0]);
await t('a scan killed by a signal (no exit status) refuses as a failed scan, not a finding or clean', { scan: 'killed' }, (r) => no(r, /failed to run/) && !r.ledgerNow.includes('LANDED'));
await t('the range scan runs with exactly the allowlisted env (Go pinned to the installed toolchain, no workspace, no env file, Go\'s default fetch settings, GOROOT the toolchain\'s own) and git config pinned off', { env: MAKEENV }, (r) => scanEnvOf(r) === scanExact(r));
await t('the commits the scan should read, and the paths the range touches, are derived by git in that same env (the name list, the rev-list and the diff, once for each of the two decisions)', { env: MAKEENV }, (r) => {
  const mine = r.w.calls.filter((c) => c.cmd === 'git' && ['log', 'rev-list'].includes(c.args[0]));
  return mine.length === 6 && mine.every((c) => envOf(c) === EXACT);
});
await t('the preflight child: stdin ignored, own session, env -i, HOME and TMPDIR inside the throwaway, entropy base is main\'s tip', {}, (r) => {
  const [sp] = r.w.spawns; if (!sp) return false; const home = sp.args.find((a) => a.startsWith('HOME='))?.slice(5) ?? ''; const tmp = sp.args.find((a) => a.startsWith('TMPDIR='))?.slice(7) ?? '';
  return sp.opts.stdio[0] === 'ignore' && sp.opts.detached === true && sp.args[0] === '-i' && path.dirname(home) === path.dirname(tmp) && home.includes('jm-land-') && sp.args.includes(`JM_ENTROPY_BASE=${MAIN}`) && sp.args.includes('PATH=/usr/bin:/bin:/usr/sbin:/sbin');
});
await t('untrusted author: refused before any fetch', { pr: { author: { login: 'mallory' } } }, (r) => no(r, /not trusted/) && !fetched(r));
await t('untrusted author in --preflight-only: refused before any fetch', { argv: PO, pr: { author: { login: 'mallory' } } }, (r) => no(r, /not trusted/) && !fetched(r));
await t('cross-repo PR: refused', { pr: { isCrossRepository: true } }, (r) => no(r, /cross-repository/));
await t('isCrossRepository missing from the reply: refused', { omit: ['isCrossRepository'] }, (r) => no(r, /not known to be same-repo/));
await t('a stranger with push on page 2 of the collaborators: refused', { page2: ['{"login":"eve","push":true}'] }, (r) => no(r, /not trusted: eve/));
await t('no ledger PASS: refused', { ledger: HDR }, (r) => no(r, /no verifier PASS for #5/));
await t('a CI failure on the head: refused', { runs: '{"total":1,"runs":[{"name":"ci","conclusion":"failure"}]}' }, (r) => no(r, /check run failed/));
await t('a PR without a pod verdict: refused', { msgs: [] }, (r) => no(r, /no accepted reviewer's pod message/));
await t('a PR with a verdict from the configured reviewer id lands', {}, (r) => r.merge);
await t('a PR with JM_VERDICT_REVIEWERS unset refuses, naming it', { env: { JM_VERDICT_REVIEWERS: '' } }, (r) => no(r, /environment variable JM_VERDICT_REVIEWERS is not set/) && r.w.pod.length === 0 && r.w.calls.length === 0);
await t('a verdict from a forged username with another id is refused', { msgs: [{ ...NOTE, userId: { _id: 'id-forged', username: 'orion' } }] }, (r) => no(r, /no accepted reviewer's pod message/));
const listed = [MAIN, ...commitsOf(`${MAIN}..${HEAD}`)]; // the trees land lists: main's tip and each commit of the range, for the scan configuration
await t('a docs-only PR with a ledger PASS and no pod verdict: refused; land classifies no paths (no diff of the PR is read, and the only tree listings are main\'s tip and the range\'s commits)', { msgs: [] }, (r) => no(r, /no accepted reviewer's pod message/) && !r.w.calls.some((c) => c.args.some((a) => ['--raw', '--numstat'].includes(a)) || (c.args.includes('ls-tree') && !listed.includes(c.args.at(-1)))));
await t('a range scan hit with a valid verdict: refused, naming the history', { scan: 'hit' }, (r) => no(r, /a secret was found in the range; remove it from history and push a new head/) && r.w.pod.length === 0); // scan first: no token read, no pod fetch
await t('a range scan that failed to run: refused', { scan: 'fail' }, (r) => no(r, /scan failed to run/) && !/secret was found/.test(r.error));
await t('a valid verdict with a clean scan lands (control)', { scan: 'clean' }, (r) => r.merge && r.ledgerNow.includes('\tLANDED\t'));
const refusedScan = (r) => no(r, /scan failed to run, or did not read the whole range/) && !/secret was found/.test(r.error) && r.w.pod.length === 0 && !r.ledgerNow.includes('LANDED');
await t('a clean scan report (exit 0, "no leaks found" and the scanned count equal to rev-list --count) lands', { count: 5 }, (r) => r.merge && argv(r, 'git', 'rev-list').includes(`rev-list ${MAIN}..${HEAD}`));
await t('a zero exit that printed the hit line (a recipe that swallowed the failure): refused as a scan failure, not clean', { scanErr: cleanScan(3) + 'WRN leaks found: 1\nexit status 3\n' }, refusedScan);
await t('a zero exit with an ERR log line (gitleaks logged an error and went on): refused as a scan failure', { scanErr: '10:11AM ERR failed to read an object\n' + cleanScan(3) }, refusedScan);
await t('a zero exit without "no leaks found": refused as a scan failure', { scanErr: '10:11AM INF 3 commits scanned.\n' }, refusedScan);
await t('a zero exit with no output at all (make -s printed nothing): refused as a scan failure', { scanErr: '' }, refusedScan);
await t('a zero exit that scanned fewer commits than the range holds: refused as a scan failure', { count: 4, scanned: 3 }, refusedScan);
await t('a zero exit that scanned an empty range while the land would merge commits: refused as a scan failure', { scanned: 0 }, refusedScan);
await t('a zero exit with the count line printed twice: refused as a scan failure', { scanErr: cleanScan(3) + '10:11AM INF 3 commits scanned.\n' }, refusedScan);
const entry = (file) => `100644 blob ${'a'.repeat(40)}\t${file}\0`;
const refusedConfig = (file) => (r) => no(r, new RegExp(`main carries ${file.replace('.', '\\.')}`)) && r.w.pod.length === 0 && !scans(r).length && !r.ledgerNow.includes('LANDED');
await t('main\'s tree at the fetched tip, then each range commit\'s, is read for scan configuration, whole (--full-tree) and with -r -z, at both decisions', {}, (r) => JSON.stringify(argv(r, 'git', 'ls-tree')) === JSON.stringify([...listed, ...listed].map((c) => `ls-tree -r -z --full-tree ${c}`)));
await t('main carries a .gitleaks.toml: refused, naming it, before the scan or the pod', { tree: entry('README.md') + entry('.gitleaks.toml') }, refusedConfig('.gitleaks.toml'));
await t('main carries a directory .gitleaks.toml/ and a .gitleaks.json (gitleaks would load the json): refused, naming the directory\'s path', { tree: entry('.gitleaks.toml/keep') + entry('.gitleaks.json') }, refusedConfig('.gitleaks.toml/keep'));
await t('main carries a .gitleaksignore: refused, naming it', { tree: entry('.gitleaksignore') }, refusedConfig('.gitleaksignore'));
await t('main carries a .gitattributes (any, owner ruling): refused, naming it, before the scan or the pod, and no blob is read', { tree: entry('README.md') + entry('.gitattributes') }, (r) => refusedConfig('.gitattributes')(r) && !argv(r, 'git', 'cat-file blob').length);
await t('main carries a nested docs/.GitAttributes: refused, naming it', { tree: entry('docs/.GitAttributes') }, refusedConfig('docs/.GitAttributes'));
await t('the checkout holds a .gitattributes in no tree: refused before the scan, naming its path, nothing merged', { topFiles: ['.gitattributes'] }, (r) => no(r, new RegExp(`${r.w.top}/\\.gitattributes exists; git reads a \\.gitattributes while it diffs`)) && !scans(r).length && r.w.pod.length === 0 && !r.ledgerNow.includes('LANDED'));
await t('commits that add no text (empty, rename, delete-only): gitleaks reads fewer than rev-list holds, and that is clean', { count: 5, bare: 2 }, (r) => r.merge && argv(r, 'git', 'log').includes(`log -m -p -U0 --format=%x01%H ${MAIN}..${HEAD}`));
await t('a range of only commits that add no text, scanned as 0: clean', { count: 2, bare: 2 }, (r) => r.merge);
await t('a scan that counted every rev-list commit although two add no text: refused as a scan failure', { count: 5, bare: 2, scanned: 5 }, refusedScan);
await t('a scan that read one commit fewer than the text commits: refused as a scan failure', { count: 5, bare: 2, scanned: 2 }, refusedScan);
await t('a killed scan (no exit status) whose stderr looks clean: refused as a failed scan, nothing merged', { scan: 'killedclean' }, (r) => no(r, /scan failed to run/) && r.w.pod.length === 0 && !r.ledgerNow.includes('LANDED'));
await t('a zero exit with only the partial-scan warning in place of "no leaks found": refused as a scan failure', { scanErr: '10:11AM INF 3 commits scanned.\n10:11AM WRN no leaks found in partial scan\n' }, refusedScan);
await t('the checkout has no .tools/bin/go: refused before the scan, naming it and the command that installs it (tools.sh with the system PATH alone; ./init.sh may install nothing), nothing merged', { noGo: true }, (r) => (r.error ?? "").includes(`${path.join(r.w.top, '.tools', 'bin', 'go')} is missing or not executable; land runs gitleaks with the checkout's own go, so run \`env PATH=/usr/bin:/bin:/usr/sbin:/sbin scripts/tools.sh\` in the land checkout (\`JM_TOOLCHAIN_CACHE=<dir> scripts/tools.sh\` links a shared toolchain cache instead)`) && no(r, /is missing or not executable/) && r.w.pod.length === 0 && !r.ledgerNow.includes('LANDED') && !r.w.calls.some((c) => c.args[0] === 'run'));
const NAMED = (path_) => `\u0001${'c0'.padEnd(40, '0')}\0\nA\0${path_}\0`;
await t('a commit that touches a path ending in a backslash: refused before the scan, naming the commit and the reason', { names: NAMED('b\\') }, (r) => no(r, /commit c0000000 touches the path "b\\\\", which ends in a backslash/) && !scans(r).length && r.w.pod.length === 0 && !r.ledgerNow.includes('LANDED'));
await t('a range whose paths all end in anything but a backslash lands (control)', { names: NAMED('b\\x.txt') }, (r) => r.merge);
const node = (code) => realRun(process.execPath, ['-e', code]);
cases.push(['the real run seam keeps stdout and stderr on a zero exit', JSON.stringify(node("console.log('o'); console.error('e')")), JSON.stringify({ status: 0, stdout: 'o\n', stderr: 'e\n' })], ['the real run seam returns the status and stderr of a failing command without throwing', JSON.stringify(node("console.error('bad'); process.exit(2)")), JSON.stringify({ status: 2, stdout: '', stderr: 'bad\n' })], ['the real run seam gives a null status for a command that never started', realRun('/nonexistent-jm-command', []).status, null]);
await t('JM_VERDICT_REVIEWERS with no valid entry is malformed, refused before the token read and the pod fetch', { env: { JM_VERDICT_REVIEWERS: 'bad,:x' } }, (r) => no(r, /JM_VERDICT_REVIEWERS is malformed/) && r.w.pod.length === 0 && r.w.calls.length === 0);
await t('a username equal to a listed id, with no _id, is not the reviewer', { msgs: [{ ...NOTE, userId: { username: 'id-orion' } }] }, (r) => no(r, /no accepted reviewer's pod message/));
await t('an _id that is an array holding the listed id is not the reviewer', { msgs: [{ ...NOTE, userId: { _id: ['id-orion'], username: 'x' } }] }, (r) => no(r, /no accepted reviewer's pod message/));
await t('an error carrying the token after the pod read is scrubbed by main\'s catch', { on: [[/^gh pr merge/, () => { throw new Error('boom jmu_FAKE-token-abc123'); }]] }, (r) => !r.ledgerNow.includes('LANDED') && r.error.includes('[token]') && !r.error.includes('abc123'));
await t('a verdict whose first line carries only a PREFIX of the head: refused', { msgs: [{ ...NOTE, content: `jellymoon#5 ${HEAD.slice(0, 8)}\nGATE: PASS` }] }, (r) => no(r, /no accepted reviewer's pod message/));
await t('a verdict whose first line carries a 39-hex prefix of the head: refused', { msgs: [{ ...NOTE, content: `jellymoon#5 ${HEAD.slice(0, 39)}\nGATE: PASS` }] }, (r) => no(r, /no accepted reviewer's pod message/));
await t('a verdict whose first line carries the head plus a suffix: refused', { msgs: [{ ...NOTE, content: `jellymoon#5 ${HEAD}0\nGATE: PASS` }] }, (r) => no(r, /no accepted reviewer's pod message/));
// Pod paging: a partial list must never be judged (a page-1 PASS over a page-2 FAIL would land).
const FAILMSG = { ...NOTE, userId: { _id: 'id-vega', username: 'vega' }, content: `jellymoon#5 ${HEAD}\nGATE: FAIL`, createdAt: '2026-10-29T09:30:00Z' };
const TWO = { JM_VERDICT_REVIEWERS: 'id-orion:Orion,id-vega:Vega' };
const older = (r) => r.w.pod.filter((p) => p.url.includes('before=')).length; // fetches that went back past page 1
const page = (items) => ({ ok: true, status: 200, json: async () => items });
const full = Array.from({ length: 200 }, (_, i) => ({ id: `m${i}`, userId: { _id: 'other', username: 'o' }, content: 'x', createdAt: new Date(Date.parse('2026-10-29T11:00:00Z') - i * 1000).toISOString() }));
await t('paging: page 2 holds the FAIL that page 1 does not: refused', { env: TWO, plan: (n) => page(n === 1 ? [NOTE] : [FAILMSG]) }, (r) => no(r, /blocked by Vega/) && older(r) >= 1);
await t('paging: a full page that repeats itself (a stuck cursor) refuses', { plan: () => page(full) }, (r) => no(r, /page boundary could hide a message/) && r.w.pod.length === 2);
await t('paging: a message without a timestamp refuses', { plan: () => page([{ ...NOTE, createdAt: undefined }]) }, (r) => no(r, /no timestamp/));
await t('paging: it stops once a page reaches back past the PR creation (no older page is fetched)', { pr: { createdAt: '2026-10-29T10:30:00Z' } }, (r) => r.merge && older(r) === 0);
await t('paging: it keeps going while pages are newer than the PR creation (an older page is fetched)', {}, (r) => r.merge && older(r) > 0);
await t('paging: an HTTP error on page 2 refuses, not a partial list', { plan: (n) => (n === 1 ? page([NOTE]) : { ok: false, status: 502, json: async () => [] }) }, (r) => no(r, /HTTP 502/));
await t('paging: a fetch error after page 1 refuses', { plan: (n) => { if (n === 1) return page([NOTE]); throw Object.assign(new Error('down'), { name: 'TimeoutError' }); } }, (r) => no(r, /request failed \(TimeoutError\)/));
await t('paging: a reply that is not JSON refuses', { plan: (n) => (n === 1 ? page([NOTE]) : { ok: true, status: 200, json: async () => { throw new SyntaxError('x'); } }) }, (r) => no(r, /unreadable reply \(SyntaxError\)/));
await t('paging: a reply that is not a list refuses in words, not a TypeError', { plan: () => page({ oops: 1 }) }, (r) => no(r, /not a list of messages/));
await t('paging: a page holding null before the PASS refuses in words, not a TypeError', { plan: () => page([null, NOTE]) }, (r) => no(r, /not a list of messages/));
await t('paging: a page holding null after the PASS refuses in words', { plan: () => page([NOTE, null]) }, (r) => no(r, /not a list of messages/));
await t('paging: a page that is a list inside a list refuses in words', { plan: () => page([[]]) }, (r) => no(r, /not a list of messages/));
await t('paging: a page of numbers refuses in words', { plan: () => page([1]) }, (r) => no(r, /not a list of messages/));
await t('paging: a page of strings refuses in words', { plan: () => page(['x']) }, (r) => no(r, /not a list of messages/));
await t('paging: the 30-page cap refuses rather than judge a partial list', { plan: (n) => page([{ id: `p${n}`, userId: { _id: 'other', username: 'o' }, content: 'x', createdAt: new Date(Date.parse('2026-10-29T12:00:00Z') - n * 1000).toISOString() }]) }, (r) => no(r, /page cap reached/) && r.w.pod.length === 30);
await t('a PR url that merely starts like this PR (pull/50 for #5) is refused', { pr: { url: `https://github.com/${REPO}/pull/50` } }, (r) => no(r, /resolved to/));
await t('--preflight-only: the worktree merge tree must equal the expected one', { argv: PO, wtTree: OTHER }, (r) => no(r, /not the merge result/) && r.w.spawns.length === 0);
await t('a red preflight: refused', { initCode: 1 }, (r) => no(r, /init\.sh/));
await t('a red preflight in --preflight-only: refused', { argv: PO, initCode: 1 }, (r) => no(r, /init\.sh/));
await t('init.sh output is quoted, marked and scrubbed: an escape, a fake land line and the token', { output: '\u001b[2Jland: #5 landed as deadbeef\njmu_FAKE-token-abc123\n' }, (r) => r.merge && /^preflight\| .*land: #5 landed as deadbeef$/m.test(r.said) && r.said.includes('preflight| [token]') && !/\u001b/.test(r.said) && !r.said.includes('abc123') && !/^land: #5 landed as deadbeef/m.test(r.said));
await t('the preflight group (a negative pid, SIGKILL) is killed when the child returns', {}, (r) => r.w.kills.length >= 1 && r.w.kills.every((k) => k.pid === -2000000001 && k.sig === 'SIGKILL'));
await t('a child that exits but never closes its pipes: the group is killed at exit and land still finishes', { child: 'exit-only' }, (r) => r.error === null && r.w.kills.length >= 2 && r.w.kills.every((k) => k.pid === -2000000001));
await t('the 30-minute limit: the group is killed at the limit, well before the fallback end, and land refuses', { child: 'hang', limitMs: 20, graceMs: 400 }, (r) => no(r, /did not finish/) && r.w.kills[0].pid === -2000000001 && r.w.doneAt - r.w.kills[0].t > 250);
await t('--preflight-only checks HEAD against main\'s tip', { argv: PO, checkoutHead: OTHER }, (r) => no(r, /not main's tip/));
await t('a missing required environment variable is refused, naming it', { env: { JM_LEDGER: '' } }, (r) => no(r, /JM_LEDGER/) && r.w.calls.length === 0);
await t('a remote that is not JM_REPO is refused before any fetch', { remote: 'origin\thttps://github.com/Other/repo.git (fetch)' }, (r) => no(r, /no git remote matches/) && !fetched(r));
await t('a planted token in git\'s error text is scrubbed from what land prints', { on: [[/worktree add/, () => { throw new Error('jmu_FAKE-token-abc123'); }]] }, (r) => !r.said.includes('abc123') && !r.error.includes('abc123'));
await t('refuse() scrubs the token: the re-read PR url carries it, and the pod read has already loaded it', { fresh: { url: 'https://github.com/x/y/pull/5?jmu_FAKE-token-abc123\u001b[2J' } }, (r) => no(r, /resolved to/) && !r.error.includes('abc123') && r.error.includes('[token]') && !r.error.includes('\u001b'));
await t('say() strips escapes from what it prints (the merge commit id here)', { pr: { mergeCommit: { oid: 'ab\u001b]0;x\u0007cd' } } }, (r) => r.merge && r.said.includes('landed as') && !/[\u001b\u0007]/.test(r.said));
await t('the PR head differs from the local fetch in --preflight-only: refused', { argv: PO, pullHead: OTHER }, (r) => no(r, /differs from the local fetch/));
await t('the throwaway worktree is removed on a normal return, not left for process exit', {}, (r) => argv(r, 'git', 'worktree remove').length === 1 && r.w.calls.findIndex((c) => c.args.includes('remove')) > -1);
await t('the PR head differs from the local fetch: refused', { pullHead: OTHER }, (r) => no(r, /differs from the local fetch/));
await t('the ledger changes during the preflight: the second decide refuses', { onSpawn: (w) => fs.appendFileSync(w.ledger, rowOf('FAIL', 'sonnet-verifier')) }, (r) => no(r, /changed before the merge/));
await t('the head moves on GitHub before the merge: refused', { fresh: { headRefOid: OTHER } }, (r) => no(r, /changed before the merge/));
await t('the PR is retargeted between the first read and the re-read: refused', { fresh: { baseRefName: 'release' } }, (r) => no(r, /based on release, not main.*changed before the merge/));
await t('the PR is closed between the first read and the re-read: refused', { fresh: { state: 'CLOSED' } }, (r) => no(r, /is CLOSED.*changed before the merge/));
await t('a PR whose draft state is unknown is refused as a draft', { omit: ['isDraft'] }, (r) => no(r, /#5 is a draft/));
await t('a PR turned draft between the first read and the re-read: refused', { fresh: { isDraft: true } }, (r) => no(r, /#5 is a draft.*changed before the merge/));
await t('the author changes before the merge: the trust gate runs again', { fresh: { author: { login: 'mallory' } } }, (r) => no(r, /not trusted/));
await t('main moves during the run: refused', { mainTips: [MAIN, OTHER] }, (r) => no(r, /main moved/));
await t('dry run: no merge, ledger byte-identical, preflight still run', { argv: ['5', '--dry-run'] }, (r) => !r.merge && r.ledgerNow === r.ledgerWas && r.error === null && r.w.spawns.length === 1);
await t('dry run on a PR merged out of band: nothing written', { argv: ['5', '--dry-run'], pr: { state: 'MERGED' }, ledger: HDR }, (r) => !r.merge && r.ledgerNow === r.ledgerWas && r.said.includes('dry run'));
await t('a PR merged out of band is recorded MERGED-UNVERIFIED, nothing merged', { pr: { state: 'MERGED' }, ledger: HDR }, (r) => !r.merge && r.ledgerNow.includes('MERGED-UNVERIFIED'));
await t('the merged commit\'s tree differs from the tested one: TREE-MISMATCH on the LANDED row, a non-zero refusal naming both trees, no "safe to delete"', { mergedTree: OTHER }, (r) => r.merge && r.ledgerNow.includes('TREE-MISMATCH') && r.ledgerNow.includes('LANDED') && /not the tested merge result/.test(r.error ?? '') && (r.error ?? '').includes(TREE) && (r.error ?? '').includes(OTHER) && !r.said.includes('safe to delete'));
await t('checkout gate: a dirty tree refuses before any fetch', { on: dirty }, (r) => no(r, /uncommitted/) && !fetched(r));
await t('checkout gate: a tree that turns dirty only after the fetch is refused', { on: [[/status --porcelain/, (a, w) => (w.fetches.length ? ' M x' : '')]] }, (r) => no(r, /uncommitted/));
await t('checkout gate: HEAD that is not main\'s tip is refused', { checkoutHead: OTHER }, (r) => no(r, /not main's tip/));
await t('checkout gate: an assume-unchanged file is refused', { on: [[/^git ls-files -v/, 'h x']] }, (r) => no(r, /assume-unchanged/));
await t('checkout gate: a diff from HEAD is refused', { on: [[/^git diff --quiet/, () => { throw fail; }]] }, (r) => no(r, /differs from HEAD/));
await t('checkout gate: credentials in a remote URL are refused', { remote: `origin\thttps://u:p@github.com/${REPO}.git (fetch)` }, (r) => no(r, /userinfo/));
await t('checkout gate: an http extraheader is refused', { on: [[/^git config --get-regexp/, 'http.x.extraheader AUTH']] }, (r) => no(r, /extraheader/));
await t('checkout gate: .ENV at the repo root is refused (any case)', { topFiles: ['.ENV'] }, (r) => no(r, /env/i));
await t('checkout gate: BUNFIG.TOML in the cwd is refused (any case)', { cwdFiles: ['BUNFIG.TOML'] }, (r) => no(r, /bunfig/i));
await t('checkout gate: --preflight-only has it too', { argv: PO, on: dirty }, (r) => no(r, /uncommitted/));
await t('bootstrap: refused once main has scripts/land.mjs, in both paths', { argv: ['5', '--bootstrap'], checkoutHead: HEAD }, (r) => no(r, /only for the first land/));
await t('bootstrap (preflight-only): refused once main has scripts/land.mjs', { argv: [...PO, '--bootstrap'], checkoutHead: HEAD }, (r) => no(r, /only for the first land/));
const boot = { argv: ['5', '--bootstrap'], checkoutHead: HEAD, on: [[/^git cat-file -e \S+:(scripts\/land\.mjs|entropy\.json)/, () => { throw fail; }]] };
await t('bootstrap from a checkout that is not the PR head: refused (land)', { ...boot, checkoutHead: OTHER }, (r) => no(r, /checkout of the PR head/) && r.w.spawns.length === 0 && scans(r).length === 0 && r.w.pod.length === 0 && !r.w.fetches.some((f) => f.startsWith('pull/')));
await t('bootstrap (preflight-only) from a checkout that is not the PR head: refused', { ...boot, argv: [...PO, '--bootstrap'], checkoutHead: OTHER }, (r) => no(r, /checkout of the PR head/) && r.w.spawns.length === 0 && scans(r).length === 0 && r.w.pod.length === 0 && !r.w.fetches.some((f) => f.startsWith('pull/')));
await t('bootstrap on a main without land.mjs: entropy base is none, a BOOTSTRAP row is written', { ...boot }, (r) => r.merge && r.w.spawns[0].args.includes('JM_ENTROPY_BASE=none') && r.ledgerNow.includes('\tBOOTSTRAP\t'));
await t('API origin that is not commonly: refused', { env: { JM_COMMONLY_API: 'https://evil.example' } }, (r) => no(r, /JM_COMMONLY_API must be/) && r.w.calls.length === 0);
await t('a PR url for another repo: refused', { pr: { url: 'https://github.com/Other/repo/pull/5' } }, (r) => no(r, /resolved to/));
await t('--dry-run with --preflight-only: refused', { argv: [...PO, '--dry-run'] }, (r) => no(r, /usage/));
await t('two PR numbers: refused', { argv: ['5', '6'] }, (r) => no(r, /usage/));
await t('an old Node: refused before anything runs', { node: '18.0.0' }, (r) => no(r, /Node 20/) && r.w.calls.length === 0);
await t('preflight-only: no ledger, no pod, no merge', { argv: PO }, (r) => r.error === null && r.w.pod.length === 0 && !r.merge && r.ledgerNow === r.ledgerWas);
await t('the worktree merge tree must equal the decision\'s merge-tree result', { wtTree: OTHER }, (r) => no(r, /not the merge result the decision used/) && r.w.spawns.length === 0);
await t('the throwaway worktree is added with repo hooks off', {}, (r) => argv(r, 'git', 'worktree add')[0]?.startsWith('-c core.hooksPath=/dev/null worktree add'));
await t('a branch name with escapes is stripped from what land prints', { pr: { headRefName: 'a\u001b[31mb\u009bc' } }, (r) => r.said.includes('branch a[31mbc ') && !/[\u001b\u009b]/.test(r.said));
await t('the bearer token never appears in a refusal', { on: [[/worktree add/, () => { throw new Error('boom jmu_FAKE-token-abc123'); }]] }, (r) => !r.merge && /worktree add failed: boom/.test(r.error) && !r.error.includes('abc123'));
// The CodeRabbit path: the ledger row names it, land reads the reviews, threads and commits itself (shaped like the API's replies).
const EARLY = 'c'.padEnd(40, '0'); // the first commit of the fake range
const BOT_REVIEW = (head = HEAD, state = 'COMMENTED') => `{"user":{"login":"coderabbitai[bot]","type":"Bot"},"commit_id":"${head}","state":"${state}"}`;
const GQL = (nodes = []) => JSON.stringify({ data: { repository: { pullRequest: { reviewThreads: { pageInfo: { hasNextPage: false, endCursor: null }, nodes } } } } });
const MAJOR_THREAD = { isResolved: false, isOutdated: false, comments: { pageInfo: { hasNextPage: false }, nodes: [{ author: { __typename: 'Bot', login: 'coderabbitai' }, body: '_⚠️ Potential issue_ | _🟠 Major_\n\nx' }] } };
const CR_LEDGER = HDR + rowOf('PASS', 'opus-verifier', EARLY) + rowOf('PASS', 'coderabbit');
const cr = (o = {}) => ({ ledger: CR_LEDGER, on: [
  [/^gh api .*pulls\/5\/reviews/, o.reviews ?? BOT_REVIEW()], [/^gh api graphql/, o.threads ?? GQL()],
  [/^gh api .*issues\/5\/comments/, () => { if (o.comments === undefined) throw new Error('comments read failed'); return o.comments; }],
  [/^gh api repos\/\S+\/commits\/c\d+ /, (a) => { const sha = a.find((x) => x.includes('/commits/')).split('/').pop(); return JSON.stringify({ sha, author: { login: o.author ?? 'alice' }, committer: { login: o.committer ?? 'alice' } }); }],
], ...o.over });
await t('coderabbit path: lands, says so, and the LANDED note records the path and both heads', cr(), (r) => r.merge && r.said.includes(`coderabbit at ${HEAD.slice(0, 8)}, after the review at ${EARLY.slice(0, 8)}`) && r.ledgerNow.includes(`admitted by coderabbit at ${HEAD.slice(0, 8)}, after the review at ${EARLY.slice(0, 8)}`));
await t('coderabbit path: the reads are the PR\'s reviews, a paginated GraphQL threads query, and one commit read per commit of main..head', cr(), (r) => argv(r, 'gh', 'pulls/5/reviews')[0]?.includes('--paginate') && argv(r, 'gh', 'api graphql').length === 2 && argv(r, 'gh', '/commits/c').length === 6);
await t('coderabbit path: a review only on an older head refuses, nothing merged', cr({ reviews: BOT_REVIEW(OTHER) }), (r) => no(r, /CodeRabbit has not reviewed/) && !r.ledgerNow.includes('LANDED'));
await t('coderabbit path: an unanswered Major thread refuses', cr({ threads: GQL([MAJOR_THREAD]) }), (r) => no(r, /unresolved and unanswered/));
await t('coderabbit path: a commit by an untrusted login refuses', cr({ committer: 'mallory' }), (r) => no(r, /no committer login in JM_TRUSTED_LOGINS/));
await t('coderabbit path: a GraphQL error reply refuses', cr({ threads: '{"errors":[{"message":"x"}],"data":null}' }), (r) => no(r, /unexpected reply/));
await t('coderabbit path: no earlier verifier row refuses before any CodeRabbit read', cr({ over: { ledger: HDR + rowOf('PASS', 'coderabbit') } }), (r) => no(r, /one local review is missing/) && argv(r, 'gh', 'api graphql').length === 0);
await t('coderabbit path: a FAIL at the head refuses even with a coderabbit PASS', cr({ over: { ledger: CR_LEDGER + rowOf('FAIL', 'sonnet-verifier') } }), (r) => no(r, /a FAIL blocks that head/));
await t('coderabbit path: the pod verdict is still required', cr({ over: { msgs: [] } }), (r) => no(r, /no accepted reviewer/));
// The summary way: a clean incremental review files no review, so land reads the issue comments (paginated) and finds CodeRabbit's summary.
const SUMMARY = (head = HEAD) => '<!-- coderabbit-ai-generated:v1 -->\n\n<!-- This is an auto-generated comment: summarize by coderabbit.ai -->\n<!-- recent_review_status_start -->\n\nNo actionable comments were generated in the recent review.\n\n<!-- recent_review_status_end -->\n<!-- walkthrough_start -->\nx\n<!-- walkthrough_end -->\n<!-- final_review_risk_start -->\n<!-- final_review_risk_coverage:' + JSON.stringify({ sourceCommitId: head, coveredCommitId: head, kind: 'reviewed' }) + ' -->\n<!-- final_review_risk_end -->\n';
const CMT = (login, type, body) => JSON.stringify({ user: { login, type }, body });
const PAGE1 = Array.from({ length: 100 }, (_, i) => CMT('alice', 'User', `comment ${i}`)).join('\n'); // a full first page, the summary is on page 2
const withSummary = (head = HEAD) => PAGE1 + '\n' + CMT('coderabbitai[bot]', 'Bot', SUMMARY(head));
await t('coderabbit summary way: no review at the head, the summary on page 2 of the comments, lands and says so', cr({ reviews: '', comments: withSummary() }), (r) => r.merge && r.said.includes(`coderabbit at ${HEAD.slice(0, 8)} (summary comment), after the review at ${EARLY.slice(0, 8)}`) && argv(r, 'gh', 'issues/5/comments')[0]?.includes('--paginate'));
await t('coderabbit summary way: a summary for another head is a gap and lands at once, with the countable note', cr({ reviews: '', comments: withSummary(OTHER) }), (r) => r.merge && r.ledgerNow.includes('no CodeRabbit review at this head'));
await t('coderabbit summary way: a failed comments read refuses, nothing merged', cr({ reviews: '' }), (r) => no(r, /CodeRabbit has not reviewed/) && !r.ledgerNow.includes('LANDED'));
await t('coderabbit summary way: the marker in another bot\'s comment is no summary: a gap, lands with the note', cr({ reviews: '', comments: CMT('github-actions[bot]', 'Bot', SUMMARY()) }), (r) => r.merge && r.ledgerNow.includes('no CodeRabbit review at this head'));
// LCR3R: a gap lands at once, with no ask, no clock and no timeline read; what CodeRabbit already said at the head still decides.
const NOTCLEAN = SUMMARY().replace('No actionable comments were generated in the recent review.', 'Actionable comments posted: 2');
const LIMITED = SUMMARY().replace('<!-- recent_review_status_start -->', '<!-- This is an auto-generated comment: rate limited by coderabbit.ai -->\n<!-- recent_review_status_start -->');
await t('coderabbit gap: no review and no summary lands at once with no ask, and says so', cr({ reviews: '', comments: CMT('alice', 'User', 'looks fine') }), (r) => r.merge && r.said.includes(`coderabbit at ${HEAD.slice(0, 8)} (no CodeRabbit review at this head), after the review at ${EARLY.slice(0, 8)}`));
await t('coderabbit gap: the LANDED note records "no CodeRabbit review at this head"', cr({ reviews: '', comments: '' }), (r) => r.merge && r.ledgerNow.includes(`admitted by coderabbit at ${HEAD.slice(0, 8)} (no CodeRabbit review at this head), after the review at`));
await t('coderabbit gap: a rate limited summary lands at once', cr({ reviews: '', comments: CMT('coderabbitai[bot]', 'Bot', LIMITED) }), (r) => r.merge && r.ledgerNow.includes('no CodeRabbit review at this head'));
await t('coderabbit gap: land reads no timeline and no ask', cr({ reviews: '', comments: '' }), (r) => r.merge && argv(r, 'gh', 'timeline').length === 0);
await t('coderabbit not clean: a not-clean summary at the head with no unanswered Major lands', cr({ reviews: '', comments: CMT('coderabbitai[bot]', 'Bot', NOTCLEAN) }), (r) => r.merge && r.said.includes('(summary comment)'));
await t('coderabbit not clean: the same beside an unanswered Major thread refuses', cr({ reviews: '', comments: CMT('coderabbitai[bot]', 'Bot', NOTCLEAN), threads: GQL([MAJOR_THREAD]) }), (r) => no(r, /unresolved and unanswered/) && !r.ledgerNow.includes('LANDED'));
await t('coderabbit gap: CHANGES_REQUESTED at the head refuses', cr({ reviews: BOT_REVIEW(HEAD, 'CHANGES_REQUESTED'), comments: '' }), (r) => no(r, /requested changes/) && !r.ledgerNow.includes('LANDED'));
await t('coderabbit gap: an unanswered Major thread refuses beside a gap', cr({ reviews: '', comments: '', threads: GQL([MAJOR_THREAD]) }), (r) => no(r, /unresolved and unanswered/) && !r.ledgerNow.includes('LANDED'));
await t('coderabbit gap: a failed comments read is no gap and refuses', cr({ reviews: '' }), (r) => no(r, /could not be read/) && !r.ledgerNow.includes('LANDED'));
await t('coderabbit summary way: a review at the head settles (c) without reading the comments', cr(), (r) => r.merge && argv(r, 'gh', 'issues/5/comments').length === 0);
await t('verifier path: unchanged, says so, reads nothing from CodeRabbit', {}, (r) => r.merge && r.said.includes('verifier PASS by opus-verifier') && r.ledgerNow.includes('admitted by verifier PASS by opus-verifier') && argv(r, 'gh', 'api graphql').length === 0 && argv(r, 'gh', 'pulls/5/reviews').length === 0);
// LPRE: land reads CI instead of running the preflight only when CI proved the merge-result tree; every other case runs the preflight as before.
const LAND = 'a'.repeat(40);
const LREF = 'refs/heads/land/5';
const RULES = '{"type":"pull_request","parameters":{}}\n{"type":"required_status_checks","parameters":{"required_status_checks":[{"context":"ci","integration_id":15368}]}}';
const crun = (o = {}) => ({ name: 'ci', status: 'completed', conclusion: 'success', app: 'github-actions', suite: 1, ...o }); // suite: the check suite; 1 is land's own push to land/5, 9 a pull_request suite
const pending = crun({ status: 'in_progress', conclusion: null });
// polls: the run lists CI shows on successive reads (the last repeats). there: a leftover land/5 on the remote, thereTree its tree.
// prior: the runs GitHub already holds for the land commit before land pushes anything (without it GitHub does not have the commit yet and the read fails).
const ci = (o = {}) => {
  let k = 0; let pushed = false;
  const polls = o.polls ?? [[crun()]];
  const reused = o.thereTree === TREE && sameArgs(o.thereParents ?? [MAIN, HEAD], [MAIN, HEAD]); // land reuses a leftover only for the merge tree AND parents main's tip, then the head
  return { on: [
    [/\/rules\/branches\/main/, () => { if (o.rules === null) throw fail; return o.rules ?? RULES; }],
    [/\/branches\/main\/protection\/required_status_checks/, () => { throw fail; }],
    [/^git (-c \S+ )?commit-tree /, () => { if (o.noCommit) throw fail; return o.commit ?? LAND; }],
    [new RegExp(`^git rev-parse ${LAND}\\^\\{tree\\}`), o.landTree ?? TREE],
    [/^git ls-remote /, o.there ? `${o.there}\t${LREF}` : ''],
    [/^git show -s --format=%ct /, o.ct ?? `${o.mainCt ?? 1700000000}\n${o.headCt ?? 1700000100}`],
    [/^git push /, (a) => { if (o.pushFails && !a.includes('--delete')) throw fail; if (o.deleteFails && a.includes('--delete')) throw fail; pushed = true; return ''; }],
    ...(o.there ? [[new RegExp(`/git/commits/${o.there}`), (a) => { if (o.thereUnreadable) throw fail; return a.some((x) => x.includes('.parents')) ? [o.thereTree ?? OTHER, ...(o.thereParents ?? [MAIN, HEAD])].join(' ') : (o.thereTree ?? OTHER); }]] : []),
    [/actions\/runs\?head_sha=\w+&event=push&branch=land\/5&/, () => JSON.stringify({ total: 1, count: 1, ids: o.suites ?? [1] })], // the suites of runs triggered by a push to land/5 on this SHA
    [new RegExp(`/git/commits/${LAND}`), () => { if (!pushed && !o.prior) throw fail; return o.ciTree ?? TREE; }],
    [new RegExp(`commits/${reused ? o.there : LAND}/check-runs`), () => { if (o.runsFail) throw fail; if (!pushed && !reused && o.prior) return JSON.stringify({ total: o.prior.length, runs: o.prior }); const runs = polls[Math.min(k, polls.length - 1)]; k += 1; return JSON.stringify({ total: runs.length, runs }); }],
  ], ...o.over };
};
const pushes = (r) => r.w.calls.filter((c) => c.cmd === 'git' && c.args[0] === 'push').map((c) => c.args);
const DEL = ['push', '--quiet', 'origin', '--delete', LREF];
const makeArgs = (there = '') => ['push', '--quiet', `--force-with-lease=${LREF}:${there}`, 'origin', `${LAND}:${LREF}`];
const sameArgs = (a, b) => JSON.stringify(a) === JSON.stringify(b);
const lastIs = (r, a) => sameArgs(pushes(r).at(-1), a);
const mergedAt = (r) => r.w.calls.findIndex((c) => c.cmd === 'gh' && c.args[1] === 'merge');
const delAt = (r) => r.w.calls.findIndex((c) => c.cmd === 'git' && sameArgs(c.args, DEL));
const skipped = (r) => r.merge && r.w.spawns.length === 0 && r.said.includes(`CI proved merge tree ${TREE.slice(0, 8)}`) && r.ledgerNow.includes(`CI proved merge tree ${TREE.slice(0, 8)}`);
const local = (r, re) => r.merge && r.w.spawns.length === 1 && r.ledgerNow.includes('; local preflight (') && re.test(r.said);
// LPRE2: land builds the merge commit, pushes it to land/<n>, and waits for CI on THAT commit; anything short of green runs the preflight as before.
await t('ci proof: green on the land commit lands without the preflight, the note says so, and the land ref is deleted after the merge', ci(), (r) => skipped(r) && sameArgs(pushes(r), [makeArgs(), DEL]) && delAt(r) > mergedAt(r) && mergedAt(r) > 0);
await t('ci proof: the land commit is built from the decision tree with main\'s tip and the head as parents', ci(), (r) => sameArgs(argv(r, 'git', 'commit-tree'), [`-c i18n.commitEncoding=UTF-8 commit-tree --no-gpg-sign ${TREE} -p ${MAIN} -p ${HEAD} -m land #5`]));
const ENV = (r) => argv(r, 'git', 'commit-tree').length === 1 && r.w.calls.find((c) => c.cmd === 'git' && c.args.includes('commit-tree')).opts.env;
await t('ci proof: the land commit is built under a fixed identity and the later parent\'s date, not the clock', ci({ mainCt: 1700000300, headCt: 1700000100 }), (r) => { const e = ENV(r); return e && e.GIT_AUTHOR_NAME === 'land' && e.GIT_COMMITTER_EMAIL === 'land@localhost' && e.GIT_AUTHOR_DATE === '@1700000300 +0000' && e.GIT_COMMITTER_DATE === '@1700000300 +0000'; });
await t('ci proof: parent dates that cannot be read run the local preflight, pushing nothing', ci({ ct: 'x' }), (r) => local(r, /could not build the land commit/) && pushes(r).length === 0);
await t('ci proof: green checks already on the deterministic land commit (its ref long deleted) are the proof: no push, no delete, merged', ci({ prior: [crun()] }), (r) => skipped(r) && pushes(r).length === 0 && r.w.sleeps.length === 0 && mergedAt(r) > 0);
await t('ci proof: a red or unfinished prior run on the land commit is no proof: land pushes and waits as before', ci({ prior: [crun({ conclusion: 'failure' })] }), (r) => skipped(r) && sameArgs(pushes(r), [makeArgs(), DEL]));
await t('ci proof: a prior proof is not looked for when a matching leftover ref is reused', ci({ prior: [crun({ conclusion: 'failure' })], there: 'd'.repeat(40), thereTree: TREE }), (r) => r.merge && pushes(r).length === 1 && lastIs(r, DEL));
await t('ci proof: green prior runs from a pull_request suite (a fork PR on the computed SHA) are no proof: land pushes and waits for its own', ci({ prior: [crun({ suite: 9 })] }), (r) => skipped(r) && sameArgs(pushes(r), [makeArgs(), DEL]));
await t('ci proof: green runs of a land/5 push suite found after the ref is gone are the proof (the re-run path)', ci({ prior: [crun({ suite: 1 }), crun({ suite: 9, conclusion: 'failure' })] }), (r) => skipped(r) && pushes(r).length === 0);
await t('ci proof: while waiting, green runs of a pull_request suite never satisfy the poll (local preflight at the cap)', ci({ polls: [[crun({ suite: 9 })]] }), (r) => local(r, /has no run on the land commit yet/));
await t('ci proof: the suites are read for push events on land/5 only, and the runs with filter=latest', ci({ prior: [crun()] }), (r) => argv(r, 'gh', 'actions/runs?head_sha=').length === 1 && argv(r, 'gh', 'check-runs?per_page=100&filter=latest').length === 1);
await t('ci proof: checks still running are polled every 20 s until green', ci({ polls: [[], [pending], [pending], [crun()]] }), (r) => skipped(r) && r.w.sleeps.length === 3 && r.w.sleeps.every((ms) => ms === 20000));
await t('ci proof: a required check failing on the land commit runs the local preflight at once, and the ref is still deleted', ci({ polls: [[crun({ conclusion: 'failure' })]] }), (r) => local(r, /did not succeed on the land commit \(failure\)/) && r.w.sleeps.length === 0 && lastIs(r, DEL));
await t('ci proof: a cancelled required check runs the local preflight', ci({ polls: [[crun({ conclusion: 'cancelled' })]] }), (r) => local(r, /cancelled/));
await t('ci proof: a skipped required check runs the local preflight', ci({ polls: [[crun({ conclusion: 'skipped' })]] }), (r) => local(r, /skipped/));
await t('ci proof: the cap (15 min of 20 s polls) runs the local preflight and says why', ci({ polls: [[pending]] }), (r) => local(r, /did not finish in 15 minutes/) && r.w.sleeps.length === 45 && lastIs(r, DEL));
await t('ci proof: a required check with no run for the whole cap runs the local preflight', ci({ polls: [[]] }), (r) => local(r, /has no run on the land commit yet/) && r.w.sleeps.length === 45);
await t('ci proof: a same-named run from another app is ignored beside the green Actions run', ci({ polls: [[crun(), crun({ app: 'some-other-app', conclusion: 'failure' })]] }), skipped);
await t('ci proof: a green same-named run from another app alone proves nothing', ci({ polls: [[crun({ app: 'some-other-app' })]] }), (r) => local(r, /has no run on the land commit yet/));
await t('ci proof: a same-named run with no app (a user) alone proves nothing', ci({ polls: [[crun({ app: null })]] }), (r) => local(r, /has no run on the land commit yet/));
await t('ci proof: the land commit built locally with a tree that is not the merge result refuses, pushing nothing', ci({ landTree: OTHER }), (r) => no(r, /land commit is not the merge result/) && pushes(r).length === 0 && r.w.spawns.length === 0);
await t('ci proof: GitHub\'s tree for the pushed land commit not the merge result refuses and deletes the ref', ci({ ciTree: OTHER }), (r) => no(r, /not the merge result/) && lastIs(r, DEL) && r.w.spawns.length === 0);
await t('ci proof: the delete only ever names the land ref, never the PR branch, even when the refusal comes later', { ...ci(), mainTips: [MAIN, OTHER] }, (r) => no(r, /main/i) && lastIs(r, DEL) && pushes(r).every((a) => !a.includes('feat') && !a.includes('refs/heads/feat') && !a.includes('main')));
await t('ci proof: a leftover land ref with a different tree is overwritten under a lease on exactly that commit', ci({ there: 'd'.repeat(40) }), (r) => skipped(r) && sameArgs(pushes(r)[0], makeArgs('d'.repeat(40))));
await t('ci proof: a leftover land ref with the same tree is reused, not pushed over, and still deleted', ci({ there: 'd'.repeat(40), thereTree: TREE }), (r) => r.merge && pushes(r).length === 1 && lastIs(r, DEL));
await t('ci proof: a leftover land ref with the merge tree but other parents (the head differs) is overwritten under a lease, not reused', ci({ there: 'd'.repeat(40), thereTree: TREE, thereParents: [MAIN, OTHER] }), (r) => skipped(r) && sameArgs(pushes(r)[0], makeArgs('d'.repeat(40))) && pushes(r).length === 2);
await t('ci proof: a leftover land ref with the merge tree but another base is overwritten under a lease, not reused', ci({ there: 'd'.repeat(40), thereTree: TREE, thereParents: [OTHER, HEAD] }), (r) => skipped(r) && sameArgs(pushes(r)[0], makeArgs('d'.repeat(40))));
await t('ci proof: a leftover land ref with the merge tree and the parents swapped is overwritten (dco reads the order)', ci({ there: 'd'.repeat(40), thereTree: TREE, thereParents: [HEAD, MAIN] }), (r) => skipped(r) && sameArgs(pushes(r)[0], makeArgs('d'.repeat(40))));
await t('ci proof: a leftover land ref with the merge tree and one parent only is overwritten', ci({ there: 'd'.repeat(40), thereTree: TREE, thereParents: [MAIN] }), (r) => skipped(r) && sameArgs(pushes(r)[0], makeArgs('d'.repeat(40))));
await t('ci proof: a leftover land ref with the merge tree and a third parent is overwritten', ci({ there: 'd'.repeat(40), thereTree: TREE, thereParents: [MAIN, HEAD, OTHER] }), (r) => skipped(r) && sameArgs(pushes(r)[0], makeArgs('d'.repeat(40))));
await t('ci proof: a leftover land ref whose commit cannot be read pushes nothing and runs the local preflight', ci({ there: 'd'.repeat(40), thereUnreadable: true }), (r) => local(r, /could not push/) && pushes(r).length === 0);
await t('ci proof: an interrupt (SIGINT or SIGTERM exit) while waiting deletes the land ref at once, once, and leaves no listener behind', { ...ci({ polls: [[pending]] }), onSleep: (w) => { if (w.sleeps.length === 1) { process.emit('exit', 143); w.atInterrupt = w.calls.filter((c) => c.cmd === 'git' && sameArgs(c.args, DEL)).length; } } }, (r) => r.w.atInterrupt === 1 && r.w.sleeps.length > 1 && pushes(r).filter((a) => sameArgs(a, DEL)).length === 1 && r.w.exitListenersLeft === 0);
await t('ci proof: an interrupt deletes only a refs/heads/land/<n> ref', { ...ci({ polls: [[pending]] }), onSleep: (w) => { if (w.sleeps.length === 1) { process.emit('exit', 130); w.atInterrupt = w.calls.filter((c) => c.cmd === 'git' && sameArgs(c.args, DEL)).length; } } }, (r) => r.w.atInterrupt === 1 && pushes(r).every((a) => !a.includes('--delete') || sameArgs(a, DEL)));
await t('ci proof: no listener is left on exit after a normal land, a dry run or a refusal', ci(), (r) => r.w.exitListenersLeft === 0);
await t('ci proof: the merged tree not the proven one (main moved under the merge) is recorded TREE-MISMATCH on the CI path, and refused naming it', { ...ci(), mergedTree: OTHER }, (r) => r.merge && r.w.spawns.length === 0 && r.ledgerNow.includes(`TREE-MISMATCH tested ${TREE} merged ${OTHER}`) && (r.error ?? '').includes('is not the tested merge result'));
await t('ci proof: the merged tree equal to the proven one carries no TREE-MISMATCH and no warning', ci(), (r) => r.merge && !r.ledgerNow.includes('TREE-MISMATCH') && !r.said.includes('is not the tested merge result'));
await t('ci proof: a push that fails runs the local preflight and deletes nothing', ci({ pushFails: true }), (r) => local(r, /could not push/) && pushes(r).length === 1);
await t('ci proof: unreadable check runs run the local preflight', ci({ runsFail: true }), (r) => local(r, /CI could not be read/) && lastIs(r, DEL));
await t('ci proof: a ruleset that cannot be read pushes nothing and runs the local preflight', ci({ rules: null }), (r) => local(r, /required checks could not be read/) && pushes(r).length === 0);
await t('ci proof: a ruleset with no required checks pushes nothing', ci({ rules: '{"type":"pull_request"}' }), (r) => local(r, /required checks could not be read/) && pushes(r).length === 0);
await t('ci proof: a land commit that cannot be built runs the local preflight, pushing nothing', ci({ noCommit: true }), (r) => local(r, /could not build the land commit/) && pushes(r).length === 0);
await t('ci proof: a failed delete is said and the land still lands', ci({ deleteFails: true }), (r) => skipped(r) && /could not delete refs\/heads\/land\/5/.test(r.said));
await t('ci proof: --dry-run builds the commit, pushes and deletes nothing, and says what it would do', { ...ci(), argv: ['5', '--dry-run'] }, (r) => !r.merge && pushes(r).length === 0 && argv(r, 'git', 'commit-tree').length === 1 && r.w.sleeps.length === 0 && r.said.includes(`dry run: would push ${LAND.slice(0, 8)} (tree ${TREE.slice(0, 8)}) to ${LREF}`));
await t('ci proof: --preflight-only never builds, pushes or waits', { ...ci(), argv: PO }, (r) => pushes(r).length === 0 && argv(r, 'git', 'commit-tree').length === 0 && r.w.sleeps.length === 0 && r.w.spawns.length === 1);
await t('ci proof: no CI read scripted (the default) runs the local preflight and the note says so', {}, (r) => local(r, /local preflight \(the required checks could not be read[^)]*\): merge result preflight green/) && pushes(r).length === 0);
await t('ci proof: a failing local preflight still refuses after a CI read that did not prove', { ...ci({ polls: [[crun({ conclusion: 'failure' })]] }), initCode: 1 }, (r) => no(r, /baseline preflight\) failed/) && lastIs(r, DEL));
cases.push(['ci proof: across every scenario, every git push is a create-with-lease or a delete of refs/heads/land/5, nothing else', ALL.filter((c) => c.cmd === 'git' && c.args[0] === 'push').every((c) => { const a = c.args; return (a.length === 5 && a[2].startsWith(`--force-with-lease=${LREF}:`) && a[3] === 'origin' && a[4] === `${LAND}:${LREF}`) || sameArgs(a, DEL); }), true]);
cases.push(['ci proof: across every scenario, every CI read is GET only', ALL.filter((c) => c.cmd === 'gh' && /rules\/branches|git\/commits|check-runs|actions\/runs/.test(c.args.join(' '))).every((c) => !c.args.some((a) => ['-X', '--method', '-f', '-F', '--input'].includes(a))), true]);
cases.push(['every gh call across all scenarios names the repo', ALL.filter((c) => c.cmd === 'gh').every((c) => c.args.includes(REPO) || c.args.some((a) => a.startsWith(`repos/${REPO}/`)) || (c.args[1] === 'graphql' && c.args.includes('owner=Team-Commonly') && c.args.includes('name=jellymoon'))), true]);
await t('a draft PR is refused before any push of a land ref, nothing merged', { ...ci(), pr: { isDraft: true } }, (r) => no(r, /#5 is a draft \(nothing merged\)/) && pushes(r).length === 0);
const bad = cases.filter(([, got, want]) => got !== want);
for (const [n] of bad) console.error(`  ${n}`);
if (bad.length) { console.error(`check-land-e2e: ${bad.length} of ${cases.length} wrong`); process.exit(1); }
console.log(`check-land-e2e: ${cases.length} cases ok`);
