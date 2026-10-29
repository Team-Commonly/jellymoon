#!/usr/bin/env node
// make land PR=<n>: the only merge path (docs/development/landing.md). Thin shell-outs to git, gh and the pod API;
// every decision is a pure function in land-decide.mjs, tested by check-land.mjs. Read-only unless merging.
//   land.mjs <pr> [--dry-run] [--reviewer-gate-unavailable] [--bootstrap]     land.mjs --owed
// Run it from main's checkout: it reads the reviewer list from main's tip, never from the working tree.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { readReviewerEntries, touchesProtected } from './review-gate.mjs';
import {
  checkApi, checkHead, checkLedger, checkMain, checkPodVerdict, checkSecondVerifier, filesFromNameStatus,
  landAction, owedList, parseLedger, remoteFor, removeTree,
} from './land-decide.mjs';

const REQUIRED = ['JM_REPO', 'JM_LEDGER', 'JM_COMMONLY_API', 'JM_COMMONLY_TOKEN_FILE', 'JM_VERDICT_POD'];
const out = (cmd, args, opts = {}) => execFileSync(cmd, args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'], ...opts }).trim();
const REPO = process.env.JM_REPO; // every gh call names it: a worktree can carry remotes for several repos
const gh = (...args) => out('gh', args);
const say = (m) => console.log(m);
const refuse = (m) => { console.error(`land: refused: ${m}`); process.exit(1); };

const readLedger = () => parseLedger(fs.existsSync(process.env.JM_LEDGER) ? fs.readFileSync(process.env.JM_LEDGER, 'utf8') : '');
function appendLedger(pr, head, verdict, by, note) {
  const f = process.env.JM_LEDGER;
  const exists = fs.existsSync(f);
  const lead = exists && !fs.readFileSync(f, 'utf8').endsWith('\n') ? '\n' : '';
  const header = exists ? '' : 'pr\thead\tverdict\tby\twhen\tnote\n';
  fs.appendFileSync(f, `${header}${lead}${[pr, head, verdict, by, new Date().toISOString(), note].join('\t')}\n`);
}

function viewPr(n) {
  const pr = JSON.parse(gh('pr', 'view', String(n), '--repo', REPO, '--json', 'url,number,state,baseRefName,headRefName,headRefOid,createdAt'));
  if (!String(pr.url).includes(`github.com/${REPO}/pull/${n}`)) refuse(`#${n} resolved to ${pr.url}, not ${REPO}`);
  return pr;
}

// One land at a time: an mkdir lock in the git common dir holding the PID; a dead PID is taken over.
function lock() {
  const dir = path.join(path.resolve(out('git', ['rev-parse', '--git-common-dir'])), 'jm-land.lock');
  for (let tries = 0; tries < 2; tries += 1) {
    try {
      fs.mkdirSync(dir);
      fs.writeFileSync(path.join(dir, 'pid'), String(process.pid));
      process.on('exit', () => fs.rmSync(dir, { recursive: true, force: true }));
      return;
    } catch {
      const pid = Number(fs.readFileSync(path.join(dir, 'pid'), 'utf8').trim());
      let alive = true;
      try { process.kill(pid, 0); } catch (e) { alive = e.code !== 'ESRCH'; }
      if (alive) refuse(`another land is running (pid ${pid})`);
      fs.rmSync(dir, { recursive: true, force: true });
    }
  }
  refuse('could not take the land lock');
}

// Fetches main and (optionally) the PR head from the one remote that is JM_REPO.
function fetchRef(ref) {
  const m = remoteFor(out('git', ['remote', '-v']), REPO);
  if (m.error) refuse(m.error);
  out('git', ['fetch', '--quiet', m.remote, ref]);
  return out('git', ['rev-parse', 'FETCH_HEAD']);
}

// Throwaway worktree at main's tip, the PR head merged in as GitHub would, then ./init.sh (which runs
// make preflight) under env -i with an empty HOME and system dirs only.
function preflight(mainTip, head) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'jm-land-'));
  const wt = path.join(dir, 'wt');
  fs.mkdirSync(path.join(dir, 'home'));
  const env = { HOME: path.join(dir, 'home'), PATH: '/usr/bin:/bin:/usr/sbin:/sbin', TERM: 'dumb' };
  if (process.env.JM_TOOLCHAIN_CACHE) env.JM_TOOLCHAIN_CACHE = process.env.JM_TOOLCHAIN_CACHE; // read once tools.sh supports it
  let result = { ok: true };
  try {
    out('git', ['worktree', 'add', '--quiet', '--detach', wt, mainTip]);
    try { out('git', ['-C', wt, '-c', 'user.name=land', '-c', 'user.email=land@localhost', 'merge', '--no-ff', '--no-edit', head]); } catch {
      result = { ok: false, reason: `merging ${head.slice(0, 8)} into main ${mainTip.slice(0, 8)} conflicts` };
    }
    if (result.ok) execFileSync('env', ['-i', ...Object.entries(env).map(([k, v]) => `${k}=${v}`), 'bash', '-c', './init.sh'], { cwd: wt, stdio: 'inherit' });
  } catch { result = { ok: false, reason: `clean-shell ./init.sh (baseline preflight) failed on main + ${head.slice(0, 8)}` }; }
  try { execFileSync('chmod', ['-R', 'u+w', dir]); out('git', ['worktree', 'remove', '--force', wt]); } catch { /* removeTree warns */ }
  removeTree(dir);
  return result;
}

