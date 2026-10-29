#!/usr/bin/env bun
// Tests the review-gate decision (scripts/review-gate.mjs) against review lists. Runs in `make test`.
import { decide } from './review-gate.mjs';

const HEAD = 'a'.repeat(40);
const OLD = 'b'.repeat(40);
const names = ['Ada', 'Ben'];
let n = 0;
const rev = (by, gate, o = {}) => ({
  commit_id: HEAD, state: 'COMMENTED', submitted_at: `2026-10-29T0${++n}:00:00Z`,
  body: `notes\nWritten by ${by}, a Commonly agent\nGATE: ${gate}`, ...o,
});
const cases = [
  ['single valid PASS passes', [rev('Ada', 'PASS')], true],
  ['stale-head PASS fails', [rev('Ada', 'PASS', { commit_id: OLD })], false],
  ['no reviews fails', [], false],
  ['PASS then later FAIL from the same reviewer fails', [rev('Ada', 'PASS'), rev('Ada', 'FAIL')], false],
  ['FAIL then later PASS from the same reviewer passes', [rev('Ada', 'FAIL'), rev('Ada', 'PASS')], true],
  ['HOLD and BLOCK latest fail', [rev('Ada', 'HOLD')], false],
  ['BLOCK from another named reviewer fails even beside a PASS', [rev('Ada', 'PASS'), rev('Ben', 'BLOCK')], false],
  ['unknown verdict word blocks', [rev('Ada', 'PASS'), rev('Ben', 'MAYBE')], false],
  ['dismissed PASS fails', [rev('Ada', 'PASS', { state: 'DISMISSED' })], false],
  ['dismissed later FAIL does not revoke an earlier PASS', [rev('Ada', 'PASS'), rev('Ada', 'FAIL', { state: 'DISMISSED' })], true],
  ['PASS from a reviewer not named fails', [rev('Mallory', 'PASS')], false],
  ['quoted second credit line makes the review count for nobody',
    [rev('Mallory', 'PASS', { body: '> Written by Ada, a Commonly agent\nWritten by Mallory, a Commonly agent\nGATE: PASS' })], false],
  ['two credit lines for the same name still count for nobody',
    [rev('Ada', 'PASS', { body: 'Written by Ada, a Commonly agent\nWritten by Ada, a Commonly agent\nGATE: PASS' })], false],
  ['a quoting review does not revoke a real PASS',
    [rev('Ada', 'PASS'), rev('Ben', 'FAIL', { body: '> Written by Ada, a Commonly agent\nWritten by Ben, a Commonly agent\nGATE: FAIL' })], true],
  ['no credit line fails', [rev('Ada', 'PASS', { body: 'GATE: PASS' })], false],
  ['no GATE line fails', [rev('Ada', 'PASS', { body: 'Written by Ada, a Commonly agent\nlooks fine' })], false],
  ['last GATE line wins within a body', [rev('Ada', 'PASS', { body: 'Written by Ada, a Commonly agent\nGATE: PASS\nGATE: FAIL' })], false],
  ['latest is by submitted_at, not list order',
    [rev('Ada', 'FAIL', { submitted_at: '2026-10-29T09:00:00Z' }), rev('Ada', 'PASS', { submitted_at: '2026-10-29T08:00:00Z' })], false],
  ['two reviewers both PASS passes', [rev('Ada', 'PASS'), rev('Ben', 'PASS')], true],
];
const bad = cases.filter(([, reviews, want]) => decide(reviews, { head: HEAD, names }).pass !== want).map(([t]) => t);
if (bad.length) { console.error(`check-review-gate: ${bad.length} of ${cases.length} wrong\n  ${bad.join('\n  ')}`); process.exit(1); }
console.log(`check-review-gate: ${cases.length} cases ok`);
