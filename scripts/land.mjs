#!/usr/bin/env node
// make land PR=<n>: the only merge path (docs/development/landing.md). Thin shell-outs to git, gh and the pod API;
// every decision is a pure function in land-decide.mjs, tested by check-land.mjs. Read-only unless merging:
// --dry-run writes no ledger row. PR code runs here only after the trust gate (checkTrust); the preflight is a
// correctness check, not a security boundary.
//   land.mjs <pr> [--dry-run] [--bootstrap]
//   land.mjs <pr> --preflight-only [--bootstrap]   (verifiers: the trust gate and the merge-result preflight only; needs JM_REPO and JM_TRUSTED_LOGINS)
// Run it from a clean checkout of main: the accepted reviewers come from operator env, never from the repo.
import { execFileSync, spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {
  acquireLock, appendLedger as writeLedger, checkApi, checkCheckout, checkNode, checkNoBunConfig, checkHead, checkLedger, checkMain, checkPodVerdict, checkRuns, landedNote, mergeAction,
  checkTrust, cursorOf, entropyBase, landAction, parseLedger, parseLogins, pushersOf, trackThrowaway, mergeTree, quoteOutput, stripControls, parseReviewers, readToken, remoteFor,
  scrub, takePage,
} from './land-decide.mjs';
const REQUIRED = ['JM_REPO', 'JM_TRUSTED_LOGINS', 'JM_LEDGER', 'JM_COMMONLY_API', 'JM_COMMONLY_TOKEN_FILE', 'JM_VERDICT_POD', 'JM_VERDICT_REVIEWERS'];
// All I/O goes through one seam, `deps`, so check-land-e2e.mjs can run main() against a scripted fake: exec(cmd, args, opts) for every
// git, gh and make call, fetch for the pod read, spawn for the preflight child, env, argv and the Node version.
export class Refusal extends Error {}
let IO;
let REPO = ''; // every gh call names it: a worktree can carry remotes for several repos
let DRY = false;
let SECRET = ''; // the bearer token once read; scrubbed from anything printed
const out = (cmd, args, opts = {}) => IO.exec(cmd, args, opts);
const gh = (...args) => out('gh', args);
const say = (m) => console.log(stripControls(m)); // every string land prints is stripped: GitHub text (a branch name, a url) can carry escapes
const refuse = (m) => { throw new Refusal(stripControls(scrub(m, SECRET))); };
const readLedger = () => parseLedger(fs.existsSync(IO.env.JM_LEDGER) ? fs.readFileSync(IO.env.JM_LEDGER, 'utf8') : '');
const appendLedger = (pr, head, verdict, by, note) => writeLedger(IO.env.JM_LEDGER, [pr, head, verdict, by, new Date().toISOString(), note], { dry: DRY });
function viewPr(n) {
  const pr = JSON.parse(gh('pr', 'view', String(n), '--repo', REPO, '--json', 'url,number,state,baseRefName,headRefName,headRefOid,createdAt,isCrossRepository,author'));
  if (pr.url !== `https://github.com/${REPO}/pull/${n}`) refuse(`#${n} resolved to ${pr.url}, not ${REPO}`);
  return pr;
}
// One land at a time per clone (two clones can land at once): an mkdir lock in the git common dir; see acquireLock.
function lock() {
  const dir = path.join(path.resolve(out('git', ['rev-parse', '--git-common-dir'])), 'jm-land.lock');
  const got = acquireLock(dir);
  if (!got.ok) refuse(got.reason);
  const free = () => fs.rmSync(dir, { recursive: true, force: true });
  process.on('exit', free);
  return () => { process.off('exit', free); free(); };
}
// Fetches main and (optionally) the PR head from the one remote that is JM_REPO.
const MAIN_REF = 'refs/heads/main'; // never bare `main`: a tag named main wins git's DWIM
function fetchRef(ref) {
  const m = remoteFor(out('git', ['remote', '-v']), REPO);
  if (m.error) refuse(m.error);
  out('git', ['fetch', '--quiet', m.remote, ref]);
  return out('git', ['rev-parse', 'FETCH_HEAD']);
}
const why = (e) => scrub(String(e.stderr || e.message).split('\n').find(Boolean) || 'unknown', SECRET);
const BASE = ['-c', 'core.hooksPath=/dev/null', '-c', 'user.name=land', '-c', 'user.email=land@localhost'];
// Runs ./init.sh (which runs make preflight) in the throwaway worktree: env -i, stdin /dev/null, a NEW session (no
// controlling tty), output captured and quoted, a wall-clock limit, and the whole process group killed when it returns.
const LIMIT_MS = 30 * 60 * 1000;
function runInit(wt, env) {
  return new Promise((done) => {
    let text = '';
    let over = false;
    let ended = false;
    const child = IO.spawn('env', ['-i', ...Object.entries(env).map(([k, v]) => `${k}=${v}`), 'bash', '-c', './init.sh'], { cwd: wt, detached: true, stdio: ['ignore', 'pipe', 'pipe'] });
    const take = (b) => { if (text.length < 2e6) text += b.toString('utf8'); };
    child.stdout.on('data', take);
    child.stderr.on('data', take);
    const kill = () => { try { IO.kill(-child.pid, 'SIGKILL'); } catch { /* the group is already gone */ } };
    const grace = IO.graceMs ?? 5000;
    let late;
    process.prependListener('exit', kill); // a crash or Ctrl-C of land must not leave the group running, and it dies before its dir is removed
    const timer = setTimeout(() => { over = true; kill(); late = setTimeout(() => end(-1), grace); }, IO.limitMs ?? LIMIT_MS); // an escapee holding the pipes cannot stall us
    const end = (code) => {
      if (ended) return;
      ended = true;
      clearTimeout(timer);
      clearTimeout(late);
      kill();
      process.off('exit', kill);
      if (text) console.error(scrub(quoteOutput(text), SECRET));
      if (over) return done({ ok: false, reason: `./init.sh did not finish in ${LIMIT_MS / 60000} minutes; killed` });
      return done(code === 0 ? { ok: true } : { ok: false, reason: 'clean-shell ./init.sh (baseline preflight) failed on the merge result' });
    };
    child.on('error', () => end(-1));
    // 'exit' is the child itself; 'close' waits for every holder of its pipes. Kill the group at exit so a descendant holding a pipe cannot stall land.
    child.on('exit', (code) => { kill(); late = setTimeout(() => end(code ?? -1), grace); });
    child.on('close', end);
  });
}
// Throwaway worktree at main's tip, the PR head merged in as GitHub would (no repo hooks), then ./init.sh under env -i with
// HOME and TMPDIR inside the throwaway dir and system dirs only on PATH. Hygiene, not isolation.
async function preflight(mainTip, head, bootstrap, expectTree) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'jm-land-'));
  const wt = path.join(dir, 'wt');
  for (const d of ['home', 'tmp']) fs.mkdirSync(path.join(dir, d));
  const release = trackThrowaway(dir, wt, (c, a) => out(c, a));
  const env = { HOME: path.join(dir, 'home'), TMPDIR: path.join(dir, 'tmp'), PATH: '/usr/bin:/bin:/usr/sbin:/sbin', TERM: 'dumb', JM_ENTROPY_BASE: entropyBase(mainTip, { bootstrap, mainHasEntropy: has(mainTip, 'entropy.json') }) }; // main's tip as fetched, never a ref the PR could move
  let result = { ok: true };
  let tree = '';
  try {
    try { out('git', ['-c', 'core.hooksPath=/dev/null', 'worktree', 'add', '--quiet', '--detach', wt, mainTip]); } catch (e) { result = { ok: false, reason: `git worktree add failed: ${why(e)}` }; }
    if (result.ok) try { out('git', ['-C', wt, ...BASE, 'merge', '--no-ff', '--no-edit', head]); } catch {
      result = { ok: false, reason: `merging ${head.slice(0, 8)} into main ${mainTip.slice(0, 8)} conflicts` };
    }
    if (result.ok) tree = out('git', ['-C', wt, 'rev-parse', 'HEAD^{tree}']);
    if (result.ok && expectTree && tree !== expectTree) result = { ok: false, reason: `the preflight's merge (tree ${tree.slice(0, 8)}) is not the merge result the decision used (${expectTree.slice(0, 8)})` };
    if (result.ok) result = await runInit(wt, env);
  } catch (e) { result = { ok: false, reason: `preflight failed: ${why(e)}` }; }
  release();
  return { ...result, tree };
}
// Pod messages newest-first, paged back past `since` (ms), deduped by id. Fails closed if the page cap is hit first,
// or if a page boundary cannot be paged past (see takePage).
async function podMessages(since) {
  SECRET = readToken(IO.env.JM_COMMONLY_TOKEN_FILE);
  const all = [];
  const seen = new Set();
  let before = '';
  for (let page = 0; page < 30; page += 1) {
    const url = `${IO.env.JM_COMMONLY_API}/api/messages/${IO.env.JM_VERDICT_POD}?limit=200${before}`;
    let res;
    try { res = await IO.fetch(url, { headers: { Authorization: `Bearer ${SECRET}` } }); } catch (e) { return refuse(`pod messages: request failed (${e.name})`); }
    if (!res.ok) refuse(`pod messages: HTTP ${res.status}`);
    let msgs;
    try { msgs = await res.json(); } catch (e) { return refuse(`pod messages: unreadable reply (${e.name})`); }
    if (!Array.isArray(msgs)) refuse('pod messages: the reply is not a list of messages');
    const { fresh, stuck } = takePage(seen, msgs, 200);
    if (stuck) refuse('pod messages: a page boundary could hide a message; cannot prove no older FAIL');
    if (!fresh.length) return all;
    all.push(...fresh);
    const cursor = cursorOf(msgs);
    if (!cursor) refuse('pod messages: a message has no timestamp; cannot page');
    if (Date.parse(cursor) - 1 < since) return all;
    before = `&before=${encodeURIComponent(cursor)}`;
  }
  return refuse('pod messages: page cap reached before paging back past the PR\'s creation; cannot prove no older FAIL');
}
const has = (ref, file) => { try { out('git', ['cat-file', '-e', `${ref}:${file}`]); return true; } catch { return false; } };
// The first land of the gate is --bootstrap and only while main has no land.mjs; after that --bootstrap is refused. Both paths.
function bootstrapGate(mainTip, bootstrap) {
  if (has(mainTip, 'scripts/land.mjs') === bootstrap) refuse(bootstrap ? 'main already has scripts/land.mjs; --bootstrap is only for the first land of the gate' : 'main has no scripts/land.mjs yet; the first land of the gate needs --bootstrap and is judged by people');
}
// Trust gate: runs before anything of the PR is fetched or executed. Trust is GitHub identity: not cross-repository, the
// author and every account with push access are in JM_TRUSTED_LOGINS (an operator env var; a PR cannot widen it).
function trustGate(pr) {
  const pushers = pushersOf(gh('api', '--paginate', `repos/${REPO}/collaborators?affiliation=all&per_page=100`, '--jq', '.[] | {login, push: .permissions.push}'));
  const t = checkTrust({ pr: { isCrossRepository: pr.isCrossRepository, author: pr.author?.login }, pushers, trusted: parseLogins(IO.env.JM_TRUSTED_LOGINS) });
  if (!t.ok) refuse(`#${pr.number}: ${t.reason}`);
}
// Accepted reviewers, from operator env only: checked before the token is read or the pod fetched. Unset is caught with the other
// variables; a value with no valid `<id>:<Name>` entry is malformed.
const reviewers = () => {
  const list = parseReviewers(IO.env.JM_VERDICT_REVIEWERS);
  if (!list.length) refuse('JM_VERDICT_REVIEWERS is malformed: it has no valid <id>:<Name> entry');
  return list;
};
// The commit-range secret scan (gitleaks git mode, -m, markers ignored). It only refuses. A finding exits gitleaks with 3, which `go run`
// reports as "exit status 3" on stderr; any other failure of the scan refuses too, because a scan that did not run proves nothing.
const rangeScan = (mainTip, head) => {
  try { out('make', ['-s', 'secrets-range', `RANGE=${mainTip}..${head}`]); return { ok: true }; } catch (e) {
    return { ok: false, reason: /^exit status 3$/m.test(String(e.stderr || '')) ? 'a secret was found in the range; remove it from history and push a new head' : 'the range secret scan failed to run; it proves nothing, so land refuses' };
  }
};
// Every ledger, CI and pod check at this moment. Run before the preflight and again right before the merge.
async function decide(pr, mainTip) { // mainTip is the tip fetched for this land
  const n = String(pr.number);
  const head = pr.headRefOid;
  const rows = readLedger();
  const led = checkLedger(rows, { pr: n, head });
  if (!led.ok) return led;
  const notes = [`ledger PASS by ${led.primary.by}`];
  const runs = JSON.parse(gh('api', `repos/${REPO}/commits/${head}/check-runs?per_page=100`, '--jq', '{total: .total_count, runs: [.check_runs[] | {name, conclusion}]}'));
  const ci = checkRuns(runs.runs, runs.total);
  if (!ci.ok) return ci;
  // The secret scan can only REFUSE; a clean scan waives nothing. Every land needs the ledger PASS above and the reviewer's verdict below.
  const scan = rangeScan(mainTip, head);
  if (!scan.ok) return scan;
  const gate = checkPodVerdict(await podMessages(Date.parse(pr.createdAt)), { pr: n, head, entries: reviewers() });
  if (!gate.ok) return gate;
  notes.push(gate.reason);
  const tree = treeOf(mainTip, head);
  return { ok: true, notes, tree };
}
// The tree `merge-tree --write-tree` gives for main's tip plus the head: what the decision judged and the preflight must reproduce.
const treeOf = (mainTip, head) => mergeTree((a) => out('git', a, { maxBuffer: 1 << 28 }), mainTip, head);
const FLAGS = ['--dry-run', '--bootstrap', '--preflight-only'];
const USAGE = 'usage: land.mjs <pr> [--dry-run] [--bootstrap] | <pr> --preflight-only [--bootstrap]';
// The checkout this runs from, checked before the first fetch (mainTip null) and again after it: a clean tree at main's tip (only --bootstrap may sit elsewhere), no credentials in its git config.
const ok = (cmd, a) => { try { out(cmd, a); return true; } catch { return false; } }; // did the command exit 0
function checkoutGate(mainTip, bootstrap) {
  const config = (() => { try { return out('git', ['config', '--get-regexp', 'extraheader']); } catch { return ''; } })();
  const c = checkCheckout({ head: out('git', ['rev-parse', 'HEAD']), mainTip, porcelain: out('git', ['-c', 'status.showUntrackedFiles=all', 'status', '--porcelain']), remoteV: out('git', ['remote', '-v']), config, lsv: out('git', ['ls-files', '-v']), diffDirty: !ok('git', ['diff', '--quiet', 'HEAD']), bootstrap });
  if (!c.ok) refuse(c.reason);
  const b = checkNoBunConfig([...fs.readdirSync(out('git', ['rev-parse', '--show-toplevel'])), ...fs.readdirSync(IO.cwd)]); // the filesystem, not porcelain: ignored files count
  if (!b.ok) refuse(b.reason);
}
// Verifiers: the trust gate and the merge-result preflight, nothing else. No ledger, no pod, no merge.
async function preflightOnly(n, bootstrap, hold) {
  hold(lock());
  const pr = viewPr(n);
  trustGate(pr);
  const mainTip = fetchRef(MAIN_REF);
  checkoutGate(mainTip, bootstrap);
  const h = checkHead(pr, fetchRef(`pull/${n}/head`));
  if (!h.ok) refuse(h.reason);
  bootstrapGate(mainTip, bootstrap);
  const pf = await preflight(mainTip, pr.headRefOid.toLowerCase(), bootstrap, treeOf(mainTip, pr.headRefOid.toLowerCase()));
  if (!pf.ok) refuse(`#${n}: ${pf.reason}`);
  say(`#${n}: merge result preflight green (main ${mainTip.slice(0, 8)} + ${pr.headRefOid.slice(0, 8)})`);
}
export async function main(deps) {
  IO = deps;
  REPO = IO.env.JM_REPO;
  SECRET = '';
  const args = IO.argv;
  DRY = args.includes('--dry-run');
  let release = () => {};
  try { await run(args, (r) => { release = r; }); } catch (e) { throw e instanceof Refusal ? e : new Refusal(stripControls(scrub(String(e.message).split('\n')[0], SECRET))); } finally { release(); }
}
async function run(args, hold) {
  const nv = checkNode(IO.nodeVersion);
  if (!nv.ok) refuse(nv.reason);
  const only = args.includes('--preflight-only');
  const missing = (only ? ['JM_REPO', 'JM_TRUSTED_LOGINS'] : REQUIRED).find((v) => !IO.env[v]);
  if (missing) refuse(`environment variable ${missing} is not set`);
  if (!only) reviewers();
  const api = only ? { ok: true } : checkApi(IO.env.JM_COMMONLY_API, IO.env.JM_ALLOW_API_ORIGIN);
  if (!api.ok) refuse(api.reason);
  const nums = args.filter((a) => /^\d+$/.test(a));
  if (nums.length !== 1 || args.some((a) => !/^\d+$/.test(a) && !FLAGS.includes(a)) || (only && args.some((a) => a === '--dry-run'))) refuse(USAGE);
  checkoutGate(null, args.includes('--bootstrap'));
  if (only) return preflightOnly(nums[0], args.includes('--bootstrap'), hold);
  const n = nums[0];
  const opts = { bootstrap: args.includes('--bootstrap') };
  hold(lock());
  const pr = viewPr(n);
  const action = landAction(pr, readLedger());
  if (action === 'already-landed') return say(`#${n} already landed`);
  if (action === 'record-only') {
    console.error(`land: warning: #${n} was merged without a LANDED row; ${DRY ? 'dry run, not recording it' : 'recording it as MERGED-UNVERIFIED'}`);
    return appendLedger(n, pr.headRefOid.toLowerCase(), 'MERGED-UNVERIFIED', 'land', 'merged out of band; recorded without any check');
  }
  trustGate(pr);
  const mainTip = fetchRef(MAIN_REF);
  checkoutGate(mainTip, opts.bootstrap);
  const h = checkHead(pr, fetchRef(`pull/${n}/head`));
  if (!h.ok) refuse(h.reason);
  bootstrapGate(mainTip, opts.bootstrap);
  const head = pr.headRefOid.toLowerCase();
  const d = await decide(pr, mainTip);
  if (!d.ok) refuse(`#${n}: ${d.reason} (nothing merged)`);
  const pf = await preflight(mainTip, head, opts.bootstrap, d.tree);
  if (!pf.ok) refuse(`#${n}: ${pf.reason} (nothing merged)`);
  say(`#${n}: ok, ${[...d.notes, 'merge result preflight green'].join('; ')}`);
  if (mergeAction({ dryRun: DRY }) === 'stop') return say(`dry run: would land #${n} (merge commit, main ${mainTip.slice(0, 8)}); merged nothing`);
  const again = await decide(pr, mainTip);
  if (!again.ok) refuse(`#${n}: ${again.reason} (changed before the merge; nothing merged)`);
  const fresh = viewPr(n); // --match-head-commit pins only the head: the trust gate, base and state are re-read too
  trustGate(fresh);
  const now = checkHead(fresh, head);
  if (!now.ok) refuse(`${now.reason} (changed before the merge; nothing merged)`);
  const moved = checkMain(mainTip, fetchRef(MAIN_REF)); // last look at main, right before the merge
  if (!moved.ok) refuse(moved.reason);
  gh('pr', 'merge', n, '--repo', REPO, '--merge', '--match-head-commit', head);
  const sha = JSON.parse(gh('pr', 'view', n, '--repo', REPO, '--json', 'mergeCommit')).mergeCommit?.oid ?? 'unknown';
  const merged = sha === 'unknown' ? 'unknown' : (() => { try { fetchRef(sha); return out('git', ['rev-parse', `${sha}^{tree}`]); } catch { return 'unreadable'; } })();
  const note = landedNote(sha, pf.tree, merged);
  if (note !== sha) console.error(`land: WARNING: #${n} merged as ${sha} but its tree is not the tested merge result (${pf.tree} tested, ${merged} merged); the base or main moved in the last window. It cannot be undone; look at it now.`);
  appendLedger(n, head, 'LANDED', 'land', note);
  if (opts.bootstrap) appendLedger(n, head, 'BOOTSTRAP', 'land', 'first land of the gate, judged by people');
  say(`#${n} landed as ${sha}; branch ${stripControls(pr.headRefName)} is now safe to delete (not deleted)`);
}
// The CLI: the real I/O. Ctrl-C and kill run the exit handlers that kill the preflight's process group and release the lock.
if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  process.on('SIGINT', () => process.exit(130));
  process.on('SIGTERM', () => process.exit(143));
  process.on('SIGHUP', () => process.exit(129));
  const exec = (cmd, args, opts = {}) => execFileSync(cmd, args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'], ...opts }).trim();
  main({ exec, fetch, spawn, kill: process.kill, env: process.env, argv: process.argv.slice(2), nodeVersion: process.versions.node, cwd: '.' })
    .catch((e) => { console.error(`land: refused: ${e.message}`); process.exit(1); });
}