// Pod messages newest-first, paged back past `since` (ms). Fails closed if the page cap is hit first.
async function podMessages(since) {
  const token = fs.readFileSync(process.env.JM_COMMONLY_TOKEN_FILE, 'utf8').trim();
  const all = [];
  let before = '';
  for (let page = 0; page < 30; page += 1) {
    const url = `${process.env.JM_COMMONLY_API}/api/messages/${process.env.JM_VERDICT_POD}?limit=200${before}`;
    const res = await fetch(url, { headers: { Authorization: `Bearer ${token}` } });
    if (!res.ok) refuse(`pod messages: HTTP ${res.status}`);
    const msgs = await res.json();
    if (!msgs.length) return all;
    all.push(...msgs);
    const t = msgs.map((m) => m.createdAt ?? m.created_at).sort()[0];
    if (Date.parse(t) < since) return all;
    before = `&before=${encodeURIComponent(t)}`;
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

async function main() {
  const args = process.argv.slice(2);
  const missing = REQUIRED.find((v) => !process.env[v]);
  if (missing) refuse(`environment variable ${missing} is not set`);
  const api = checkApi(process.env.JM_COMMONLY_API);
  if (!api.ok) refuse(api.reason);
  if (args.includes('--owed')) return owed();
  const n = args.find((a) => /^\d+$/.test(a));
  if (!n || args.some((a) => !/^\d+$/.test(a) && !['--dry-run', '--reviewer-gate-unavailable', '--bootstrap'].includes(a))) {
    refuse('usage: land.mjs <pr> [--dry-run] [--reviewer-gate-unavailable] [--bootstrap] | --owed');
  }
  const opts = { reviewer: args.includes('--reviewer-gate-unavailable'), bootstrap: args.includes('--bootstrap') };
  lock();
  const pr = viewPr(n);
  const action = landAction(pr, readLedger());
  if (action === 'already-landed') return say(`#${n} already landed`);
  if (action === 'record-only') {
    console.error(`land: warning: #${n} was merged without a LANDED row; recording it`);
    return appendLedger(n, pr.headRefOid.toLowerCase(), 'LANDED', 'land', 'recorded after an out-of-band merge');
  }
  const mainTip = fetchRef('main');
  const h = checkHead(pr, fetchRef(`pull/${n}/head`));
  if (!h.ok) refuse(h.reason);
  if (hasLand(mainTip) === opts.bootstrap) refuse(opts.bootstrap ? 'main already has scripts/land.mjs; --bootstrap is only for the first land of the gate' : 'main has no scripts/land.mjs yet; the first land of the gate needs --bootstrap and is judged by people');
  const head = pr.headRefOid.toLowerCase();
  const d = await decide(pr, mainTip, opts);
  if (!d.ok) refuse(`#${n}: ${d.reason} (nothing merged)`);
  const pf = preflight(mainTip, head);
  if (!pf.ok) refuse(`#${n}: ${pf.reason} (nothing merged)`);
  say(`#${n}: ok, ${[...d.notes, 'merge result preflight green'].join('; ')}`);
  if (args.includes('--dry-run')) return say(`dry run: would land #${n} (merge commit, main ${mainTip.slice(0, 8)}); merged nothing`);
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
