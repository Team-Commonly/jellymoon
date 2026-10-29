#!/usr/bin/env node
// make land PR=<n>: the only merge path (docs/development/landing.md). Thin shell-outs to git, gh and the pod API;
// every decision is a pure function in land-decide.mjs, tested by check-land.mjs. Read-only unless merging.
//   land.mjs <pr> [--stack] [--dry-run] [--reviewer-gate-unavailable]    land.mjs --owed
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { readReviewerEntries, touchesProtected } from './review-gate.mjs';
import { checkHead, checkLedger, checkPodVerdict, checkSecondVerifier, landAction, owedList, parseLedger } from './land-decide.mjs';

const REQUIRED = ['JM_LEDGER', 'JM_COMMONLY_API', 'JM_COMMONLY_TOKEN_FILE', 'JM_VERDICT_POD'];
const out = (cmd, args, opts = {}) => execFileSync(cmd, args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'], ...opts }).trim();
const gh = (...args) => out('gh', args);
const say = (m) => console.log(m);
const refuse = (m) => { console.error(`land: refused: ${m}`); process.exit(1); };

const readLedger = () => parseLedger(fs.existsSync(process.env.JM_LEDGER) ? fs.readFileSync(process.env.JM_LEDGER, 'utf8') : '');
const when = () => new Date().toISOString();
function appendLedger(pr, head, verdict, by, note) {
  const f = process.env.JM_LEDGER;
  const lead = fs.existsSync(f) && !fs.readFileSync(f, 'utf8').endsWith('\n') ? '\n' : '';
  fs.appendFileSync(f, `${lead}${[pr, head, verdict, by, when(), note].join('\t')}\n`);
}

const viewPr = (n) => JSON.parse(gh('pr', 'view', String(n), '--json', 'number,state,baseRefName,headRefName,headRefOid,createdAt'));
const changedFiles = (n) => gh('api', `repos/{owner}/{repo}/pulls/${n}/files`, '--paginate', '--jq', '.[] | {filename, previous_filename}')
  .split('\n').filter(Boolean).map((l) => JSON.parse(l));

// Base chain from this PR down to main, bottom-up. A merged PR ends the walk.
function chainOf(n, stack) {
  const chain = [viewPr(n)];
  while (stack && chain[0].state !== 'MERGED' && chain[0].baseRefName !== 'main') {
    const below = JSON.parse(gh('pr', 'list', '--state', 'all', '--head', chain[0].baseRefName, '--json', 'number', '--limit', '1'))[0];
    if (!below) refuse(`#${chain[0].number} is based on ${chain[0].baseRefName}, which has no PR; not a stack down to main`);
    chain.unshift(viewPr(below.number));
  }
  if (!stack && chain[0].baseRefName !== 'main' && chain[0].state !== 'MERGED') refuse(`#${n} is based on ${chain[0].baseRefName}, not main; use --stack`);
  return chain;
}

function fetchLocal(n) {
  const slug = gh('repo', 'view', '--json', 'nameWithOwner', '-q', '.nameWithOwner');
  const remote = out('git', ['remote', '-v']).split('\n').find((l) => l.includes(`/${slug}`) || l.includes(`:${slug}`))?.split(/\s/)[0];
  if (!remote) refuse(`no git remote points at ${slug}`);
  out('git', ['fetch', '--quiet', remote, `pull/${n}/head`]);
  return out('git', ['rev-parse', 'FETCH_HEAD']);
}

// Clean-shell preflight at the head: throwaway worktree, empty HOME, system PATH only.
function preflight(head) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'jm-land-'));
  const wt = path.join(dir, 'wt');
  fs.mkdirSync(path.join(dir, 'home'));
  const env = { HOME: path.join(dir, 'home'), PATH: '/usr/bin:/bin:/usr/sbin:/sbin', TERM: 'dumb' };
  if (process.env.JM_TOOLCHAIN_CACHE) env.JM_TOOLCHAIN_CACHE = process.env.JM_TOOLCHAIN_CACHE;
  try {
    out('git', ['worktree', 'add', '--quiet', '--detach', wt, head]);
    execFileSync('env', ['-i', ...Object.entries(env).map(([k, v]) => `${k}=${v}`), 'bash', '-c', './init.sh && make preflight'], { cwd: wt, stdio: 'inherit' });
    return { ok: true };
  } catch { return { ok: false, reason: `clean-shell preflight failed at ${head.slice(0, 8)}` }; } finally {
    try { out('git', ['worktree', 'remove', '--force', wt]); } catch { /* already gone */ }
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

// Pod messages newest-first, paged back until older than `since` (ms).
async function podMessages(since) {
  const token = fs.readFileSync(process.env.JM_COMMONLY_TOKEN_FILE, 'utf8').trim();
  const all = [];
  let before = '';
  for (let page = 0; page < 30; page += 1) {
    const url = `${process.env.JM_COMMONLY_API}/api/messages/${process.env.JM_VERDICT_POD}?limit=200${before}`;
    const res = await fetch(url, { headers: { Authorization: `Bearer ${token}` } });
    if (!res.ok) refuse(`pod messages: HTTP ${res.status}`);
    const msgs = await res.json();
    if (!msgs.length) break;
    all.push(...msgs);
    const oldest = msgs.reduce((a, m) => ((m.createdAt ?? m.created_at) < (a.createdAt ?? a.created_at) ? m : a));
    const t = oldest.createdAt ?? oldest.created_at;
    if (Date.parse(t) < since) break;
    before = `&before=${encodeURIComponent(t)}`;
  }
  return all;
}

const reviewers = () => readReviewerEntries(fs.readFileSync(new URL('../.github/opus-reviewers.txt', import.meta.url), 'utf8'));

// All dry checks for one open PR. Returns { ok, reason, notes, regate }.
async function checkOne(pr, chain, rows, flag) {
  const n = String(pr.number);
  const head = pr.headRefOid;
  const h = checkHead(pr, fetchLocal(n));
  if (!h.ok) return h;
  const led = checkLedger(rows, { pr: n, head, chain: chain.map((c) => ({ pr: String(c.number), head: c.headRefOid })) });
  if (!led.ok) return led;
  const notes = [`ledger ${led.primary.verdict} by ${led.primary.by}`];
  let regate = false;
  if (touchesProtected(changedFiles(n))) {
    const gate = flag
      ? checkSecondVerifier(rows, { pr: n, head, primary: led.primary })
      : checkPodVerdict(await podMessages(Date.parse(pr.createdAt)), { pr: n, head, entries: reviewers() });
    if (!gate.ok) return gate;
    notes.push(gate.reason);
    regate = flag;
  }
  const pf = preflight(head);
  if (!pf.ok) return pf;
  notes.push('clean-shell preflight green');
  return { ok: true, notes, regate };
}

async function owed() {
  const rows = readLedger();
  const since = Math.min(...rows.filter((r) => r.verdict === 'REGATE-OWED').map((r) => Date.parse(r.when) || Date.now()), Date.now());
  const msgs = rows.some((r) => r.verdict === 'REGATE-OWED') ? await podMessages(since) : [];
  const list = owedList(rows, (pr, head, after) => checkPodVerdict(msgs, { pr, head, entries: reviewers(), after }).ok);
  say(list.length ? list.map((r) => `#${r.pr} ${r.head} owes a reviewer re-gate (landed ${r.when})`).join('\n') : 'no PR owes a reviewer re-gate');
}

async function main() {
  const args = process.argv.slice(2);
  const missing = REQUIRED.find((v) => !process.env[v]);
  if (missing) refuse(`environment variable ${missing} is not set`);
  if (args.includes('--owed')) return owed();
  const n = args.find((a) => /^\d+$/.test(a));
  if (!n) refuse('usage: land.mjs <pr> [--stack] [--dry-run] [--reviewer-gate-unavailable] | --owed');
  const [dry, flag] = [args.includes('--dry-run'), args.includes('--reviewer-gate-unavailable')];
  const chain = chainOf(n, args.includes('--stack'));
  const rows = readLedger();
  const plan = [];
  for (const pr of chain) {
    const action = landAction(pr, rows);
    if (action !== 'check') { plan.push({ pr, action }); say(`#${pr.number}: ${action}`); continue; }
    const r = await checkOne(pr, chain, rows, flag);
    if (!r.ok) refuse(`#${pr.number}: ${r.reason} (nothing merged)`);
    say(`#${pr.number}: ok, ${r.notes.join('; ')}`);
    plan.push({ pr, action: 'merge', regate: r.regate });
  }
  if (dry) { say(`dry run: would land ${plan.filter((p) => p.action === 'merge').map((p) => `#${p.pr.number}`).join(', ') || 'nothing'}; merged nothing`); return; }
  for (const { pr, action, regate } of plan) {
    const head = pr.headRefOid;
    if (action === 'already-landed') { say(`#${pr.number} already landed`); continue; }
    if (action === 'record-only') {
      console.error(`land: warning: #${pr.number} was merged without a LANDED row; recording it`);
    } else {
      if (pr.baseRefName !== 'main') gh('pr', 'edit', String(pr.number), '--base', 'main');
      gh('pr', 'merge', String(pr.number), '--merge', '--match-head-commit', head);
    }
    const sha = JSON.parse(gh('pr', 'view', String(pr.number), '--json', 'mergeCommit')).mergeCommit?.oid ?? 'unknown';
    appendLedger(pr.number, head, 'LANDED', 'land', sha);
    if (regate) appendLedger(pr.number, head, 'REGATE-OWED', 'land', 'reviewer-gate-unavailable');
    say(`#${pr.number} landed as ${sha}`);
  }
  say(`branches now safe to delete (not deleted): ${chain.map((c) => c.headRefName).join(', ')}`);
}

main().catch((e) => refuse(e.message.split('\n')[0]));
