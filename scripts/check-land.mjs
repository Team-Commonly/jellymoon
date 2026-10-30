#!/usr/bin/env bun
// Tests the pure decisions behind scripts/land.mjs (scripts/land-decide.mjs). No network, no gh. Own step in `make test`.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import {
  acquireLock, appendLedger, checkApi, checkHead, checkLedger, checkMain, checkPlatform, checkPodVerdict, checkSecondVerifier, cursorOf,
  filesFromNameStatus, landAction, owedList, parseLedger, proveSandbox, readToken, remoteFor, removeTree, sandboxProfile, scrub, takePage,
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
  ['a MERGED-UNVERIFIED row at the head is idempotent', landAction({ number: 5, state: 'MERGED', headRefOid: H }, ledger(row(5, H, 'MERGED-UNVERIFIED', 'land', 'x'))), 'already-landed'],
  ['MERGED-UNVERIFIED is a known verdict, not an endorsement (no PASS)', led(ledger(row(5, H, 'MERGED-UNVERIFIED', 'land', 'x'))).reason, `no verifier PASS for #5 at ${H.slice(0, 8)}`],
  // TEMPORARY reviewer-gate-unavailable rule (remove when the reviewers return)
  ...(() => {
    const primary = row(5, H, 'PASS', 'opus-verifier');
    const second = (by, note = 'reviewer-gate-unavailable: fresh context') => row(5, H, 'PASS', by, note);
    const ok = (rows) => { const r = ledger(...rows); const l = led(r); return l.ok && checkSecondVerifier(r, { pr: '5', head: H, primary: l.primary }).ok; };
    return [
      ['a second verifier on a different model passes', ok([primary, second('sonnet-verifier')]), true],
      ['a second verifier on the same model is refused', ok([primary, second('opus-verifier')]), false],
      ['a second row without the note is refused', ok([primary, second('sonnet-verifier', '')]), false],
      ['the reviewer note matches case-insensitively', ok([primary, second('sonnet-verifier', 'Reviewer-Gate-Unavailable: fresh')]), true],
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

// Pod paging: dedupe by id across pages, and fail closed where a boundary could hide a message.
const at1 = (id, ts) => ({ id, createdAt: ts });
const seen = new Set();
takePage(seen, [at1('a', '2026-10-29T10:00:00.005Z'), at1('b', '2026-10-29T10:00:00.002Z')], 200);
const page2 = takePage(seen, [at1('b', '2026-10-29T10:00:00.002Z'), at1('c', '2026-10-29T10:00:00.001Z')], 200);
cases.push(
  ['paging: a message seen on a previous page is dropped', page2.fresh.map((m) => m.id).join(), 'c'],
  ['paging: the next cursor is the oldest time + 1 ms, so same-millisecond messages return', cursorOf([at1('a', '2026-10-29T10:00:00.005Z'), at1('c', '2026-10-29T10:00:00.001Z')]), '2026-10-29T10:00:00.002Z'],
  ['paging: a message without a time makes the cursor unusable', cursorOf([at1('a', '2026-10-29T10:00:00.005Z'), { id: 'x' }]), null],
  ['paging: a full page with nothing new is stuck (fail closed)', takePage(seen, [at1('a', '2026-10-29T10:00:00.005Z'), at1('b', '2026-10-29T10:00:00.002Z')], 2).stuck, true],
  ['paging: a short page with nothing new is the end, not stuck', takePage(seen, [at1('a', '2026-10-29T10:00:00.005Z')], 200).stuck, false],
);

// Token hygiene: a malformed token file is never echoed, and scrub removes a token from any text.
const tdir = fs.mkdtempSync(path.join(os.tmpdir(), 'check-token-'));
const tokenOf = (body) => { const f = path.join(tdir, 'tok'); fs.writeFileSync(f, body); try { return { value: readToken(f) }; } catch (e) { return { error: e.message }; } };
const SECRET = 'jmu_FAKE-token-abc123';
const bads = [`${SECRET}\n# note`, `${SECRET} extra`, `${SECRET}\u0007`, `\n`].map(tokenOf);
cases.push(
  ['token: a one-line token file is read', tokenOf(`${SECRET}\n`).value, SECRET],
  ['token: every malformed file is refused', bads.every((b) => b.error), true],
  ['token: no refusal message contains the token', bads.some((b) => b.error?.includes('jmu_FAKE') || b.error?.includes('abc123')), false],
  ['token: scrub removes the token from a message', scrub(`Bearer ${SECRET} rejected`, SECRET).includes('abc123'), false],
  ['token: land.mjs prints errors only through the scrubbing refuse', /main\(\)\.catch\(\(e\) => refuse\(/.test(fs.readFileSync(new URL('./land.mjs', import.meta.url), 'utf8')), true],
);
fs.rmSync(tdir, { recursive: true, force: true });

// --dry-run writes no ledger row; land.mjs has no other ledger writer.
const ldir = fs.mkdtempSync(path.join(os.tmpdir(), 'check-ledger-'));
const lf = path.join(ldir, 'ledger.tsv');
const dryWrote = appendLedger(lf, ['5', H, 'LANDED', 'land', 'now', 'x'], { dry: true });
const dryTouched = fs.existsSync(lf);
appendLedger(lf, ['5', H, 'LANDED', 'land', 'now', 'x']);
cases.push(
  ['dry run: appendLedger reports no write and creates no file', !dryWrote && !dryTouched, true],
  ['dry run: the real write creates header and row', fs.readFileSync(lf, 'utf8').split('\n').filter(Boolean).length, 2],
  ['dry run: land.mjs has no ledger writer of its own (only appendLedger with dry)', /appendFileSync/.test(fs.readFileSync(new URL('./land.mjs', import.meta.url), 'utf8')), false],
);
fs.rmSync(ldir, { recursive: true, force: true });

// Sandbox: the preflight runs the PR's code. Any platform but macOS refuses; on macOS the profile is proved below.
cases.push(
  ['sandbox: linux is refused with the exact message', checkPlatform('linux').reason, 'preflight sandbox is macOS-only; add one for linux before landing here'],
  ['sandbox: darwin is accepted', checkPlatform('darwin').ok, true],
);
// The probe land runs before every preflight, under the exact profile. It must fail all four forbidden actions, and
// it must catch a profile that does not hold. It cannot nest inside the preflight sandbox, so it skips there.
const usable = process.platform === 'darwin' && spawnSync('sandbox-exec', ['-p', '(version 1)(allow default)', 'true']).status === 0;
if (usable) {
  const root = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'check-sandbox-')));
  const [home, work, ledgerDir, tokenDir] = ['home', 'work', 'ledger', 'token'].map((d) => path.join(root, d));
  [path.join(home, '.ssh'), work, ledgerDir, tokenDir].forEach((d) => fs.mkdirSync(d, { recursive: true }));
  fs.writeFileSync(path.join(home, '.ssh', 'id'), 'FAKE-SECRET');
  fs.writeFileSync(path.join(ledgerDir, 'l.tsv'), 'x');
  fs.writeFileSync(path.join(tokenDir, 't'), 'x');
  const hold = { homes: [home], allowRead: [], allowWrite: [work], hide: [path.join(ledgerDir, 'l.tsv'), path.join(tokenDir, 't')] };
  const prof = sandboxProfile(hold);
  const run = (cmd) => spawnSync('sandbox-exec', ['-p', prof, 'sh', '-c', cmd], { encoding: 'utf8' }).status;
  const outside = path.join(root, 'outside'); // the work dir's parent: not on the allow-list
  const proven = await proveSandbox({ profile: prof, dir: work, home });
  const weak = await proveSandbox({ profile: sandboxProfile({ ...hold, homes: [path.join(root, 'elsewhere')] }), dir: work, home });
  const open2 = await proveSandbox({ profile: '(version 1)(allow default)', dir: work, home });
  cases.push(
    ['sandbox proof: the profile passes every forbidden-action probe', proven.ok, true],
    ['sandbox proof: a profile that leaves the home readable is caught', weak.reason, `sandbox profile is not holding: reading a file under ${home}`],
    ['sandbox proof: an open profile is caught', open2.ok, false],
    ['sandbox proof: reading a file under the home fails', run(`cat '${home}/.ssh/id' >/dev/null`) !== 0, true],
    ['sandbox proof: reading the ledger dir fails', run(`cat '${ledgerDir}/l.tsv' >/dev/null`) !== 0, true],
    ['sandbox proof: reading the token dir fails', run(`cat '${tokenDir}/t' >/dev/null`) !== 0, true],
    ['sandbox proof: writing outside the work dir fails and leaves no file', run(`echo x > '${outside}'`) !== 0 && !fs.existsSync(outside), true],
    ['sandbox proof: writing inside the work dir succeeds', run(`echo x > '${work}/ok'`), 0],
    ['sandbox proof: TMPDIR-style writes outside, even in /tmp, fail', run(`echo x > /tmp/jm-sandbox-probe-${process.pid}`) !== 0 && !fs.existsSync(`/tmp/jm-sandbox-probe-${process.pid}`), true],
  );
  fs.rmSync(root, { recursive: true, force: true });
} else console.log('check-land: SKIP sandbox proof (needs macOS with a usable sandbox-exec; one cannot nest inside the preflight sandbox itself)');

// The lock: per clone; a dead owner or a lock dir with no pid is stale and taken over by an atomic rename.
const lroot = fs.mkdtempSync(path.join(os.tmpdir(), 'check-lock-'));
const L = path.join(lroot, 'jm-land.lock');
const first = acquireLock(L, { pid: 111, alive: () => true });
const live = acquireLock(L, { pid: 222, alive: () => true });
const dead = acquireLock(L, { pid: 333, alive: () => false });
const deadPid = fs.readFileSync(path.join(L, 'pid'), 'utf8');
fs.rmSync(path.join(L, 'pid'));
const nopid = acquireLock(L, { pid: 444, alive: () => true });
const nopidPid = fs.readFileSync(path.join(L, 'pid'), 'utf8');
cases.push(
  ['lock: a free lock is taken', first.ok, true],
  ['lock: a live owner refuses', live.reason, 'another land is running (pid 111)'],
  ['lock: a dead owner is taken over and the pid is ours', dead.ok && deadPid === '333', true],
  ['lock: a lock dir with no pid file is stale, not an error', nopid.ok && nopidPid === '444', true],
  ['lock: no stale dir is left behind', fs.readdirSync(lroot).join(), 'jm-land.lock'],
);
fs.rmSync(lroot, { recursive: true, force: true });

// land.mjs wiring (shell-outs are not unit-testable): no repo hooks on the merge, one PR number, scripts/ equal to main's.
const landSrc = fs.readFileSync(new URL('./land.mjs', import.meta.url), 'utf8');
cases.push(
  ['land.mjs: the merge disables repo hooks', /'core\.hooksPath=\/dev\/null'/.test(landSrc) && /\.\.\.BASE, 'merge'/.test(src), true],
  ['land.mjs: refuses more than one PR number', /nums\.length !== 1/.test(landSrc), true],
  ['land.mjs: outside --bootstrap, scripts/ must equal main\'s', /scriptsAreMains\(mainTip\)/.test(landSrc), true],
  ['land.mjs: TMPDIR is inside the throwaway dir', /TMPDIR: path\.join\(dir, 'tmp'\)/.test(landSrc), true],
  ['land.mjs: the preflight runs only after the profile is proved', landSrc.indexOf('proveSandbox(') < landSrc.indexOf("'sandbox-exec', ['-p'"), true],
);

const bad = cases.filter(([, got, want]) => got !== want).map(([n]) => n);
if (bad.length) { console.error(`check-land: ${bad.length} of ${cases.length} wrong\n  ${bad.join('\n  ')}`); process.exit(1); }
console.log(`check-land: ${cases.length} cases ok`);
