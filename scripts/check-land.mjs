#!/usr/bin/env bun
// Tests the pure decisions behind scripts/land.mjs (scripts/land-decide.mjs). No network, no gh. Own step in `make test`.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {
  checkApi, checkHead, checkLedger, checkMain, checkPodVerdict, checkSecondVerifier, filesFromNameStatus,
  landAction, owedList, parseLedger, remoteFor, removeTree,
} from './land-decide.mjs';

const H = 'a'.repeat(40);
const H2 = 'c'.repeat(40);
const HEADER = 'pr\thead\tverdict\tby\twhen\tnote';
const ledger = (...rows) => parseLedger([HEADER, ...rows.map((r) => r.join('\t'))].join('\n'));
const row = (pr, head, verdict, by, note = '') => [pr, head, verdict, by, '2026-10-29T10:00:00Z', note];
const entries = [{ name: 'Ada', username: 'ada' }, { name: 'Ben', username: 'ben' }];
let t = 0;
const msg = (username, content, o = {}) => ({ userId: { username }, content, createdAt: `2026-10-29T1${++t % 10}:00:00Z`, ...o });
const verdict = (pr, head, gate = 'PASS') => `jellymoon#${pr} ${head}\nlooks right\nGATE: ${gate}`;
const pod = (msgs, pr = '5', head = H, o = {}) => checkPodVerdict(msgs, { pr, head, entries, ...o });
const led = (rows, head = H) => checkLedger(rows, { pr: '5', head });
const open = (o = {}) => ({ number: 5, state: 'OPEN', baseRefName: 'main', headRefOid: H, ...o });
const R = 'Team-Commonly/jellymoon';
const rem = (...l) => l.map(([n, u]) => `${n}\t${u} (fetch)\n${n}\t${u} (push)`).join('\n');

