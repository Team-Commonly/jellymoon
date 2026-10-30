#!/usr/bin/env node
// make land PR=<n>: the only merge path (docs/development/landing.md). Thin shell-outs to git, gh and the pod API;
// every decision is a pure function in land-decide.mjs, tested by check-land.mjs. Read-only unless merging:
// --dry-run writes no ledger row. PR code runs here only after the trust gate (checkTrust); the preflight is a
// correctness check, not a security boundary.
//   land.mjs <pr> [--dry-run] [--bootstrap]
//   land.mjs <pr> --preflight-only [--bootstrap]   (verifiers: the trust gate and the merge-result preflight only; needs JM_REPO and JM_TRUSTED_LOGINS)
// Run it from a clean checkout of main: it reads the reviewer list from main's tip, never from the working tree.
import { execFileSync, spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {
  acquireLock, addsAllowMarker, appendLedger as writeLedger, checkApi, checkCheckout, checkNoBunConfig, checkHead, checkLedger, checkMain, checkPodVerdict, checkRuns,
  checkTrust, cursorOf, landAction, parseLedger, parseLogins, mergeResult, quoteOutput, readReviewerEntries, readToken, remoteFor,
  removeTree, scrub, takePage, touchesProtected,
} from './land-decide.mjs';

const REQUIRED = ['JM_REPO', 'JM_TRUSTED_LOGINS', 'JM_LEDGER', 'JM_COMMONLY_API', 'JM_COMMONLY_TOKEN_FILE', 'JM_VERDICT_POD'];
const out = (cmd, args, opts = {}) => execFileSync(cmd, args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'], ...opts }).trim();
const REPO = process.env.JM_REPO; // every gh call names it: a worktree can carry remotes for several repos
const gh = (...args) => out('gh', args);
const say = (m) => console.log(m);
let SECRET = ''; // the bearer token once read; scrubbed from anything printed
const DRY = process.argv.includes('--dry-run');
const refuse = (m) => { console.error(`land: refused: ${scrub(m, SECRET)}`); process.exit(1); };

const readLedger = () => parseLedger(fs.existsSync(process.env.JM_LEDGER) ? fs.readFileSync(process.env.JM_LEDGER, 'utf8') : '');
const appendLedger = (pr, head, verdict, by, note) => writeLedger(process.env.JM_LEDGER, [pr, head, verdict, by, new Date().toISOString(), note], { dry: DRY });

function viewPr(n) {
  const pr = JSON.parse(gh('pr', 'view', String(n), '--repo', REPO, '--json', 'url,number,state,baseRefName,headRefName,headRefOid,createdAt,isCrossRepository,author'));
  if (!String(pr.url).includes(`github.com/${REPO}/pull/${n}`)) refuse(`#${n} resolved to ${pr.url}, not ${REPO}`);
  return pr;
}

// One land at a time per clone (two clones can land at once): an mkdir lock in the git common dir; see acquireLock.
function lock() {
  const dir = path.join(path.resolve(out('git', ['rev-parse', '--git-common-dir'])), 'jm-land.lock');
  const got = acquireLock(dir);
  if (!got.ok) refuse(got.reason);
  process.on('exit', () => fs.rmSync(dir, { recursive: true, force: true }));
}

// Fetches main and (optionally) the PR head from the one remote that is JM_REPO.
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
    const child = spawn('env', ['-i', ...Object.entries(env).map(([k, v]) => `${k}=${v}`), 'bash', '-c', './init.sh'], { cwd: wt, detached: true, stdio: ['ignore', 'pipe', 'pipe'] });
    const take = (b) => { if (text.length < 2e6) text += b.toString('utf8'); };
    child.stdout.on('data', take);
    child.stderr.on('data', take);
    const kill = () => { try { process.kill(-child.pid, 'SIGKILL'); } catch { /* the group is already gone */ } };
    process.on('exit', kill); // a crash or Ctrl-C of land must not leave the group running
    const timer = setTimeout(() => { over = true; kill(); }, LIMIT_MS);
    const end = (code) => {
      clearTimeout(timer);
      kill();
      process.off('exit', kill);
      if (text) console.error(scrub(quoteOutput(text), SECRET));
      if (over) return done({ ok: false, reason: `./init.sh did not finish in ${LIMIT_MS / 60000} minutes; killed` });
      return done(code === 0 ? { ok: true } : { ok: false, reason: 'clean-shell ./init.sh (baseline preflight) failed on the merge result' });
    };
    child.on('error', () => end(-1));
    child.on('close', end);
  });
}

