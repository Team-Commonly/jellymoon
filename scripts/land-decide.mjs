// Pure decisions behind scripts/land.mjs (docs/development/landing.md). Every function takes parsed data and
// returns { ok, reason } (or a plain value); the shell-outs live in land.mjs. Tests: scripts/check-land.mjs.
import fs from 'node:fs';
import { verdictOf } from './review-gate.mjs';

const KNOWN = ['PASS', 'FAIL', 'LANDED', 'REGATE-OWED', 'BOOTSTRAP'];
const VERIFIER = /^(opus|sonnet|haiku|fable)-verifier$/;
const SEAT_NOTE = /^reviewer-gate-unavailable/; // TEMPORARY: remove when the reviewers return
const ref = (v) => String(v).trim().replace(/^#/, '');

// Rows are normalised (trim, case). The first line is dropped only if it IS the header.
export function parseLedger(text) {
  const lines = text.split('\n').filter((l) => l.trim());
  const rows = /^pr\t/i.test(lines[0] || '') ? lines.slice(1) : lines;
  return rows.map((l) => {
    const [pr = '', head = '', verdict = '', by = '', when = '', note = ''] = l.split('\t');
    return { pr: ref(pr), head: head.trim().toLowerCase(), verdict: verdict.trim().toUpperCase(), by: by.trim().toLowerCase(), when: when.trim(), note: note.trim() };
  });
}

const family = (by) => VERIFIER.exec(by || '')?.[1] ?? null;
const at = (rows, pr, head) => rows.filter((r) => r.pr === ref(pr) && r.head === head.toLowerCase());

// Fails closed. At (pr, head): an unrecognised verdict blocks; a verdict row by a non-verifier blocks; a FAIL
// blocks (whatever its note) unless the SAME label recorded a later PASS; the latest primary row must be a PASS.
export function checkLedger(rows, { pr, head }) {
  const mine = at(rows, pr, head);
  const odd = mine.find((r) => !KNOWN.includes(r.verdict));
  if (odd) return { ok: false, reason: `#${pr} has an unrecognised verdict '${odd.verdict}' at ${head.slice(0, 8)}` };
  const verdicts = mine.filter((r) => ['PASS', 'FAIL'].includes(r.verdict));
  const notV = verdicts.find((r) => !family(r.by));
  if (notV) return { ok: false, reason: `#${pr} has a ${notV.verdict} by '${notV.by}', which is not a <model>-verifier` };
  const failed = verdicts.find((r, i) => r.verdict === 'FAIL' && !verdicts.slice(i + 1).some((p) => p.verdict === 'PASS' && p.by === r.by));
  if (failed) return { ok: false, reason: `#${pr} has a FAIL by ${failed.by} at ${head.slice(0, 8)} with no later PASS from the same verifier` };
  const primary = verdicts.filter((r) => !SEAT_NOTE.test(r.note)).at(-1);
  if (primary?.verdict !== 'PASS') return { ok: false, reason: `no verifier PASS for #${pr} at ${head.slice(0, 8)}` };
  return { ok: true, primary };
}

// TEMPORARY (remove when the reviewers return): a second verifier from a different model family stands in
// for the pod verdict.
export function checkSecondVerifier(rows, { pr, head, primary }) {
  const row = at(rows, pr, head).filter((r) => ['PASS', 'FAIL'].includes(r.verdict) && SEAT_NOTE.test(r.note)).at(-1);
  if (!row) return { ok: false, reason: `#${pr} touches a protected path: no pod verdict, and no 'reviewer-gate-unavailable' second-verifier row` };
  if (row.verdict !== 'PASS') return { ok: false, reason: `#${pr} latest second-verifier row is ${row.verdict}` };
  if (family(row.by) === family(primary.by)) return { ok: false, reason: `#${pr} second verifier '${row.by}' is the same model family as the first` };
  return { ok: true, reason: `second verifier ${row.by}` };
}

const msgUser = (m) => m.userId?.username ?? m.username;
const msgTime = (m) => Date.parse(m.createdAt ?? m.created_at ?? 0) || 0;

// A message counts for #N only if its FIRST non-empty line is exactly `jellymoon#<N> <head>` and its server-stamped
// username is a listed reviewer's. The latest such message per reviewer decides; text is never an identity.
export function checkPodVerdict(messages, { pr, head, entries, after = 0 }) {
  const byUser = new Map(entries.filter((e) => e.username).map((e) => [e.username, e.name]));
  const first = `jellymoon#${ref(pr)} ${head.toLowerCase()}`;
  const latestBy = new Map();
  messages.forEach((m, i) => {
    const name = byUser.get(msgUser(m));
    const line = (m.content || '').split('\n').map((l) => l.trim()).find(Boolean);
    if (!name || msgTime(m) < after || line?.toLowerCase() !== first) return;
    const prev = latestBy.get(name);
    if (!prev || msgTime(m) > msgTime(prev.m) || (msgTime(m) === msgTime(prev.m) && i > prev.i)) latestBy.set(name, { m, i });
  });
  const pass = [];
  const block = [];
  for (const [name, { m }] of latestBy) (verdictOf(m.content) === 'PASS' ? pass : block).push(name);
  if (block.length) return { ok: false, reason: `pod verdict blocked by ${block.join(', ')}` };
  if (!pass.length) return { ok: false, reason: `no accepted reviewer's pod message starts with 'jellymoon#${ref(pr)} <full head>'` };
  return { ok: true, reason: `pod verdict PASS (${pass.join(', ')})` };
}

export function checkHead(pr, local) {
  if (pr.state !== 'OPEN') return { ok: false, reason: `#${pr.number} is ${pr.state}` };
  if (pr.baseRefName !== 'main') return { ok: false, reason: `#${pr.number} is based on ${pr.baseRefName}, not main; land one PR retargeted to main` };
  if (String(local).toLowerCase() !== pr.headRefOid.toLowerCase()) return { ok: false, reason: `#${pr.number} head on GitHub ${pr.headRefOid.slice(0, 8)} differs from the local fetch ${String(local).slice(0, 8)}` };
  return { ok: true };
}

// Idempotency: what to do with this PR.
export function landAction(pr, rows) {
  if (pr.state !== 'MERGED') return 'check';
  return at(rows, pr.number, pr.headRefOid).some((r) => r.verdict === 'LANDED') ? 'already-landed' : 'record-only';
}

// PRs that still owe a reviewer re-gate. regated(pr, head, since) says a reviewer's pod PASS exists after that row.
export function owedList(rows, regated) {
  const seen = new Set();
  return rows.filter((r) => r.verdict === 'REGATE-OWED' && !seen.has(`${r.pr}@${r.head}`) && seen.add(`${r.pr}@${r.head}`)
    && !regated(r.pr, r.head, Date.parse(r.when) || 0));
}

// `git diff --name-status -M` output to the shape touchesProtected reads: renames and copies list both names.
export function filesFromNameStatus(text) {
  return text.split('\n').filter(Boolean).map((l) => {
    const [status, a, b] = l.split('\t');
    return /^[RC]/.test(status) ? { filename: b, previous_filename: a } : { filename: a };
  });
}

export const checkMain = (before, after) => (before === after ? { ok: true }
  : { ok: false, reason: `main moved from ${before.slice(0, 8)} to ${after.slice(0, 8)} during the run; run land again` });

export const checkApi = (url) => (/^https:\/\//.test(url) ? { ok: true } : { ok: false, reason: 'JM_COMMONLY_API must be https (the bearer token would travel in clear)' });

// The one git remote whose URL is the target repo (https or ssh, with or without .git), from `git remote -v`.
export function remoteFor(remoteV, repo) {
  const esc = repo.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const url = new RegExp(`^(https://([^@/]+@)?github\\.com/|git@github\\.com:|ssh://git@github\\.com/)${esc}(\\.git)?/?$`, 'i');
  const names = [...new Set(remoteV.split('\n').map((l) => l.split(/\s+/)).filter(([, u]) => u && url.test(u)).map(([n]) => n))];
  if (names.length === 1) return { remote: names[0] };
  return { error: names.length ? `more than one git remote matches ${repo}: ${names.join(', ')}` : `no git remote matches ${repo}` };
}

// Removes a throwaway tree. Go's module cache is read-only, so make it writable first. Never throws: a failed
// cleanup warns with the directory and the caller's result stands.
export function removeTree(dir, { rm = (d) => fs.rmSync(d, { recursive: true, force: true }), warn = console.error } = {}) {
  const writable = (d) => {
    try {
      fs.chmodSync(d, 0o700);
      for (const e of fs.readdirSync(d, { withFileTypes: true })) if (e.isDirectory() && !e.isSymbolicLink()) writable(`${d}/${e.name}`); else if (!e.isSymbolicLink()) fs.chmodSync(`${d}/${e.name}`, 0o600);
    } catch { /* best effort */ }
  };
  try { writable(dir); rm(dir); return true; } catch (e) { warn(`land: warning: could not remove ${dir}: ${e.message.split('\n')[0]}`); return false; }
}