const cases = [
  // GitHub vs local head, base
  ['head mismatch refused', checkHead(open(), 'b'.repeat(40)).ok, false],
  ['head match on a main-based open PR accepted', checkHead(open(), H).ok, true],
  ['closed unmerged PR refused', checkHead(open({ state: 'CLOSED' }), H).ok, false],
  ['a PR not based on main is refused', checkHead(open({ baseRefName: 'p05' }), H).ok, false],
  // ledger
  ['verifier PASS at the head accepted', led(ledger(row(5, H, 'PASS', 'opus-verifier'))).ok, true],
  ['stale-head PASS refused', led(ledger(row(5, H2, 'PASS', 'opus-verifier'))).ok, false],
  ['a later FAIL at the head refuses', led(ledger(row(5, H, 'PASS', 'opus-verifier'), row(5, H, 'FAIL', 'opus-verifier'))).ok, false],
  ['a FAIL superseded by a later PASS from the same verifier is accepted', led(ledger(row(5, H, 'FAIL', 'opus-verifier'), row(5, H, 'PASS', 'opus-verifier'))).ok, true],
  ['a FAIL from one verifier is not superseded by another verifier\'s PASS', led(ledger(row(5, H, 'FAIL', 'opus-verifier'), row(5, H, 'PASS', 'sonnet-verifier'))).ok, false],
  ['a FAIL with a reviewer-gate note still blocks', led(ledger(row(5, H, 'PASS', 'opus-verifier'), row(5, H, 'FAIL', 'sonnet-verifier', 'reviewer-gate-unavailable'))).ok, false],
  ['coordinator PASS refused', led(ledger(row(5, H, 'PASS', 'coordinator'))).ok, false],
  ['a coordinator PASS beside a verifier PASS still refuses', led(ledger(row(5, H, 'PASS', 'opus-verifier'), row(5, H, 'PASS', 'coordinator'))).ok, false],
  ['land as a verdict author refused', led(ledger(row(5, H, 'PASS', 'land'))).ok, false],
  ['an unrecognised verdict at the head blocks', led(ledger(row(5, H, 'PASS', 'opus-verifier'), row(5, H, 'fixed-fwd', 'opus-verifier', '#6'))).ok, false],
  ['a lowercase fail with padding is normalised and blocks', led(ledger(row(5, H, 'PASS', 'opus-verifier'), row(5, H.toUpperCase(), ' fail ', 'Opus-Verifier '))).ok, false],
  ['an unknown label such as opus-5-verifier is not a verifier', led(ledger(row(5, H, 'PASS', 'opus-5-verifier'))).ok, false],
  ['coordinator-verifier is not a verifier', led(ledger(row(5, H, 'PASS', 'coordinator-verifier'))).ok, false],
  ['#5 rows do not satisfy #50', checkLedger(ledger(row(5, H, 'PASS', 'opus-verifier')), { pr: '50', head: H }).ok, false],
  ['a headerless ledger keeps its first row', parseLedger(row(5, H, 'PASS', 'opus-verifier').join('\t')).length, 1],
  ['a header-first ledger drops only the header', ledger(row(5, H, 'PASS', 'opus-verifier')).length, 1],
  // pod verdict
  ['a listed reviewer PASS naming the PR and full head passes', pod([msg('ada', verdict(5, H))]).ok, true],
  ['no message fails', pod([]).ok, false],
  ['a non-reviewer quoting "Written by Ada" is refused', pod([msg('mallory', `Written by Ada, a Commonly agent\n${verdict(5, H)}`)]).ok, false],
  ['a verdict naming another PR is refused', pod([msg('ada', verdict(50, H))]).ok, false],
  ['a verdict naming a short SHA is refused', pod([msg('ada', verdict(5, H.slice(0, 8)))]).ok, false],
  ['a verdict at another head is refused', pod([msg('ada', verdict(5, H2))]).ok, false],
  ['a #11 verdict that cites #10 and its head is not a #10 PASS',
    pod([msg('ada', `jellymoon#11 ${H2}\nbase jellymoon#10 at ${H}\nGATE: PASS`)], '10', H).ok, false],
  ['a #11 PASS citing #10 cannot override the reviewer\'s earlier #10 FAIL',
    pod([msg('ada', verdict(10, H, 'FAIL')), msg('ada', `jellymoon#11 ${H2}\nbase jellymoon#10 at ${H}\nGATE: PASS`)], '10', H).ok, false],
  ['a hidden-comment first line does not count', pod([msg('ada', `<!-- jellymoon#5 ${H} -->\nGATE: PASS`)]).ok, false],
  ['a leading blank line and padding are trimmed', pod([msg('ada', `\n  jellymoon#5 ${H}  \nGATE: PASS`)]).ok, true],
  ['a reviewer\'s later FAIL at the head refuses', pod([msg('ada', verdict(5, H)), msg('ada', verdict(5, H, 'FAIL'))]).ok, false],
  ['another reviewer\'s FAIL refuses beside a PASS', pod([msg('ada', verdict(5, H)), msg('ben', verdict(5, H, 'FAIL'))]).ok, false],
  ['a non-reviewer FAIL is ignored', pod([msg('ada', verdict(5, H)), msg('mallory', verdict(5, H, 'FAIL'))]).ok, true],
  ['a PASS followed by prose is refused', pod([msg('ada', `${verdict(5, H)}\nscratch that`)]).ok, false],
  ['a message older than `after` does not count', pod([msg('ada', verdict(5, H), { createdAt: '2026-10-29T09:00:00Z' })], '5', H, { after: Date.parse('2026-10-29T10:00:00Z') }).ok, false],
  ['a message after `after` counts', pod([msg('ada', verdict(5, H), { createdAt: '2026-10-29T11:00:00Z' })], '5', H, { after: Date.parse('2026-10-29T10:00:00Z') }).ok, true],
  // idempotency
  ['merged with a LANDED row at the head is already landed', landAction({ number: 5, state: 'MERGED', headRefOid: H }, ledger(row(5, H, 'LANDED', 'land', 'abc'))), 'already-landed'],
  ['a LANDED row at another head does not count', landAction({ number: 5, state: 'MERGED', headRefOid: H }, ledger(row(5, H2, 'LANDED', 'land', 'abc'))), 'record-only'],
  ['merged without a LANDED row is recorded', landAction({ number: 5, state: 'MERGED', headRefOid: H }, ledger()), 'record-only'],
  ['open is checked', landAction(open(), ledger()), 'check'],
  // TEMPORARY reviewer-gate-unavailable rule (remove when the reviewers return)
  ...(() => {
    const primary = row(5, H, 'PASS', 'opus-verifier');
    const second = (by, note = 'reviewer-gate-unavailable: fresh context') => row(5, H, 'PASS', by, note);
    const ok = (rows) => { const r = ledger(...rows); const l = led(r); return l.ok && checkSecondVerifier(r, { pr: '5', head: H, primary: l.primary }).ok; };
    return [
      ['a second verifier on a different model passes', ok([primary, second('sonnet-verifier')]), true],
      ['a second verifier on the same model is refused', ok([primary, second('opus-verifier')]), false],
      ['a second row without the note is refused', ok([primary, second('sonnet-verifier', '')]), false],
      ['a later FAIL second row refuses', ok([primary, second('sonnet-verifier'), row(5, H, 'FAIL', 'sonnet-verifier', 'reviewer-gate-unavailable')]), false],
      ['a coordinator second row is refused', ok([primary, second('coordinator')]), false],
      ['the second row alone is not a primary verdict', led(ledger(second('sonnet-verifier'))).ok, false],
      ['without the flag the second row is ignored: no pod verdict, refused', pod([]).ok, false],
    ];
  })(),
  ['--owed lists a PR with no later pod verdict', owedList(ledger(row(5, H, 'REGATE-OWED', 'land', 'reviewer-gate-unavailable')), () => false).map((r) => r.pr).join(), '5'],
  ['--owed omits a re-gated PR', owedList(ledger(row(5, H, 'REGATE-OWED', 'land', 'reviewer-gate-unavailable')), () => true).length, 0],
  ['--owed lists each PR and head once', owedList(ledger(row(5, H, 'REGATE-OWED', 'land'), row(5, H, 'REGATE-OWED', 'land')), () => false).length, 1],
  ['--owed passes the row\'s time to the re-gate test', owedList(ledger(row(5, H, 'REGATE-OWED', 'land')), (p, h, after) => after === Date.parse('2026-10-29T10:00:00Z')).length, 0],
  // remotes
  ['remote: https matches', remoteFor(rem(['jm', `https://github.com/${R}`]), R).remote, 'jm'],
  ['remote: https with .git matches', remoteFor(rem(['jm', `https://github.com/${R}.git`]), R).remote, 'jm'],
  ['remote: ssh with .git matches', remoteFor(rem(['jm', `git@github.com:${R}.git`]), R).remote, 'jm'],
  ['remote: ssh without .git matches', remoteFor(rem(['jm', `git@github.com:${R}`]), R).remote, 'jm'],
  ['remote: the other repo is not matched, the right one is', remoteFor(rem(['origin', 'git@github.com:Team-Commonly/commonly.git'], ['jm', `https://github.com/${R}.git`]), R).remote, 'jm'],
  ['remote: a repo with the same prefix is not matched', remoteFor(rem(['x', `https://github.com/${R}-old.git`]), R).error, `no git remote matches ${R}`],
  ['remote: no match refused', remoteFor(rem(['origin', 'git@github.com:Team-Commonly/commonly.git']), R).error, `no git remote matches ${R}`],
  ['remote: two matches refused', Boolean(remoteFor(rem(['a', `https://github.com/${R}`], ['b', `git@github.com:${R}.git`]), R).error), true],
  // local diff, main tip, api
  ['name-status: a rename lists both names', JSON.stringify(filesFromNameStatus('R100\tschema/a.json\tdocs/a.json\nM\tx.md')),
    JSON.stringify([{ filename: 'docs/a.json', previous_filename: 'schema/a.json' }, { filename: 'x.md' }])],
  ['name-status: add and delete list the file', filesFromNameStatus('A\ta\nD\tb').map((f) => f.filename).join(), 'a,b'],
  ['main unchanged is accepted', checkMain('1'.repeat(40), '1'.repeat(40)).ok, true],
  ['main moved is refused', checkMain('1'.repeat(40), '2'.repeat(40)).ok, false],
  ['an http API is refused', checkApi('http://example.test').ok, false],
  ['an https API is accepted', checkApi('https://example.test').ok, true],
];

