#!/usr/bin/env bun
// Tests the pure decisions behind scripts/land.mjs (scripts/land-decide.mjs). No network, no gh. Own step in `make test`.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {
  acquireLock, appendLedger, checkApi, checkCheckout, checkHead, checkLedger, checkMain, checkPodVerdict, checkRuns, checkTrust, cursorOf,
  filesFromNameStatus, landAction, parseLedger, parseLogins, quoteOutput, readToken, remoteFor, removeTree, scrub, takePage,
} from './land-decide.mjs';

const H = 'a'.repeat(40);
const H2 = 'c'.repeat(40);
const HEADER = 'pr\thead\tverdict\tby\twhen\tnote';
const ledger = (...rows) => parseLedger([HEADER, ...rows.map((r) => r.join('\t'))].join('\n'));
const row = (pr, head, verdict, by, note = '') => [pr, head, verdict, by, '2026-10-29T10:00:00Z', note];
const entries = [{ name: 'Ada', username: 'ada' }, { name: 'Ben', username: 'ben' }];
let t = 0;
const msg = (username, content, o = {}) => ({ userId: { username }, content, createdAt: `2026-10-29T10:${String(++t).padStart(2, '0')}:00Z`, ...o });
const verdict = (pr, head, gate = 'PASS') => `jellymoon#${pr} ${head}\nlooks right\nGATE: ${gate}`;
const pod = (msgs, pr = '5', head = H, o = {}) => checkPodVerdict(msgs, { pr, head, entries, ...o });
const led = (rows, head = H) => checkLedger(rows, { pr: '5', head });
const open = (o = {}) => ({ number: 5, state: 'OPEN', baseRefName: 'main', headRefOid: H, ...o });
const R = 'Team-Commonly/jellymoon';
const rem = (...l) => l.map(([n, u]) => `${n}\t${u} (fetch)\n${n}\t${u} (push)`).join('\n');

