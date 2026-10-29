// Pure decisions behind scripts/land.mjs (docs/development/landing.md). Every function takes parsed data and
// returns { ok, reason } (or a plain value); the shell-outs live in land.mjs. Tests: scripts/check-land.mjs.
import { verdictOf } from './review-gate.mjs';

const VERDICTS = ['PASS', 'FAIL', 'FIXED-FWD'];
const SEAT_NOTE = /^reviewer-gate-unavailable/; // TEMPORARY: remove when the reviewers return
const ref = (v) => String(v).replace(/^#/, '');

export function parseLedger(text) {
  const [, ...lines] = text.split('\n').filter((l) => l.trim());
  return lines.map((l) => {
    const [pr, head, verdict, by, when, note = ''] = l.split('\t');
    return { pr: ref(pr), head, verdict, by, when, note };
  });
}

const verifierModel = (by) => /^(.+)-verifier$/.exec(by || '')?.[1] ?? null;

// The last row of the verdicts at (pr, head) decides. Second-verifier rows are not primary verdicts.
function latest(rows, pr, head, { reviewer = false } = {}) {
  return rows.filter((r) => ref(r.pr) === ref(pr) && r.head === head && VERDICTS.includes(r.verdict)
    && SEAT_NOTE.test(r.note) === reviewer).at(-1);
}

// chain: [{ pr, head }] bottom-up. Returns { ok, reason, primary }.
export function checkLedger(rows, { pr, head, chain = [{ pr, head }] }) {
  const row = latest(rows, pr, head);
  if (!row) return { ok: false, reason: `no verifier verdict for #${pr} at ${head.slice(0, 8)}` };
  if (!verifierModel(row.by)) return { ok: false, reason: `#${pr} verdict is by '${row.by}', not a <model>-verifier` };
  if (row.verdict === 'FAIL') return { ok: false, reason: `#${pr} latest verdict at ${head.slice(0, 8)} is FAIL` };
  if (row.verdict === 'PASS') return { ok: true, primary: row };
  const m = /^#(\d+)$/.exec(row.note);
  const at = chain.findIndex((c) => c.pr === pr);
  const target = m && chain.slice(at + 1).find((c) => c.pr === m[1]);
  if (!target) return { ok: false, reason: `#${pr} FIXED-FWD names '${row.note}', which is not above it in this stack` };
  const t = latest(rows, target.pr, target.head);
  if (t?.verdict !== 'PASS' || !verifierModel(t.by)) return { ok: false, reason: `#${pr} FIXED-FWD target #${target.pr} has no verifier PASS at its head` };
  return { ok: true, primary: row };
}

// TEMPORARY (remove when the reviewers return): a second verifier on a different model stands in for the pod verdict.
export function checkSecondVerifier(rows, { pr, head, primary }) {
  const row = latest(rows, pr, head, { reviewer: true });
  if (!row) return { ok: false, reason: `#${pr} touches a protected path: no pod verdict, and no 'reviewer-gate-unavailable' second-verifier row` };
  const model = verifierModel(row.by);
  if (!model) return { ok: false, reason: `#${pr} second verifier '${row.by}' is not a <model>-verifier` };
  if (row.verdict !== 'PASS') return { ok: false, reason: `#${pr} latest second-verifier row is ${row.verdict}` };
  if (model === verifierModel(primary.by)) return { ok: false, reason: `#${pr} second verifier '${row.by}' is on the same model as the first` };
  return { ok: true, reason: `second verifier ${row.by}` };
}

const hasPr = (text, pr) => new RegExp(`jellymoon#${ref(pr)}(?!\\d)`).test(text);
const msgUser = (m) => m.userId?.username ?? m.username;
const msgTime = (m) => Date.parse(m.createdAt ?? m.created_at ?? 0) || 0;

// Verdicts from the pod. Only a message whose server-stamped username is a listed reviewer's counts,
// whatever its text says. The latest qualifying message per reviewer at the head decides.
export function checkPodVerdict(messages, { pr, head, entries, after = 0 }) {
  const byUser = new Map(entries.filter((e) => e.username).map((e) => [e.username, e.name]));
  const latestBy = new Map();
  messages.forEach((m, i) => {
    const name = byUser.get(msgUser(m));
    const text = (m.content || '').replace(/<!--[\s\S]*?-->/g, '');
    if (!name || msgTime(m) < after || !hasPr(text, pr) || !text.toLowerCase().includes(head.toLowerCase())) return;
    const prev = latestBy.get(name);
    if (!prev || msgTime(m) > msgTime(prev.m) || (msgTime(m) === msgTime(prev.m) && i > prev.i)) latestBy.set(name, { m, i });
  });
  const pass = [];
  const block = [];
  for (const [name, { m }] of latestBy) (verdictOf(m.content) === 'PASS' ? pass : block).push(name);
  if (block.length) return { ok: false, reason: `pod verdict blocked by ${block.join(', ')}` };
  if (!pass.length) return { ok: false, reason: `no accepted reviewer's pod message names jellymoon#${ref(pr)} and the full head` };
  return { ok: true, reason: `pod verdict PASS (${pass.join(', ')})` };
}

export function checkHead(pr, local) {
  if (pr.state === 'MERGED') return { ok: true };
  if (pr.state !== 'OPEN') return { ok: false, reason: `#${pr.number} is ${pr.state}` };
  if (local !== pr.headRefOid) return { ok: false, reason: `#${pr.number} head on GitHub ${pr.headRefOid.slice(0, 8)} differs from the local fetch ${String(local).slice(0, 8)}` };
  return { ok: true };
}

// Idempotency: what to do with this PR.
export function landAction(pr, rows) {
  if (pr.state !== 'MERGED') return 'check';
  return rows.some((r) => ref(r.pr) === String(pr.number) && r.head === pr.headRefOid && r.verdict === 'LANDED') ? 'already-landed' : 'record-only';
}

// PRs that still owe a reviewer re-gate. regated(pr, head, since) says a reviewer's pod verdict PASS exists after that row.
export function owedList(rows, regated) {
  const seen = new Set();
  return rows.filter((r) => r.verdict === 'REGATE-OWED' && !seen.has(`${ref(r.pr)}@${r.head}`) && seen.add(`${ref(r.pr)}@${r.head}`)
    && !regated(ref(r.pr), r.head, Date.parse(r.when) || 0));
}

// The one git remote whose URL is the target repo (https or ssh, with or without .git), from `git remote -v`.
export function remoteFor(remoteV, repo) {
  const esc = repo.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const url = new RegExp(`^(https://([^@/]+@)?github\\.com/|git@github\\.com:|ssh://git@github\\.com/)${esc}(\\.git)?/?$`, 'i');
  const names = [...new Set(remoteV.split('\n').map((l) => l.split(/\s+/)).filter(([, u]) => u && url.test(u)).map(([n]) => n))];
  if (names.length === 1) return { remote: names[0] };
  return { error: names.length ? `more than one git remote matches ${repo}: ${names.join(', ')}` : `no git remote matches ${repo}` };
}
