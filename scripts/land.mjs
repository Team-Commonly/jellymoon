#!/usr/bin/env node
// make land PR=<n>: the only merge path (docs/development/landing.md). Thin shell-outs to git, gh and the pod API;
// every decision is a pure function in land-decide.mjs, tested by check-land.mjs. Read-only unless merging:
// --dry-run writes no ledger row and the preflight runs in a macOS sandbox, proved before each run.
//   land.mjs <pr> [--dry-run] [--reviewer-gate-unavailable] [--bootstrap]     land.mjs --owed
//   land.mjs <pr> --preflight-only   (verifiers: the sandboxed merge-result preflight only; needs just JM_REPO)
// Run it from main's checkout: it reads the reviewer list from main's tip, never from the working tree, and a land
// refuses when this checkout's scripts/ differ from main's.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {
  acquireLock, appendLedger as writeLedger, checkApi, checkHead, checkLedger, checkMain, checkPlatform, checkPodVerdict, checkSecondVerifier,
  cursorOf, filesFromNameStatus, landAction, owedList, parseLedger, proveSandbox, readReviewerEntries, readToken, remoteFor, removeTree,
  sandboxProfile, scrub, takePage, touchesProtected,
} from './land-decide.mjs';

const REQUIRED = ['JM_REPO', 'JM_LEDGER', 'JM_COMMONLY_API', 'JM_COMMONLY_TOKEN_FILE', 'JM_VERDICT_POD'];
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
  const pr = JSON.parse(gh('pr', 'view', String(n), '--repo', REPO, '--json', 'url,number,state,baseRefName,headRefName,headRefOid,createdAt'));
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

// The PR's own code runs here: no read of the real home (but the git common dir, the read-only toolchain cache and the
// throwaway dir), no write outside the throwaway dir and /dev, no unix socket, no keychain. A cache miss downloads into
// the throwaway dir. A land checkout under a denied dir (the token's or the ledger's) can never pass.
async function sandboxed(dir, wt, env) {
  const homes = [...new Set([os.userInfo().homedir, process.env.HOME].filter(Boolean))];
  const profile = sandboxProfile({ homes, allowRead: [path.resolve(out('git', ['rev-parse', '--git-common-dir'])), process.env.JM_TOOLCHAIN_CACHE],
    allowWrite: [dir], hide: [process.env.JM_LEDGER, process.env.JM_COMMONLY_TOKEN_FILE] });
  const proof = await proveSandbox({ profile, dir, home: homes });
  if (!proof.ok) return proof;
  try {
    execFileSync('sandbox-exec', ['-p', profile, 'env', '-i', ...Object.entries(env).map(([k, v]) => `${k}=${v}`), 'bash', '-c', './init.sh'], { cwd: wt, stdio: 'inherit' });
    return { ok: true };
  } catch { return { ok: false, reason: 'clean-shell ./init.sh (baseline preflight) failed on the merge result' }; }
}