const cases = [
  // GitHub vs local head, base
  ['head mismatch refused', checkHead(open(), 'b'.repeat(40)).ok, false],
  ['a PR not based on main is refused', checkHead(open({ baseRefName: 'p05' }), H).ok, false],
  // ledger
  ['verifier PASS at the head accepted', led(ledger(row(5, H, 'PASS', 'opus-verifier'))).ok, true],
  ['stale-head PASS refused', led(ledger(row(5, H2, 'PASS', 'opus-verifier'))).ok, false],
  ['a later FAIL at the head refuses', led(ledger(row(5, H, 'PASS', 'opus-verifier'), row(5, H, 'FAIL', 'opus-verifier'))).ok, false],
  ['no verdict shopping: a FAIL is not superseded by a later PASS from the same verifier', led(ledger(row(5, H, 'FAIL', 'opus-verifier'), row(5, H, 'PASS', 'opus-verifier'))).ok, false],
  ['no verdict shopping: a FAIL is not superseded by another verifier\'s PASS', led(ledger(row(5, H, 'FAIL', 'opus-verifier'), row(5, H, 'PASS', 'sonnet-verifier'))).ok, false],
  ['a FAIL at the head blocks whatever its note', led(ledger(row(5, H, 'PASS', 'opus-verifier'), row(5, H, 'FAIL', 'sonnet-verifier', 'anything'))).ok, false],
  ['a FAIL at another head does not block this head', led(ledger(row(5, H2, 'FAIL', 'opus-verifier'), row(5, H, 'PASS', 'opus-verifier'))).ok, true],
  ['the latest verdict must be a PASS', led(ledger(row(5, H, 'PASS', 'opus-verifier'), row(5, H, 'PASS', 'sonnet-verifier'))).primary.by, 'sonnet-verifier'],
  ['coordinator PASS refused', led(ledger(row(5, H, 'PASS', 'coordinator'))).ok, false],
  ['a headerless ledger keeps its first row', parseLedger(row(5, H, 'PASS', 'opus-verifier').join('\t')).length, 1],
  // pod verdict
  ['a listed reviewer PASS naming the PR and full head passes', pod([msg('ada', verdict(5, H))]).ok, true],
  ['a non-reviewer quoting "Written by Ada" is refused', pod([msg('mallory', `Written by Ada, a Commonly agent\n${verdict(5, H)}`)]).ok, false],
  ['a verdict naming another PR is refused', pod([msg('ada', verdict(50, H))]).ok, false],
  ['a verdict at another head is refused', pod([msg('ada', verdict(5, H2))]).ok, false],
  ['a #11 PASS citing #10 cannot override the reviewer\'s earlier #10 FAIL',
    pod([msg('ada', verdict(10, H, 'FAIL')), msg('ada', `jellymoon#11 ${H2}\nbase jellymoon#10 at ${H}\nGATE: PASS`)], '10', H).ok, false],
  ['a hidden-comment first line does not count', pod([msg('ada', `<!-- jellymoon#5 ${H} -->\nGATE: PASS`)]).ok, false],
  ['a reviewer\'s later FAIL at the head refuses', pod([msg('ada', verdict(5, H)), msg('ada', verdict(5, H, 'FAIL'))]).ok, false],
  ['another reviewer\'s FAIL refuses beside a PASS', pod([msg('ada', verdict(5, H)), msg('ben', verdict(5, H, 'FAIL'))]).ok, false],
  ['a PASS followed by prose is refused', pod([msg('ada', `${verdict(5, H)}\nscratch that`)]).ok, false],
  ['a message older than `after` does not count', pod([msg('ada', verdict(5, H), { createdAt: '2026-10-29T09:00:00Z' })], '5', H, { after: Date.parse('2026-10-29T10:00:00Z') }).ok, false],
  ['on equal timestamps the newer message (lower index, newest first) decides: FAIL over an older PASS', pod([msg('ada', verdict(5, H, 'FAIL'), { createdAt: '2026-10-29T12:00:00Z' }), msg('ada', verdict(5, H), { createdAt: '2026-10-29T12:00:00Z' })]).ok, false],
  ['on equal timestamps the newer message decides: PASS over an older FAIL', pod([msg('ada', verdict(5, H), { createdAt: '2026-10-29T12:00:00Z' }), msg('ada', verdict(5, H, 'FAIL'), { createdAt: '2026-10-29T12:00:00Z' })]).ok, true],
  ['a message after `after` counts', pod([msg('ada', verdict(5, H), { createdAt: '2026-10-29T11:00:00Z' })], '5', H, { after: Date.parse('2026-10-29T10:00:00Z') }).ok, true],
  // idempotency
  ['merged with a LANDED row at the head is already landed', landAction({ number: 5, state: 'MERGED', headRefOid: H }, ledger(row(5, H, 'LANDED', 'land', 'abc'))), 'already-landed'],
  ['a LANDED row at another head does not count', landAction({ number: 5, state: 'MERGED', headRefOid: H }, ledger(row(5, H2, 'LANDED', 'land', 'abc'))), 'record-only'],
  ['merged without a LANDED row is recorded', landAction({ number: 5, state: 'MERGED', headRefOid: H }, ledger()), 'record-only'],
  ['open is checked', landAction(open(), ledger()), 'check'],
  ['a MERGED-UNVERIFIED row at the head is idempotent', landAction({ number: 5, state: 'MERGED', headRefOid: H }, ledger(row(5, H, 'MERGED-UNVERIFIED', 'land', 'x'))), 'already-landed'],
  ['MERGED-UNVERIFIED is a known verdict, not an endorsement (no PASS)', led(ledger(row(5, H, 'MERGED-UNVERIFIED', 'land', 'x'))).reason, `no verifier PASS for #5 at ${H.slice(0, 8)}`],
  // trust gate: GitHub identity, never commit authors
  ...(() => {
    const trusted = parseLogins('Lily, ada ,');
    const pr = { isCrossRepository: false, author: 'lily' };
    const t = (o = {}) => checkTrust({ pr, pushers: ['lily', 'ada'], trusted, ...o });
    return [
      ['trust: same repo, trusted author, only trusted pushers', t().ok, true],
      ['trust: logins compare case-insensitively', t({ pr: { ...pr, author: 'LILY' }, pushers: ['Ada'] }).ok, true],
      ['trust: a cross-repository PR is refused', t({ pr: { ...pr, isCrossRepository: true } }).ok, false],
      ['trust: an untrusted PR author is refused', t({ pr: { ...pr, author: 'mallory' } }).ok, false],
      ['trust: a missing author is refused', t({ pr: { ...pr, author: undefined } }).ok, false],
      ['trust: an untrusted push-capable collaborator is refused and named', t({ pushers: ['lily', 'eve'] }).reason.includes("'eve'"), true],
      ['trust: an unset list refuses', checkTrust({ pr, pushers: ['lily'], trusted: parseLogins('') }).reason, 'JM_TRUSTED_LOGINS is not set; refusing to run any PR code'],
      ['trust: a list of only commas is unset', parseLogins(' , ,').length, 0],
      ['trust: the refusal says the PR is not executed here', t({ pr: { ...pr, isCrossRepository: true } }).reason.startsWith('untrusted PR: not executed on this host; CI runs it. Re-author it through the loop to land it.'), true],
    ];
  })(),
  // CI is a second witness only when it exists
  ['checks: none at all is fine', checkRuns([]).ok, true],
  ['checks: all success is fine', checkRuns([{ name: 'a', conclusion: 'success' }]).ok, true],
  ['checks: a failure refuses and names the run', checkRuns([{ name: 'a', conclusion: 'success' }, { name: 'lint', conclusion: 'failure' }]).reason.includes('lint'), true],
  ['checks: an unfinished run is not a failure', checkRuns([{ name: 'a', conclusion: null }]).ok, true],
  ['checks: more runs than were read refuses', checkRuns([{ name: 'a', conclusion: 'success' }], 101).ok, false],
  // the land checkout
  ...(() => {
    const M = 'b'.repeat(40);
    const c = (o = {}) => checkCheckout({ head: M, mainTip: M, porcelain: '', remoteV: `jm\thttps://github.com/${R}.git (fetch)`, config: '', ...o });
    return [
      ['checkout: HEAD at main and clean is accepted', c().ok, true],
      ['checkout: HEAD not at main is refused', c({ head: H }).ok, false],
      ['checkout: HEAD elsewhere is fine when main is not required', c({ head: H, needMain: false }).ok, true],
      ['checkout: a dirty tree is refused', c({ porcelain: ' M scripts/land.mjs' }).ok, false],
      ['checkout: an untracked file is refused', c({ porcelain: '?? x' }).ok, false],
      ['checkout: a remote URL with userinfo is refused', c({ remoteV: `jm\thttps://tok@github.com/${R}.git (fetch)` }).ok, false],
      ['checkout: a remote URL with user:password is refused', c({ remoteV: `jm\thttps://u:p@github.com/${R}.git (push)` }).ok, false],
      ['checkout: an ssh remote is accepted', c({ remoteV: `jm\t${'git@github.com:'}${R}.git (fetch)\njm\tssh://git@github.com/${R} (push)` }).ok, true],
      ['checkout: an http extraheader is refused', c({ config: 'http.https://github.com/.extraheader AUTHORIZATION: basic x' }).ok, false],
    ];
  })(),
  // PR output is quoted
  ['output: control characters are stripped, newline and tab kept', quoteOutput('a\u001b[2Jb\r\nc\td\u0007\n'), 'preflight| a[2Jb\npreflight| c\td'],
  ['output: a fake land line is marked', quoteOutput('dry run: would land #5'), 'preflight| dry run: would land #5'],
  // remotes
  ['remote: https matches', remoteFor(rem(['jm', `https://github.com/${R}`]), R).remote, 'jm'],
  ['remote: a URL with userinfo is not matched', remoteFor(rem(['jm', `https://tok@github.com/${R}`]), R).error, `no git remote matches ${R}`],
  ['remote: the other repo is not matched, the right one is', remoteFor(rem(['origin', 'git@github.com:Team-Commonly/commonly.git'], ['jm', `https://github.com/${R}.git`]), R).remote, 'jm'],
  ['remote: two matches refused', Boolean(remoteFor(rem(['a', `https://github.com/${R}`], ['b', `git@github.com:${R}.git`]), R).error), true],
  // local diff, main tip, api
  ['name-status: a rename lists both names', JSON.stringify(filesFromNameStatus('R100\tschema/a.json\tdocs/a.json\nM\tx.md')),
    JSON.stringify([{ filename: 'docs/a.json', previous_filename: 'schema/a.json' }, { filename: 'x.md' }])],
  ['main unchanged is accepted', checkMain('1'.repeat(40), '1'.repeat(40)).ok, true],
  ['main moved is refused', checkMain('1'.repeat(40), '2'.repeat(40)).ok, false],
  ['an https API is accepted', checkApi('https://example.test').ok, true],
];