// Throwaway worktree at main's tip, the PR head merged in as GitHub would (no repo hooks), then ./init.sh under env -i with
// HOME and TMPDIR inside the throwaway dir and system dirs only on PATH. Hygiene, not isolation.
async function preflight(mainTip, head) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'jm-land-'));
  const wt = path.join(dir, 'wt');
  for (const d of ['home', 'tmp']) fs.mkdirSync(path.join(dir, d));
  const env = { HOME: path.join(dir, 'home'), TMPDIR: path.join(dir, 'tmp'), PATH: '/usr/bin:/bin:/usr/sbin:/sbin', TERM: 'dumb', JM_ENTROPY_BASE: mainTip }; // the ratchet reads main's tip as fetched, never a ref the PR could move
  let result = { ok: true };
  try {
    try { out('git', ['worktree', 'add', '--quiet', '--detach', wt, mainTip]); } catch (e) { result = { ok: false, reason: `git worktree add failed: ${why(e)}` }; }
    if (result.ok) try { out('git', ['-C', wt, ...BASE, 'merge', '--no-ff', '--no-edit', head]); } catch {
      result = { ok: false, reason: `merging ${head.slice(0, 8)} into main ${mainTip.slice(0, 8)} conflicts` };
    }
    if (result.ok) result = await runInit(wt, env);
  } catch (e) { result = { ok: false, reason: `preflight failed: ${why(e)}` }; }
  try { execFileSync('chmod', ['-R', 'u+w', dir]); out('git', ['worktree', 'remove', '--force', wt]); } catch { /* removeTree warns */ }
  removeTree(dir);
  return result;
}

