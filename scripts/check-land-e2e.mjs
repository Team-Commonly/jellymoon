#!/usr/bin/env bun
// End-to-end land cases: main() from land.mjs runs against a scripted fake of git, gh, make, the pod and the preflight child. The fake
// answers by argv pattern and records every call, so each case asserts the refusal AND the argv that proves which guard fired.
import { EventEmitter } from 'node:events';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { main } from './land.mjs';

const [MAIN, HEAD, TREE, MERGED, OTHER] = ['1', '2', '3', '4', '5'].map((c) => c.repeat(40));
const REPO = 'Team-Commonly/jellymoon';
const HDR = 'pr\thead\tverdict\tby\twhen\tnote\n';
const rowOf = (verdict, by = 'opus-verifier', head = HEAD) => `5\t${head}\t${verdict}\t${by}\t2026-10-29T10:00:00Z\tn\n`;
const PROT = ':100644 100644 aaa bbb M\tscripts/x.mjs';
const NOTE = { userId: { username: 'ada' }, content: `jellymoon#5 ${HEAD}\nok\nGATE: PASS`, createdAt: '2026-10-29T10:01:00Z' };
const fail = new Error('scripted failure');

const ALL = [];
async function land(over = {}) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'land-e2e-'));
  const [top, common, cwd] = ['top', 'common', 'cwd'].map((d) => { fs.mkdirSync(path.join(dir, d)); return path.join(dir, d); });
  const ledger = path.join(dir, 'ledger.tsv');
  fs.writeFileSync(ledger, over.ledger ?? HDR + rowOf('PASS'));
  fs.writeFileSync(path.join(dir, 'token'), 'jmu_FAKE-token-abc123\n');
  (over.topFiles || []).forEach((f) => fs.writeFileSync(path.join(top, f), ''));
  (over.cwdFiles || []).forEach((f) => fs.writeFileSync(path.join(cwd, f), ''));
  const w = { calls: [], fetches: [], pod: [], spawns: [], views: 0, ledger, lockAtMerge: null, scanned: [] };
  const base = { url: `https://github.com/${REPO}/pull/5`, number: 5, state: 'OPEN', baseRefName: 'main', headRefName: 'feat', headRefOid: HEAD, createdAt: '2026-10-29T09:00:00Z', isCrossRepository: false, author: { login: 'lily' }, mergeCommit: { oid: MERGED }, ...over.pr };
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
    [/^gh api .*collaborators/, (a) => ['{"login":"lily","push":true}', ...(a.includes('--paginate') ? over.page2 || [] : [])].join('\n')],
    [/^gh api .*check-runs/, over.runs ?? '{"total":1,"runs":[{"name":"ci","conclusion":"success"}]}'],
    [/^git merge-tree --write-tree/, `${TREE}\n`], [/--raw -M/, over.raw ?? ':100644 100644 aaa bbb M\tdocs/a.md'], [/--numstat/, over.numstat ?? '1\t1\tdocs/a.md'], [/ -U0 /, ''], [/^git .*log -p/, ''], [/^git log --format=%B/, over.messages ?? ''],
    [/^git ls-tree/, 'README.md\0'], [/^make -s secrets-range/, ''],
    [/^make -s secrets-dir DIR=(\S+)/, (a, s) => { const d = a[3].slice(4); s.scanned = fs.readdirSync(d).map((f) => [f, fs.readFileSync(path.join(d, f), 'utf8')]); if (over.dirHit) throw fail; return ''; }],
    [/^git cat-file blob/, over.blob ?? 'hello'], [/^git cat-file -e/, ''],
    [new RegExp(`^git show ${MAIN}:.github/opus-reviewers.txt`), 'Ada ada'], [/^git show \S+:.github\/opus-reviewers.txt/, 'Mallory mallory'],
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
  const spawn = (cmd, args, opts) => {
    w.spawns.push({ cmd, args, opts });
    const c = Object.assign(new EventEmitter(), { stdout: new EventEmitter(), stderr: new EventEmitter(), pid: 2000000001 });
    setImmediate(() => { over.onSpawn?.(w); c.emit('close', over.initCode ?? 0); });
    return c;
  };
  const fetch = async (url, o) => { w.pod.push({ url, o }); return { ok: true, status: 200, json: async () => (url.includes('before=') ? [] : (over.msgs ?? [NOTE])) }; };
  const env = { JM_REPO: REPO, JM_TRUSTED_LOGINS: 'lily', JM_LEDGER: ledger, JM_COMMONLY_API: 'https://api.commonly.me', JM_COMMONLY_TOKEN_FILE: path.join(dir, 'token'), JM_VERDICT_POD: 'pod1', ...over.env };
  const [log, err] = [console.log, console.error];
  const said = [];
  console.log = (m) => said.push(m); console.error = (m) => said.push(m);
  let error = null;
  try { await main({ exec, fetch, spawn, env, argv: over.argv ?? ['5'], nodeVersion: over.node ?? '22.0.0', cwd }); } catch (e) { error = e.message; } finally { console.log = log; console.error = err; }
  const merge = w.calls.find((c) => c.cmd === 'gh' && c.args[1] === 'merge');
  const res = { error, w, merge, said: said.join('\n'), ledgerNow: fs.readFileSync(ledger, 'utf8'), ledgerWas: over.ledger ?? HDR + rowOf('PASS'), lock: fs.existsSync(path.join(common, 'jm-land.lock')) };
  fs.rmSync(dir, { recursive: true, force: true });
  ALL.push(...w.calls);
  return res;
}
// Each case runs land once; `no` is: refused with this reason, nothing merged.
const no = (r, re) => !r.merge && re.test(r.error ?? '');
const argv = (r, cmd, sub) => r.w.calls.filter((c) => c.cmd === cmd && [cmd, ...c.args].join(' ').includes(sub)).map((c) => c.args.join(' '));
const fetched = (r) => r.w.calls.some((c) => c.args[0] === 'fetch');
const cases = [];
const t = async (name, over, f) => { cases.push([name, !!f(await land(over)), true]); };
const mergeArgs = `pr merge 5 --repo ${REPO} --merge --match-head-commit ${HEAD}`;
const dirty = [[/status --porcelain/, ' M x']];
const PO = ['5', '--preflight-only'];
await t('happy path: lands with --merge and --match-head-commit <head>; the ledger gets LANDED', {}, (r) => r.merge?.args.join(' ') === mergeArgs && r.ledgerNow.includes('\tLANDED\t') && r.lock === false && r.w.lockAtMerge === true);
await t('happy path: pr view asks for isCrossRepository; the collaborators call is paginated and unfiltered', {}, (r) => argv(r, 'gh', 'pr view')[0].includes('isCrossRepository') && argv(r, 'gh', 'collaborators')[0].includes('--paginate') && !argv(r, 'gh', 'collaborators')[0].includes('permission='));
await t('every main fetch names refs/heads/main; the merge disables hooks; untracked files are shown in full', {}, (r) => r.w.fetches.filter((f) => !f.startsWith('pull/') && f !== MERGED).every((f) => f === 'refs/heads/main') && argv(r, 'git', 'merge --no-ff')[0].includes('core.hooksPath=/dev/null') && argv(r, 'git', 'status')[0].includes('status.showUntrackedFiles=all'));
await t('diffs run with external diff and textconv off; the history scan covers merge commits (-m)', {}, (r) => argv(r, 'git', '--raw').every((a) => a.includes('diff.external= diff --no-ext-diff --no-textconv')) && argv(r, 'git', ' log -p').every((a) => a.includes('--no-ext-diff --no-textconv') && a.includes(' -m ')));
await t('the secret range is <main tip>..<head>', {}, (r) => argv(r, 'make', 'secrets-range')[0] === `-s secrets-range RANGE=${MAIN}..${HEAD}`);
await t('the preflight child: stdin ignored, own session, env -i, HOME and TMPDIR inside the throwaway, entropy base is main\'s tip', {}, (r) => {
  const [sp] = r.w.spawns; const home = sp.args.find((a) => a.startsWith('HOME=')).slice(5); const tmp = sp.args.find((a) => a.startsWith('TMPDIR=')).slice(7);
  return sp.opts.stdio[0] === 'ignore' && sp.opts.detached === true && sp.args[0] === '-i' && path.dirname(home) === path.dirname(tmp) && home.includes('jm-land-') && sp.args.includes(`JM_ENTROPY_BASE=${MAIN}`) && sp.args.includes('PATH=/usr/bin:/bin:/usr/sbin:/sbin');
});
await t('untrusted author: refused before any fetch', { pr: { author: { login: 'mallory' } } }, (r) => no(r, /not trusted/) && !fetched(r));
await t('untrusted author in --preflight-only: refused before any fetch', { argv: PO, pr: { author: { login: 'mallory' } } }, (r) => no(r, /not trusted/) && !fetched(r));
await t('cross-repo PR: refused', { pr: { isCrossRepository: true } }, (r) => no(r, /cross-repository/));
await t('isCrossRepository missing from the reply: refused', { omit: ['isCrossRepository'] }, (r) => no(r, /not known to be same-repo/));
await t('a stranger with push on page 2 of the collaborators: refused', { page2: ['{"login":"eve","push":true}'] }, (r) => no(r, /not trusted: eve/));
await t('no ledger PASS: refused', { ledger: HDR }, (r) => no(r, /./));
await t('a CI failure on the head: refused', { runs: '{"total":1,"runs":[{"name":"ci","conclusion":"failure"}]}' }, (r) => no(r, /check run failed/));
await t('a protected PR without a pod verdict: refused', { raw: PROT, msgs: [] }, (r) => no(r, /./));
await t('a protected PR with a verdict lands; reviewers were read from main\'s tip', { raw: PROT }, (r) => r.merge && argv(r, 'git', 'git show ')[0] === `show ${MAIN}:.github/opus-reviewers.txt`);
await t('a verdict by a reviewer only the PR lists is refused', { raw: PROT, msgs: [{ ...NOTE, userId: { username: 'mallory' } }] }, (r) => no(r, /./));
await t('a secret hit in the range needs the verdict', { msgs: [], on: [[/^make -s secrets-range/, () => { throw fail; }]] }, (r) => no(r, /./));
await t('a red preflight: refused', { initCode: 1 }, (r) => no(r, /init\.sh/));
await t('a red preflight in --preflight-only: refused', { argv: PO, initCode: 1 }, (r) => no(r, /init\.sh/));
await t('the PR head differs from the local fetch in --preflight-only: refused', { argv: PO, pullHead: OTHER }, (r) => no(r, /differs from the local fetch/));
await t('the throwaway worktree is removed on a normal return, not left for process exit', {}, (r) => argv(r, 'git', 'worktree remove').length === 1 && r.w.calls.findIndex((c) => c.args.includes('remove')) > -1);
await t('the PR head differs from the local fetch: refused', { pullHead: OTHER }, (r) => no(r, /differs from the local fetch/));
await t('the ledger changes during the preflight: the second decide refuses', { onSpawn: (w) => fs.appendFileSync(w.ledger, rowOf('FAIL', 'sonnet-verifier')) }, (r) => no(r, /changed before the merge/));
await t('the head moves on GitHub before the merge: refused', { fresh: { headRefOid: OTHER } }, (r) => no(r, /changed before the merge/));
await t('the author changes before the merge: the trust gate runs again', { fresh: { author: { login: 'mallory' } } }, (r) => no(r, /not trusted/));
await t('main moves during the run: refused', { mainTips: [MAIN, OTHER] }, (r) => no(r, /main moved/));
await t('dry run: no merge, ledger byte-identical, preflight still run', { argv: ['5', '--dry-run'] }, (r) => !r.merge && r.ledgerNow === r.ledgerWas && r.error === null && r.w.spawns.length === 1);
await t('dry run on a PR merged out of band: nothing written', { argv: ['5', '--dry-run'], pr: { state: 'MERGED' }, ledger: HDR }, (r) => !r.merge && r.ledgerNow === r.ledgerWas && r.said.includes('dry run'));
await t('a PR merged out of band is recorded MERGED-UNVERIFIED, nothing merged', { pr: { state: 'MERGED' }, ledger: HDR }, (r) => !r.merge && r.ledgerNow.includes('MERGED-UNVERIFIED'));
await t('the merged commit\'s tree differs from the tested one: TREE-MISMATCH on the LANDED row', { mergedTree: OTHER }, (r) => r.ledgerNow.includes('TREE-MISMATCH'));
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
await t('bootstrap: refused once main has scripts/land.mjs, in both paths', { argv: ['5', '--bootstrap'] }, (r) => no(r, /only for the first land/));
await t('bootstrap (preflight-only): refused once main has scripts/land.mjs', { argv: [...PO, '--bootstrap'] }, (r) => no(r, /only for the first land/));
const boot = { argv: ['5', '--bootstrap'], checkoutHead: OTHER, on: [[/^git cat-file -e \S+:(scripts\/land\.mjs|entropy\.json)/, () => { throw fail; }]] };
await t('bootstrap on a main without land.mjs: reviewers come from the head, entropy base is none, a BOOTSTRAP row is written', { ...boot, raw: PROT, msgs: [{ ...NOTE, userId: { username: 'mallory' } }] }, (r) => r.merge && r.w.spawns[0].args.includes('JM_ENTROPY_BASE=none') && r.ledgerNow.includes('\tBOOTSTRAP\t'));
await t('API origin that is not commonly: refused', { env: { JM_COMMONLY_API: 'https://evil.example' } }, (r) => no(r, /./) && r.w.calls.length === 0);
await t('a PR url for another repo: refused', { pr: { url: 'https://github.com/Other/repo/pull/5' } }, (r) => no(r, /resolved to/));
await t('--dry-run with --preflight-only: refused', { argv: [...PO, '--dry-run'] }, (r) => no(r, /usage/));
await t('two PR numbers: refused', { argv: ['5', '6'] }, (r) => no(r, /usage/));
await t('an old Node: refused before anything runs', { node: '18.0.0' }, (r) => no(r, /Node 20/) && r.w.calls.length === 0);
await t('preflight-only: no ledger, no pod, no merge', { argv: PO }, (r) => r.error === null && r.w.pod.length === 0 && !r.merge && r.ledgerNow === r.ledgerWas);
await t('the worktree merge tree must equal the decision\'s merge-tree result', { wtTree: OTHER }, (r) => no(r, /not the merge result the decision used/) && r.w.spawns.length === 0);
await t('a binary change (numstat -\\t-) needs the verdict', { numstat: '-\t-\tdocs/a.md', msgs: [] }, (r) => no(r, /./));
await t('inert files are scanned under neutral names with their content', {}, (r) => r.w.scanned.length === 1 && r.w.scanned[0][0] === '0.txt' && r.w.scanned[0][1] === 'hello');
await t('commit messages in the range are scanned under neutral names too', { messages: 'subject\nbody' }, (r) => r.w.scanned.length === 2 && r.w.scanned[1][1] === 'subject\nbody' && argv(r, 'git', 'log --format=%B')[0].endsWith(`${MAIN}..${HEAD}`));
await t('the throwaway worktree is added with repo hooks off', {}, (r) => argv(r, 'git', 'worktree add')[0].startsWith('-c core.hooksPath=/dev/null worktree add'));
await t('a hit from the neutral-name scan needs the verdict', { dirHit: true, msgs: [] }, (r) => no(r, /./));
await t('a branch name with escapes is stripped from what land prints', { pr: { headRefName: 'a\u001b[31mb\u009bc' } }, (r) => r.said.includes('branch a[31mbc ') && !/[\u001b\u009b]/.test(r.said));
await t('the bearer token never appears in a refusal', { raw: PROT, on: [[/^git show \S+:.github/, () => { throw new Error('boom jmu_FAKE-token-abc123'); }]] }, (r) => !r.merge && r.error !== null && !r.error.includes('abc123'));
cases.push(['every gh call across all scenarios names the repo', ALL.filter((c) => c.cmd === 'gh').every((c) => c.args.includes(REPO) || c.args.some((a) => a.startsWith(`repos/${REPO}/`))), true]);
const bad = cases.filter(([, got, want]) => got !== want);
for (const [n] of bad) console.error(`  ${n}`);
if (bad.length) { console.error(`check-land-e2e: ${bad.length} of ${cases.length} wrong`); process.exit(1); }
console.log(`check-land-e2e: ${cases.length} cases ok`);