// Throwaway worktree at main's tip, the PR head merged in as GitHub would (no repo hooks), then ./init.sh (which runs
// make preflight) under env -i with an empty HOME and TMPDIR inside the throwaway dir, system dirs only, in sandbox-exec.
async function preflight(mainTip, head) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'jm-land-'));
  const wt = path.join(dir, 'wt');
  for (const d of ['home', 'tmp']) fs.mkdirSync(path.join(dir, d));
  const env = { HOME: path.join(dir, 'home'), TMPDIR: path.join(dir, 'tmp'), PATH: '/usr/bin:/bin:/usr/sbin:/sbin', TERM: 'dumb' };
  if (process.env.JM_TOOLCHAIN_CACHE) env.JM_TOOLCHAIN_CACHE = process.env.JM_TOOLCHAIN_CACHE; // read-only; read once tools.sh supports it
  let result = { ok: true };
  try {
    try { out('git', ['worktree', 'add', '--quiet', '--detach', wt, mainTip]); } catch (e) { result = { ok: false, reason: `git worktree add failed: ${why(e)}` }; }
    if (result.ok) try { out('git', ['-C', wt, ...BASE, 'merge', '--no-ff', '--no-edit', head]); } catch {
      result = { ok: false, reason: `merging ${head.slice(0, 8)} into main ${mainTip.slice(0, 8)} conflicts` };
    }
    if (result.ok) result = await sandboxed(dir, wt, env);
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

// Every ledger and pod check at this moment. Run before the preflight and again right before the merge.
async function decide(pr, mainTip, opts) {
  const n = String(pr.number);
  const head = pr.headRefOid;
  const rows = readLedger();
  const led = checkLedger(rows, { pr: n, head });
  if (!led.ok) return led;
  const notes = [`ledger PASS by ${led.primary.by}`];
  const files = filesFromNameStatus(out('git', ['diff', '--name-status', '-M', out('git', ['merge-base', mainTip, head]), head]));
  let regate = false;
  if (touchesProtected(files)) {
    const gate = opts.reviewer
      ? checkSecondVerifier(rows, { pr: n, head, primary: led.primary })
      : checkPodVerdict(await podMessages(Date.parse(pr.createdAt)), { pr: n, head, entries: reviewers(opts.bootstrap ? head : mainTip) });
    if (!gate.ok) return gate;
    notes.push(gate.reason);
    regate = opts.reviewer;
  }
  return { ok: true, notes, regate };
}

async function owed() {
  const mainTip = fetchRef('main');
  const rows = readLedger();
  const owing = rows.filter((r) => r.verdict === 'REGATE-OWED');
  const since = Math.min(Date.now(), ...owing.map((r) => Date.parse(r.when) || Date.now()));
  const msgs = owing.length ? await podMessages(since) : [];
  const entries = reviewers(mainTip);
  const list = owedList(rows, (pr, head, after) => checkPodVerdict(msgs, { pr, head, entries, after }).ok);
  say(list.length ? list.map((r) => `#${r.pr} ${r.head} owes a reviewer re-gate (landed ${r.when})`).join('\n') : 'no PR owes a reviewer re-gate');
}

const FLAGS = ['--dry-run', '--reviewer-gate-unavailable', '--bootstrap', '--preflight-only'];
const USAGE = 'usage: land.mjs <pr> [--dry-run] [--reviewer-gate-unavailable] [--bootstrap] | <pr> --preflight-only | --owed';

// This checkout's scripts/ must be main's: land runs whatever scripts/land.mjs is in the working tree.
function scriptsAreMains(mainTip) {
  try { execFileSync('git', ['diff', '--quiet', mainTip, '--', 'scripts/']); } catch { return false; }
  return out('git', ['ls-files', '--others', '--exclude-standard', 'scripts/']) === '';
}

// Verifiers: the sandboxed merge-result preflight and nothing else. No ledger, no pod, no merge.
async function preflightOnly(n) {
  lock();
  const pr = viewPr(n);
  const mainTip = fetchRef('main');
  const h = checkHead(pr, fetchRef(`pull/${n}/head`));
  if (!h.ok) refuse(h.reason);
  const pf = await preflight(mainTip, pr.headRefOid.toLowerCase());
  if (!pf.ok) refuse(`#${n}: ${pf.reason}`);
  say(`#${n}: merge result preflight green in the sandbox (main ${mainTip.slice(0, 8)} + ${pr.headRefOid.slice(0, 8)})`);
}

async function main() {
  const args = process.argv.slice(2);
  const only = args.includes('--preflight-only');
  const missing = (only ? ['JM_REPO'] : REQUIRED).find((v) => !process.env[v]);
  if (missing) refuse(`environment variable ${missing} is not set`);
  const api = only ? { ok: true } : checkApi(process.env.JM_COMMONLY_API);
  if (!api.ok) refuse(api.reason);
  const sb = checkPlatform(process.platform);
  if (!sb.ok) refuse(sb.reason);
  if (args.includes('--owed')) return owed();
  const nums = args.filter((a) => /^\d+$/.test(a));
  if (nums.length !== 1 || args.some((a) => !/^\d+$/.test(a) && !FLAGS.includes(a)) || (only && args.length > 2)) refuse(USAGE);
  if (only) return preflightOnly(nums[0]);
  const n = nums[0];
  const opts = { reviewer: args.includes('--reviewer-gate-unavailable'), bootstrap: args.includes('--bootstrap') };
  lock();
  const pr = viewPr(n);
  const action = landAction(pr, readLedger());
  if (action === 'already-landed') return say(`#${n} already landed`);
  if (action === 'record-only') {
    console.error(`land: warning: #${n} was merged without a LANDED row; ${DRY ? 'dry run, not recording it' : 'recording it as MERGED-UNVERIFIED'}`);
    return appendLedger(n, pr.headRefOid.toLowerCase(), 'MERGED-UNVERIFIED', 'land', 'merged out of band; recorded without any check');
  }
  const mainTip = fetchRef('main');
  const h = checkHead(pr, fetchRef(`pull/${n}/head`));
  if (!h.ok) refuse(h.reason);
  if (hasLand(mainTip) === opts.bootstrap) refuse(opts.bootstrap ? 'main already has scripts/land.mjs; --bootstrap is only for the first land of the gate' : 'main has no scripts/land.mjs yet; the first land of the gate needs --bootstrap and is judged by people');
  if (!opts.bootstrap && !scriptsAreMains(mainTip)) refuse("scripts/ in this checkout differ from main's tip; land from a clean checkout of main");
  const head = pr.headRefOid.toLowerCase();
  const d = await decide(pr, mainTip, opts);
  if (!d.ok) refuse(`#${n}: ${d.reason} (nothing merged)`);
  const pf = await preflight(mainTip, head);
  if (!pf.ok) refuse(`#${n}: ${pf.reason} (nothing merged)`);
  say(`#${n}: ok, ${[...d.notes, 'merge result preflight green'].join('; ')}`);
  if (DRY) return say(`dry run: would land #${n} (merge commit, main ${mainTip.slice(0, 8)}); merged nothing`);
  const moved = checkMain(mainTip, fetchRef('main'));
  if (!moved.ok) refuse(moved.reason);
  const again = await decide(pr, mainTip, opts);
  if (!again.ok) refuse(`#${n}: ${again.reason} (changed before the merge; nothing merged)`);
  gh('pr', 'merge', n, '--repo', REPO, '--merge', '--match-head-commit', head);
  const sha = JSON.parse(gh('pr', 'view', n, '--repo', REPO, '--json', 'mergeCommit')).mergeCommit?.oid ?? 'unknown';
  appendLedger(n, head, 'LANDED', 'land', sha);
  if (d.regate) appendLedger(n, head, 'REGATE-OWED', 'land', 'reviewer-gate-unavailable');
  if (opts.bootstrap) appendLedger(n, head, 'BOOTSTRAP', 'land', 'first land of the gate, judged by people');
  say(`#${n} landed as ${sha}; branch ${pr.headRefName} is now safe to delete (not deleted)`);
}

main().catch((e) => refuse(e.message.split('\n')[0]));
