#!/usr/bin/env bun
// Tests the review-gate decision (scripts/review-gate.mjs) against review lists. Own step in `make test`.
import fs from 'node:fs';
import { decide, readReviewerEntries, readReviewers, touchesProtected } from './review-gate.mjs';

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
  ['a blockquoted credit line is ignored: the review counts for its own author only',
    [rev('Ada', 'PASS'), rev('Mallory', 'PASS', { body: '> Written by Ada, a Commonly agent\nWritten by Mallory, a Commonly agent\nGATE: PASS' })], true],
  ['an unlisted reviewer quoting a listed credit does not pass alone',
    [rev('Mallory', 'PASS', { body: '> Written by Ada, a Commonly agent\nWritten by Mallory, a Commonly agent\nGATE: PASS' })], false],
  ['two credit lines for the same name still credit that name', [rev('Ada', 'PASS', { body: 'Written by Ada, a Commonly agent\nWritten by Ada, a Commonly agent\nGATE: PASS' })], true],
  ['a FAIL quoting another listed credit (unquoted) blocks beside a PASS',
    [rev('Ada', 'PASS'), rev('Ben', 'FAIL', { body: 'Written by Ada, a Commonly agent\nWritten by Ben, a Commonly agent\nGATE: FAIL' })], false],
  ['a review credited to two names blocks even if it says PASS',
    [rev('Ada', 'PASS', { body: 'Written by Ada, a Commonly agent\nWritten by Ben, a Commonly agent\nGATE: PASS' })], false],
  ['a FAIL that blockquotes another credit blocks beside a PASS',
    [rev('Ada', 'PASS'), rev('Ben', 'FAIL', { body: '> Written by Ada, a Commonly agent\nWritten by Ben, a Commonly agent\nGATE: FAIL' })], false],
  ['PASS then CHANGES_REQUESTED with no GATE line fails',
    [rev('Ada', 'PASS'), rev('Ada', 'x', { state: 'CHANGES_REQUESTED', body: 'Written by Ada, a Commonly agent\nfound a bug, rechecking' })], false],
  ['CHANGES_REQUESTED blocks even with GATE: PASS', [rev('Ada', 'PASS', { state: 'CHANGES_REQUESTED' })], false],
  ['PASS then bold FAIL fails', [rev('Ada', 'PASS'), rev('Ada', 'x', { body: 'Written by Ada, a Commonly agent\n**GATE: FAIL**' })], false],
  ['PASS then lowercase fail fails', [rev('Ada', 'PASS'), rev('Ada', 'x', { body: 'Written by Ada, a Commonly agent\ngate: fail' })], false],
  ['bold PASS passes', [rev('Ada', 'x', { body: 'Written by Ada, a Commonly agent\n**GATE: PASS**' })], true],
  ['PASS only inside a code fence fails',
    [rev('Ada', 'x', { body: 'Written by Ada, a Commonly agent\n```\nGATE: PASS\n```' })], false],
  ['a fenced FAIL under a real PASS is ignored',
    [rev('Ada', 'x', { body: 'Written by Ada, a Commonly agent\n```\nGATE: FAIL\n```\nGATE: PASS' })], true],
  ['GATE: PASS with trailing words is not exactly PASS', [rev('Ada', 'PASS but check X')], false],
  ['a credited latest review with no GATE line blocks a PASS before it',
    [rev('Ada', 'PASS'), rev('Ada', 'x', { body: 'Written by Ada, a Commonly agent\nlooks fine' })], false],
  ['no credit line fails', [rev('Ada', 'PASS', { body: 'GATE: PASS' })], false],
  ['no GATE line fails', [rev('Ada', 'PASS', { body: 'Written by Ada, a Commonly agent\nlooks fine' })], false],
  ['last GATE line wins within a body', [rev('Ada', 'PASS', { body: 'Written by Ada, a Commonly agent\nGATE: PASS\nGATE: FAIL' })], false],
  ['latest is by submitted_at, not list order',
    [rev('Ada', 'FAIL', { submitted_at: '2026-10-29T09:00:00Z' }), rev('Ada', 'PASS', { submitted_at: '2026-10-29T08:00:00Z' })], false],
  ['two reviewers both PASS passes', [rev('Ada', 'PASS'), rev('Ben', 'PASS')], true],
  ['lowercase gate: pass passes', [rev('Ada', 'x', { body: 'Written by Ada, a Commonly agent\ngate: pass' })], true],
  ['a heading FAIL after the PASS blocks', [rev('Ada', 'x', { body: 'Written by Ada, a Commonly agent\nGATE: PASS\n## GATE: FAIL' })], false],
  ['a list FAIL after the PASS blocks', [rev('Ada', 'x', { body: 'Written by Ada, a Commonly agent\nGATE: PASS\n- GATE: FAIL' })], false],
  ['a zero-width-space FAIL after the PASS blocks', [rev('Ada', 'x', { body: 'Written by Ada, a Commonly agent\nGATE: PASS\n\u200bGATE: FAIL' })], false],
  ['a fullwidth colon FAIL after the PASS blocks', [rev('Ada', 'x', { body: 'Written by Ada, a Commonly agent\nGATE: PASS\nGATE\uff1a FAIL' })], false],
  ['GATE : FAIL after the PASS blocks', [rev('Ada', 'x', { body: 'Written by Ada, a Commonly agent\nGATE: PASS\nGATE : FAIL' })], false],
  ['prose after the PASS blocks', [rev('Ada', 'x', { body: 'Written by Ada, a Commonly agent\nGATE: PASS\n\nScratch that: found a token leak, FAIL.' })], false],
  ['a PASS hidden in an HTML comment after a FAIL blocks', [rev('Ada', 'x', { body: 'Written by Ada, a Commonly agent\nGATE: FAIL\n<!-- GATE: PASS -->' })], false],
  ['a PASS in an indented code block after a FAIL blocks', [rev('Ada', 'x', { body: 'Written by Ada, a Commonly agent\nGATE: FAIL\n\n    GATE: PASS' })], false],
  ['a ~~~ line inside a ``` fence does not close it', [rev('Ada', 'x', { body: '```\n~~~\nWritten by Ada, a Commonly agent\n```\nGATE: PASS' })], false],
  ['a malformed credit (bold, lowercase) still credits the reviewer', [rev('Ada', 'x', { body: 'Written by **ada**, a Commonly agent\nGATE: PASS' })], true],
  ['a malformed credit on a FAIL cannot hide behind an earlier PASS', [rev('Ada', 'PASS'), rev('Ada', 'x', { body: 'Written by [Ada](http://x)\u00a0, a Commonly agent\nGATE: FAIL' })], false],
  ['an unrecognised credit on a FAIL blocks beside a PASS', [rev('Ada', 'PASS'), rev('Mallory', 'FAIL')], false],
];
const paths = [
  ['a schema edit is protected', [{ filename: 'schema/x.json' }], true],
  ['a workflow edit is protected', [{ filename: '.github/workflows/ci.yml' }], true],
  ['the gate script and reviewer list are protected', [{ filename: 'scripts/review-gate.mjs' }, { filename: '.github/opus-reviewers.txt' }], true],
  ['a rename OUT of schema/ is protected (previous_filename)', [{ filename: 'elsewhere/x.json', previous_filename: 'schema/x.json' }], true],
  ['a rename INTO schema/ is protected', [{ filename: 'schema/x.json', previous_filename: 'elsewhere/x.json' }], true],
  ['an unrelated edit is not protected', [{ filename: 'docs/INDEX.md' }, { filename: 'scripts/gen.mjs' }], false],
  ['a rename between unprotected paths is not protected', [{ filename: 'docs/b.md', previous_filename: 'docs/a.md' }], false],
];
const badPaths = paths.filter(([, files, want]) => touchesProtected(files) !== want).map(([t]) => t);
if (badPaths.length) { console.error(`check-review-gate: protected-path cases wrong\n  ${badPaths.join('\n  ')}`); process.exit(1); }
const text = fs.readFileSync(new URL('../.github/opus-reviewers.txt', import.meta.url), 'utf8');
const entries = readReviewerEntries(text);
const fileOk = entries.length > 0 && entries.every((e) => e.name && /^[a-z0-9][a-z0-9-]*$/.test(e.username || '')) && readReviewers(text).length === entries.length;
if (!fileOk) { console.error(`check-review-gate: .github/opus-reviewers.txt does not parse as 'Name username' lines: ${JSON.stringify(entries)}`); process.exit(1); }
const bad = cases.filter(([, reviews, want]) => decide(reviews, { head: HEAD, names }).pass !== want).map(([t]) => t);
if (bad.length) { console.error(`check-review-gate: ${bad.length} of ${cases.length} wrong\n  ${bad.join('\n  ')}`); process.exit(1); }
console.log(`check-review-gate: ${cases.length} gate cases and ${paths.length} path cases ok`);
