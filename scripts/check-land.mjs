#!/usr/bin/env bun
// Tests the pure decisions behind scripts/land.mjs (scripts/land-decide.mjs). No network, no gh. Own step in `make test`.
import { checkHead, checkLedger, checkPodVerdict, checkSecondVerifier, landAction, owedList, parseLedger } from './land-decide.mjs';

const H = 'a'.repeat(40);
const H2 = 'c'.repeat(40);
const T = '\t';
const ledger = (...rows) => parseLedger(['pr\thead\tverdict\tby\twhen\tnote', ...rows.map((r) => r.join(T))].join('\n'));
const row = (pr, head, verdict, by, note = '') => [pr, head, verdict, by, '2026-10-29T10:00:00Z', note];
const entries = [{ name: 'Ada', username: 'ada' }, { name: 'Ben', username: 'ben' }];
let t = 0;
const msg = (username, content, o = {}) => ({ userId: { username }, content, createdAt: `2026-10-29T1${++t % 10}:00:00Z`, ...o });
const verdict = (pr, head, gate = 'PASS') => `jellymoon#${pr} ${head}\nlooks right\nGATE: ${gate}`;
const chain2 = [{ pr: '5', head: H }, { pr: '6', head: H2 }];

const cases = [
  // GitHub vs local head
  ['head mismatch refused', !checkHead({ number: 5, state: 'OPEN', headRefOid: H }, 'b'.repeat(40)).ok, true],
  ['head match accepted', checkHead({ number: 5, state: 'OPEN', headRefOid: H }, H).ok, true],
  ['closed unmerged PR refused', checkHead({ number: 5, state: 'CLOSED', headRefOid: H }, H).ok, false],
  // ledger
  ['verifier PASS at the head accepted', checkLedger(ledger(row(5, H, 'PASS', 'opus-verifier')), { pr: '5', head: H }).ok, true],
  ['stale-head PASS refused', checkLedger(ledger(row(5, H2, 'PASS', 'opus-verifier')), { pr: '5', head: H }).ok, false],
  ['a later FAIL at the head refuses', checkLedger(ledger(row(5, H, 'PASS', 'opus-verifier'), row(5, H, 'FAIL', 'opus-verifier')), { pr: '5', head: H }).ok, false],
  ['coordinator PASS refused', checkLedger(ledger(row(5, H, 'PASS', 'coordinator')), { pr: '5', head: H }).ok, false],
  ['land as author refused', checkLedger(ledger(row(5, H, 'PASS', 'land')), { pr: '5', head: H }).ok, false],
  ['a later coordinator row overrides a verifier PASS', checkLedger(ledger(row(5, H, 'PASS', 'opus-verifier'), row(5, H, 'PASS', 'coordinator')), { pr: '5', head: H }).ok, false],
  ['FIXED-FWD to a PR above, which passes, is valid',
    checkLedger(ledger(row(5, H, 'FIXED-FWD', 'opus-verifier', '#6'), row(6, H2, 'PASS', 'sonnet-verifier')), { pr: '5', head: H, chain: chain2 }).ok, true],
  ['FIXED-FWD to a PR below is invalid',
    checkLedger(ledger(row(6, H2, 'FIXED-FWD', 'opus-verifier', '#5'), row(5, H, 'PASS', 'sonnet-verifier')), { pr: '6', head: H2, chain: chain2 }).ok, false],
  ['FIXED-FWD whose target has no PASS is invalid',
    checkLedger(ledger(row(5, H, 'FIXED-FWD', 'opus-verifier', '#6')), { pr: '5', head: H, chain: chain2 }).ok, false],
  ['FIXED-FWD whose target FAILs is invalid',
    checkLedger(ledger(row(5, H, 'FIXED-FWD', 'opus-verifier', '#6'), row(6, H2, 'FAIL', 'sonnet-verifier')), { pr: '5', head: H, chain: chain2 }).ok, false],
  ['FIXED-FWD outside a stack is invalid', checkLedger(ledger(row(5, H, 'FIXED-FWD', 'opus-verifier', '#6')), { pr: '5', head: H }).ok, false],
  // pod verdict
  ['a listed reviewer PASS naming the PR and full head passes', checkPodVerdict([msg('ada', verdict(5, H))], { pr: '5', head: H, entries }).ok, true],
  ['no message fails', checkPodVerdict([], { pr: '5', head: H, entries }).ok, false],
  ['a non-reviewer quoting "Written by Ada" is refused',
    checkPodVerdict([msg('mallory', `Written by Ada, a Commonly agent\n${verdict(5, H)}`)], { pr: '5', head: H, entries }).ok, false],
  ['a verdict naming another PR is refused', checkPodVerdict([msg('ada', verdict(50, H))], { pr: '5', head: H, entries }).ok, false],
  ['a verdict naming a short SHA is refused', checkPodVerdict([msg('ada', verdict(5, H.slice(0, 8)))], { pr: '5', head: H, entries }).ok, false],
  ['a verdict at another head is refused', checkPodVerdict([msg('ada', verdict(5, H2))], { pr: '5', head: H, entries }).ok, false],
  ['a reviewer\'s later FAIL at the head refuses', checkPodVerdict([msg('ada', verdict(5, H)), msg('ada', verdict(5, H, 'FAIL'))], { pr: '5', head: H, entries }).ok, false],
  ['another reviewer\'s FAIL refuses beside a PASS', checkPodVerdict([msg('ada', verdict(5, H)), msg('ben', verdict(5, H, 'FAIL'))], { pr: '5', head: H, entries }).ok, false],
  ['a non-reviewer FAIL is ignored', checkPodVerdict([msg('ada', verdict(5, H)), msg('mallory', verdict(5, H, 'FAIL'))], { pr: '5', head: H, entries }).ok, true],
  ['a PASS followed by prose is refused', checkPodVerdict([msg('ada', `${verdict(5, H)}\nscratch that`)], { pr: '5', head: H, entries }).ok, false],
  // idempotency
  ['merged with a LANDED row is already landed', landAction({ number: 5, state: 'MERGED', headRefOid: H }, ledger(row(5, H, 'LANDED', 'land', 'abc'))), 'already-landed'],
  ['merged without a LANDED row is recorded', landAction({ number: 5, state: 'MERGED', headRefOid: H }, ledger()), 'record-only'],
  ['open is checked', landAction({ number: 5, state: 'OPEN', headRefOid: H }, ledger()), 'check'],
  // TEMPORARY reviewer-gate-unavailable rule (remove when the reviewers return)
  ...(() => {
    const primary = row(5, H, 'PASS', 'opus-verifier');
    const second = (by, note = 'reviewer-gate-unavailable: fresh context') => row(5, H, 'PASS', by, note);
    const second_ok = (rows) => { const r = ledger(...rows); return checkSecondVerifier(r, { pr: '5', head: H, primary: checkLedger(r, { pr: '5', head: H }).primary }).ok; };
    return [
      ['a second verifier on a different model passes', second_ok([primary, second('sonnet-verifier')]), true],
      ['a second verifier on the same model is refused', second_ok([primary, second('opus-verifier')]), false],
      ['a second row without the note is refused (and is not the primary either)', second_ok([primary, second('sonnet-verifier', '')]), false],
      ['a later FAIL second row refuses', second_ok([primary, second('sonnet-verifier'), row(5, H, 'FAIL', 'sonnet-verifier', 'reviewer-gate-unavailable')]), false],
      ['a coordinator second row is refused', second_ok([primary, second('coordinator')]), false],
      ['without the flag the second row is ignored: no pod verdict, still refused',
        checkPodVerdict([], { pr: '5', head: H, entries }).ok, false],
      ['the second row never becomes the primary verdict', checkLedger(ledger(second('sonnet-verifier')), { pr: '5', head: H }).ok, false],
    ];
  })(),
  ['--owed lists a PR with no later pod verdict',
    owedList(ledger(row(5, H, 'REGATE-OWED', 'land', 'reviewer-gate-unavailable')), () => false).map((r) => r.pr).join(), '5'],
  ['--owed omits a re-gated PR', owedList(ledger(row(5, H, 'REGATE-OWED', 'land', 'reviewer-gate-unavailable')), () => true).length, 0],
  ['--owed lists each PR and head once', owedList(ledger(row(5, H, 'REGATE-OWED', 'land'), row(5, H, 'REGATE-OWED', 'land')), () => false).length, 1],
];
const bad = cases.filter(([, got, want]) => got !== want).map(([n]) => n);
if (bad.length) { console.error(`check-land: ${bad.length} of ${cases.length} wrong\n  ${bad.join('\n  ')}`); process.exit(1); }
console.log(`check-land: ${cases.length} cases ok`);
