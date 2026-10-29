// Decides whether the protected-path review gate is satisfied (called by .github/workflows/review-gate.yml).
// Rules, all at the exact PR head:
//  - a DISMISSED review is ignored;
//  - a review counts for reviewer N only if its body has exactly one "Written by <X>, a Commonly agent"
//    line, X is N, and its last GATE: line is present (a quoted second credit line makes it count for nobody);
//  - for each named reviewer only the latest counting review counts;
//  - pass iff at least one reviewer's latest is "GATE: PASS" and no reviewer's latest is anything else.
// KNOWN LIMIT: the credit line is a pointer, not an identity (see the workflow header).
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';

const CREDIT = /Written by ([^,\n]+), a Commonly agent/g;

function verdictOf(body) {
  const line = body.split('\n').map((l) => l.trim()).filter((l) => /^GATE:/.test(l)).pop();
  return line ? line.slice(5).trim().split(/\s+/)[0].toUpperCase() : null;
}

export function decide(reviews, { head, names }) {
  const latest = new Map();
  const at = (r) => Date.parse(r.submitted_at || 0) || 0;
  reviews.forEach((r, i) => {
    if (r.commit_id !== head || r.state === 'DISMISSED') return;
    const body = r.body || '';
    const credits = [...body.matchAll(CREDIT)].map((m) => m[1].trim());
    if (credits.length !== 1 || !names.includes(credits[0])) return;
    const verdict = verdictOf(body);
    if (!verdict) return;
    const prev = latest.get(credits[0]);
    if (!prev || at(r) > at(prev.r) || (at(r) === at(prev.r) && i > prev.i)) latest.set(credits[0], { r, i, verdict });
  });
  const verdicts = [...latest].map(([name, { verdict }]) => `${name}: GATE: ${verdict}`);
  if (![...latest.values()].some((v) => v.verdict === 'PASS')) return { pass: false, reason: `no reviewer's latest review at ${head} is GATE: PASS (${verdicts.join('; ') || 'none counted'})` };
  const blocking = [...latest].filter(([, v]) => v.verdict !== 'PASS').map(([n]) => n);
  if (blocking.length) return { pass: false, reason: `latest review from ${blocking.join(', ')} is not GATE: PASS` };
  return { pass: true, reason: `Opus gate found at head (${verdicts.join('; ')})` };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const { HEAD, OPUS_REVIEWERS = '' } = process.env;
  const names = OPUS_REVIEWERS.split(',').map((s) => s.trim()).filter(Boolean);
  const reviews = JSON.parse(fs.readFileSync(process.argv[2], 'utf8')).flat();
  const { pass, reason } = decide(reviews, { head: HEAD, names });
  console.log(pass ? reason : `::error::protocol or security path touched; ${reason}; named reviewers: ${names.join(', ')}`);
  process.exit(pass ? 0 : 1);
}