// Pod messages newest-first, paged back past `since` (ms), deduped by id. Fails closed if the page cap is hit first,
// or if a page boundary cannot be paged past (see takePage).
async function podMessages(since) {
  SECRET = readToken(process.env.JM_COMMONLY_TOKEN_FILE);
  const all = [];
  const seen = new Set();
  let before = '';
  for (let page = 0; page < 30; page += 1) {
    const url = `${process.env.JM_COMMONLY_API}/api/messages/${process.env.JM_VERDICT_POD}?limit=200${before}`;
    let msgs;
    try {
      const res = await fetch(url, { headers: { Authorization: `Bearer ${SECRET}` } });
      if (!res.ok) refuse(`pod messages: HTTP ${res.status}`);
      msgs = await res.json();
    } catch (e) { return refuse(`pod messages: request failed (${e.name})`); }
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

// Reviewers come from main's tip, never from the working tree. Bootstrap (main has no land.mjs) reads H instead.
const reviewers = (ref) => readReviewerEntries(out('git', ['show', `${ref}:.github/opus-reviewers.txt`]));
function hasLand(mainTip) { try { out('git', ['cat-file', '-e', `${mainTip}:scripts/land.mjs`]); return true; } catch { return false; } }

// Trust gate: runs before anything of the PR is fetched or executed. Trust is GitHub identity: not cross-repository, the
// author and every account with push access are in JM_TRUSTED_LOGINS (an operator env var; a PR cannot widen it).
function trustGate(pr) {
  const pushers = gh('api', '--paginate', `repos/${REPO}/collaborators?affiliation=all&permission=push&per_page=100`, '--jq', '.[].login').split('\n').filter(Boolean);
  const t = checkTrust({ pr: { isCrossRepository: pr.isCrossRepository, author: pr.author?.login }, pushers, trusted: parseLogins(process.env.JM_TRUSTED_LOGINS) });
  if (!t.ok) refuse(`#${pr.number}: ${t.reason}`);
}

// Every ledger, CI and pod check at this moment. Run before the preflight and again right before the merge.
async function decide(pr, mainTip, opts) { // mainTip is the tip fetched for this land
  const n = String(pr.number);
  const head = pr.headRefOid;
  const rows = readLedger();
  const led = checkLedger(rows, { pr: n, head });
  if (!led.ok) return led;
  const notes = [`ledger PASS by ${led.primary.by}`];
  const runs = JSON.parse(gh('api', `repos/${REPO}/commits/${head}/check-runs?per_page=100`, '--jq', '{total: .total_count, runs: [.check_runs[] | {name, conclusion}]}'));
  const ci = checkRuns(runs.runs, runs.total);
  if (!ci.ok) return ci;
  const merged = mergeResult((a) => out('git', a, { maxBuffer: 1 << 28 }), mainTip, head);
  if (touchesProtected(merged.files) || addsAllowMarker(merged.patch)) {
    const gate = checkPodVerdict(await podMessages(Date.parse(pr.createdAt)), { pr: n, head, entries: reviewers(opts.bootstrap ? head : mainTip) });
    if (!gate.ok) return gate;
    notes.push(gate.reason);
  }
  return { ok: true, notes };
}

const FLAGS = ['--dry-run', '--bootstrap', '--preflight-only'];
const USAGE = 'usage: land.mjs <pr> [--dry-run] [--bootstrap] | <pr> --preflight-only [--bootstrap]';

// The checkout this runs from, checked before the first fetch (mainTip null) and again after it: a clean tree at main's tip (only --bootstrap may sit elsewhere), no credentials in its git config.
function checkoutGate(mainTip, bootstrap) {
  const config = (() => { try { return out('git', ['config', '--get-regexp', 'extraheader']); } catch { return ''; } })();
  const c = checkCheckout({ head: out('git', ['rev-parse', 'HEAD']), mainTip, porcelain: out('git', ['status', '--porcelain']), remoteV: out('git', ['remote', '-v']), config, bootstrap });
  if (!c.ok) refuse(c.reason);
  const b = checkNoBunConfig([...fs.readdirSync(out('git', ['rev-parse', '--show-toplevel'])), ...fs.readdirSync('.')]); // the filesystem, not porcelain: ignored files count
  if (!b.ok) refuse(b.reason);
}

// Verifiers: the trust gate and the merge-result preflight, nothing else. No ledger, no pod, no merge.
async function preflightOnly(n, bootstrap) {
  lock();
  const pr = viewPr(n);
  trustGate(pr);
  const mainTip = fetchRef('main');
  checkoutGate(mainTip, bootstrap);
  const h = checkHead(pr, fetchRef(`pull/${n}/head`));
  if (!h.ok) refuse(h.reason);
  const pf = await preflight(mainTip, pr.headRefOid.toLowerCase());
  if (!pf.ok) refuse(`#${n}: ${pf.reason}`);
  say(`#${n}: merge result preflight green (main ${mainTip.slice(0, 8)} + ${pr.headRefOid.slice(0, 8)})`);
}

async function main() {
  const args = process.argv.slice(2);
  const only = args.includes('--preflight-only');
  const missing = (only ? ['JM_REPO', 'JM_TRUSTED_LOGINS'] : REQUIRED).find((v) => !process.env[v]);
  if (missing) refuse(`environment variable ${missing} is not set`);
  const api = only ? { ok: true } : checkApi(process.env.JM_COMMONLY_API, process.env.JM_ALLOW_API_ORIGIN);
  if (!api.ok) refuse(api.reason);
  const nums = args.filter((a) => /^\d+$/.test(a));
  if (nums.length !== 1 || args.some((a) => !/^\d+$/.test(a) && !FLAGS.includes(a)) || (only && args.some((a) => a === '--dry-run'))) refuse(USAGE);
  checkoutGate(null, args.includes('--bootstrap'));
  if (only) return preflightOnly(nums[0], args.includes('--bootstrap'));
  const n = nums[0];
  const opts = { bootstrap: args.includes('--bootstrap') };
  lock();
  const pr = viewPr(n);
  const action = landAction(pr, readLedger());
  if (action === 'already-landed') return say(`#${n} already landed`);
  if (action === 'record-only') {
    console.error(`land: warning: #${n} was merged without a LANDED row; ${DRY ? 'dry run, not recording it' : 'recording it as MERGED-UNVERIFIED'}`);
    return appendLedger(n, pr.headRefOid.toLowerCase(), 'MERGED-UNVERIFIED', 'land', 'merged out of band; recorded without any check');
  }
  trustGate(pr);
  const mainTip = fetchRef('main');
  checkoutGate(mainTip, opts.bootstrap);
  const h = checkHead(pr, fetchRef(`pull/${n}/head`));
  if (!h.ok) refuse(h.reason);
  if (hasLand(mainTip) === opts.bootstrap) refuse(opts.bootstrap ? 'main already has scripts/land.mjs; --bootstrap is only for the first land of the gate' : 'main has no scripts/land.mjs yet; the first land of the gate needs --bootstrap and is judged by people');
  const head = pr.headRefOid.toLowerCase();
  const d = await decide(pr, mainTip, opts);
  if (!d.ok) refuse(`#${n}: ${d.reason} (nothing merged)`);
  const pf = await preflight(mainTip, head);
  if (!pf.ok) refuse(`#${n}: ${pf.reason} (nothing merged)`);
  say(`#${n}: ok, ${[...d.notes, 'merge result preflight green'].join('; ')}`);
  if (DRY) return say(`dry run: would land #${n} (merge commit, main ${mainTip.slice(0, 8)}); merged nothing`);
  const again = await decide(pr, mainTip, opts);
  if (!again.ok) refuse(`#${n}: ${again.reason} (changed before the merge; nothing merged)`);
  const fresh = viewPr(n); // --match-head-commit pins only the head: the trust gate, base and state are re-read too
  trustGate(fresh);
  const now = checkHead(fresh, head);
  if (!now.ok) refuse(`${now.reason} (changed before the merge; nothing merged)`);
  const moved = checkMain(mainTip, fetchRef('main')); // last look at main, right before the merge
  if (!moved.ok) refuse(moved.reason);
  gh('pr', 'merge', n, '--repo', REPO, '--merge', '--match-head-commit', head);
  const sha = JSON.parse(gh('pr', 'view', n, '--repo', REPO, '--json', 'mergeCommit')).mergeCommit?.oid ?? 'unknown';
  appendLedger(n, head, 'LANDED', 'land', sha);
  if (opts.bootstrap) appendLedger(n, head, 'BOOTSTRAP', 'land', 'first land of the gate, judged by people');
  say(`#${n} landed as ${sha}; branch ${pr.headRefName} is now safe to delete (not deleted)`);
}

main().catch((e) => refuse(e.message.split('\n')[0]));
