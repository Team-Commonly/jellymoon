// The scripted fake of git, gh, gitleaks, the pod and the preflight child that check-land-e2e.mjs and check-land-batch-e2e.mjs run main() against: land() runs one land and returns what it saw.
import { EventEmitter } from 'node:events';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { main } from './land.mjs';

export const [MAIN, HEAD, TREE, MERGED, OTHER] = ['1', '2', '3', '4', '5'].map((c) => c.repeat(40));
export const REPO = 'Team-Commonly/jellymoon';
export const HDR = 'pr\thead\tverdict\tby\twhen\tnote\n';
export const rowOf = (verdict, by = 'opus-verifier', head = HEAD) => `5\t${head}\t${verdict}\t${by}\t2026-10-29T10:00:00Z\tn\n`;
export const NOTE = { userId: { _id: 'id-orion', username: 'orion' }, content: `jellymoon#5 ${HEAD}\nok\nGATE: PASS`, createdAt: '2026-10-29T10:01:00Z' };
export const fail = new Error('scripted failure');
// What a clean gitleaks run prints to stderr (it colours its log even into a pipe): the scanned count and "no leaks found".
export const cleanScan = (n) => `\x1b[90m10:11AM\x1b[0m \x1b[32mINF\x1b[0m \x1b[1m${n} commits scanned.\x1b[0m\n10:11AM INF scanned ~6 bytes (6 bytes) in 37.9ms\n\x1b[90m10:11AM\x1b[0m \x1b[32mINF\x1b[0m \x1b[1mno leaks found\x1b[0m\n`;

// The scripted range scan: a throw is a failed scan, a return is a zero exit with that stderr. runOf turns either into { status, stdout, stderr }.
const scanCall = (over) => {
  const bad = { hit: [2, 'WRN leaks found: 1\nexit status 3\n'], killed: [null, 'exit status 3\n'], killedclean: [null, cleanScan(3)], fail: [2, 'go: no network\nexit status 1\n'] }[over.scan];
  if (bad) throw Object.assign(new Error('scan failed'), { status: bad[0], stderr: bad[1] });
  return { status: 0, stderr: over.scanErr ?? cleanScan(over.scanned ?? (over.count ?? 3) - (over.bare ?? 0)) };
};
const runOf = (f) => { try { const v = f(); return typeof v === 'object' ? { stdout: '', ...v } : { status: 0, stdout: v, stderr: '' }; } catch (e) { return { status: typeof e.status === 'number' ? e.status : null, stdout: '', stderr: String(e.stderr || '') }; } };

// The commits of <main tip>..<head>; any other range holds one.
export const HEADS = [HEAD]; // every PR head a range may end at (the batch cases add theirs)
export const commitsOf = (range, n = 3) => Array.from({ length: HEADS.some((h) => range.endsWith(`..${h}`)) && !range.startsWith(HEAD) ? n : 1 }, (_, i) => `c${i}`.padEnd(40, '0'));
export const ALL = [];
// What the scan's config and ignore paths held while it ran (land removes them after): read from the argv land passed.
const seenOf = (a) => { const [config, ignore] = ['--config', '--gitleaks-ignore-path'].map((f) => a[a.indexOf(f) + 1]); return { config, ignore, text: fs.readFileSync(config, 'utf8'), entries: fs.readdirSync(ignore) }; };
const seedGo = (top) => { fs.mkdirSync(path.join(top, '.tools', 'bin'), { recursive: true }); fs.writeFileSync(path.join(top, '.tools', 'bin', 'go'), '', { mode: 0o755 }); }; // the checkout's own go
export async function land(over = {}) {
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