// Cleanup never turns a pass into a refusal: a read-only tree (Go's module cache) is removed, and a failure only warns.
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'check-land-'));
fs.mkdirSync(path.join(tmp, 'cache', 'mod'), { recursive: true });
fs.writeFileSync(path.join(tmp, 'cache', 'mod', 'f.go'), 'x');
fs.chmodSync(path.join(tmp, 'cache', 'mod', 'f.go'), 0o444);
fs.chmodSync(path.join(tmp, 'cache', 'mod'), 0o555);
fs.chmodSync(path.join(tmp, 'cache'), 0o555);
const removed = removeTree(tmp, { warn: () => {} });
let warned = '';
const failed = removeTree('/nonexistent/dir', { rm: () => { throw new Error('boom'); }, warn: (m) => { warned = m; } });
cases.push(
  ['removeTree removes a read-only tree', removed && !fs.existsSync(tmp), true],
  ['a failing cleanup returns false, warns naming the dir, and does not throw', !failed && warned.includes('/nonexistent/dir'), true],
);

// Every gh invocation names the repo (a worktree can carry remotes for several repos).
const src = fs.readFileSync(new URL('./land.mjs', import.meta.url), 'utf8').split('\n');
const ghCalls = src.map((l, i) => [l, i + 1]).filter(([l]) => /\bgh\(/.test(l) && !l.includes('const gh ='));
cases.push(
  ['every gh call in land.mjs carries --repo (or a literal repos/${REPO} api path)',
    ghCalls.filter(([l]) => !(l.includes("'--repo'") || (l.includes("gh('api'") && l.includes('repos/${REPO}')))).map(([, n]) => n).join(), ''],
  ['land.mjs has gh calls to scan', ghCalls.length >= 3, true],
  ['land.mjs has no stack or FIXED-FWD code', /--stack|FIXED-FWD|chainOf/.test(src.join('\n')), false],
);

const bad = cases.filter(([, got, want]) => got !== want).map(([n]) => n);
if (bad.length) { console.error(`check-land: ${bad.length} of ${cases.length} wrong\n  ${bad.join('\n  ')}`); process.exit(1); }
console.log(`check-land: ${cases.length} cases ok`);