// Cleanup never turns a pass into a refusal: a read-only tree (Go's module cache) is removed, and a failure only warns.
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'check-land-'));
fs.mkdirSync(path.join(tmp, 'cache', 'mod'), { recursive: true });
fs.writeFileSync(path.join(tmp, 'cache', 'mod', 'f.go'), 'x');
fs.chmodSync(path.join(tmp, 'cache', 'mod', 'f.go'), 0o444);
fs.chmodSync(path.join(tmp, 'cache', 'mod'), 0o555);
fs.chmodSync(path.join(tmp, 'cache'), 0o555);
removeTree(tmp, { warn: () => {} });
let warned = '';
const failed = removeTree('/nonexistent/dir', { rm: () => { throw new Error('boom'); }, warn: (m) => { warned = m; } });
cases.push(
  ['removeTree removes a read-only tree; a failing cleanup warns naming the dir and does not throw', !fs.existsSync(tmp) && !failed && warned.includes('/nonexistent/dir'), true],
);

// Every gh invocation names the repo (a worktree can carry remotes for several repos).
const landSrc = fs.readFileSync(new URL('./land.mjs', import.meta.url), 'utf8');
const src = landSrc.split('\n');
const ghCalls = src.map((l, i) => [l, i + 1]).filter(([l]) => /\bgh\(/.test(l) && !l.includes('const gh ='));
cases.push(
  ['every gh call in land.mjs carries --repo (or a literal repos/${REPO} api path)',
    ghCalls.filter(([l]) => !(l.includes("'--repo'") || (l.includes("gh('api'") && l.includes('repos/${REPO}')))).map(([, n]) => n).join(), ''],
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
  ['token: land.mjs prints errors only through the scrubbing refuse', /main\(\)\.catch\(\(e\) => refuse\(/.test(landSrc), true],
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
  ['dry run: land.mjs has no ledger writer of its own (only appendLedger with dry)', /appendFileSync/.test(landSrc), false],
);
fs.rmSync(ldir, { recursive: true, force: true });

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

// land.mjs wiring (shell-outs are not unit-testable).
cases.push(
  ['land.mjs: the merge disables repo hooks', /'core\.hooksPath=\/dev\/null'/.test(landSrc) && /\.\.\.BASE, 'merge'/.test(src), true],
  ['land.mjs: refuses more than one PR number', /nums\.length !== 1/.test(landSrc), true],
  ['land.mjs: the preflight child has no stdin, its own session, quoted output and a group kill', /detached: true/.test(landSrc) && /stdio: \['ignore', 'pipe', 'pipe'\]/.test(landSrc) && /process\.kill\(-child\.pid/.test(landSrc) && /quoteOutput\(/.test(landSrc), true],
  ['land.mjs: the trust gate runs before anything is fetched', landSrc.indexOf('trustGate(pr);\n  const mainTip') > 0, true],
  ['land.mjs: the base is re-read right before the merge', landSrc.indexOf('checkHead(viewPr(n), head)') < landSrc.indexOf("gh('pr', 'merge'"), true],
  ['land.mjs: TMPDIR is inside the throwaway dir', /TMPDIR: path\.join\(dir, 'tmp'\)/.test(landSrc), true],
);

const bad = cases.filter(([, got, want]) => got !== want).map(([n]) => n);
if (bad.length) { console.error(`check-land: ${bad.length} of ${cases.length} wrong\n  ${bad.join('\n  ')}`); process.exit(1); }
console.log(`check-land: ${cases.length} cases ok`);
