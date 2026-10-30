#!/usr/bin/env bun
// Tests the pure decisions behind scripts/land.mjs (scripts/land-decide.mjs). No network, no gh. Own step in `make test`.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import {
  acquireLock, appendLedger, checkApi, checkCheckout, checkHead, checkLedger, checkMain, checkNoBunConfig, checkPodVerdict, checkRuns, checkTrust, cursorOf,
  filesFromNameStatus, landAction, mergeResult, parseLedger, parseLogins, quoteOutput, readToken, remoteFor, removeTree, scrub, takePage,
} from './land-decide.mjs';

const H = 'a'.repeat(40);
const H2 = 'c'.repeat(40);
const HEADER = 'pr\thead\tverdict\tby\twhen\tnote';
const ledger = (...rows) => parseLedger([HEADER, ...rows.map((r) => r.join('\t'))].join('\n'));
const row = (pr, head, verdict, by, note = '') => [pr, head, verdict, by, '2026-10-29T10:00:00Z', note];
const throws = (f) => { try { f(); return false; } catch { return true; } };
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
  ['no verdict shopping: a FAIL is not superseded by a later PASS from the same verifier', led(ledger(row(5, H, 'FAIL', 'opus-verifier'), row(5, H, 'PASS', 'opus-verifier'))).ok, false],
  ['no verdict shopping: a FAIL is not superseded by another verifier\'s PASS', led(ledger(row(5, H, 'FAIL', 'opus-verifier'), row(5, H, 'PASS', 'sonnet-verifier'))).ok, false],
  // pod verdict
  ['a listed reviewer PASS naming the PR and full head passes', pod([msg('ada', verdict(5, H))]).ok, true],
  ['a non-reviewer quoting "Written by Ada" is refused', pod([msg('mallory', `Written by Ada, a Commonly agent\n${verdict(5, H)}`)]).ok, false],
  ['a verdict naming another PR is refused', pod([msg('ada', verdict(50, H))]).ok, false],
  ['a #11 PASS citing #10 cannot override the reviewer\'s earlier #10 FAIL',
    pod([msg('ada', verdict(10, H, 'FAIL')), msg('ada', `jellymoon#11 ${H2}\nbase jellymoon#10 at ${H}\nGATE: PASS`)], '10', H).ok, false],
  ['a reviewer\'s later FAIL at the head refuses', pod([msg('ada', verdict(5, H)), msg('ada', verdict(5, H, 'FAIL'))]).ok, false],
  ['another reviewer\'s FAIL refuses beside a PASS', pod([msg('ada', verdict(5, H)), msg('ben', verdict(5, H, 'FAIL'))]).ok, false],
  ['a PASS followed by prose is refused', pod([msg('ada', `${verdict(5, H)}\nscratch that`)]).ok, false],
  ['on equal timestamps the newer message (lower index, newest first) decides: FAIL over an older PASS', pod([msg('ada', verdict(5, H, 'FAIL'), { createdAt: '2026-10-29T12:00:00Z' }), msg('ada', verdict(5, H), { createdAt: '2026-10-29T12:00:00Z' })]).ok, false],
  ['on equal timestamps the newer message decides: PASS over an older FAIL', pod([msg('ada', verdict(5, H), { createdAt: '2026-10-29T12:00:00Z' }), msg('ada', verdict(5, H, 'FAIL'), { createdAt: '2026-10-29T12:00:00Z' })]).ok, true],
  // idempotency
  ['merged with a LANDED row at the head is already landed', landAction({ number: 5, state: 'MERGED', headRefOid: H }, ledger(row(5, H, 'LANDED', 'land', 'abc'))), 'already-landed'],
  ['a LANDED row at another head does not count', landAction({ number: 5, state: 'MERGED', headRefOid: H }, ledger(row(5, H2, 'LANDED', 'land', 'abc'))), 'record-only'],
  ['a MERGED-UNVERIFIED row at the head is idempotent', landAction({ number: 5, state: 'MERGED', headRefOid: H }, ledger(row(5, H, 'MERGED-UNVERIFIED', 'land', 'x'))), 'already-landed'],
  ['MERGED-UNVERIFIED is a known verdict, not an endorsement (no PASS)', led(ledger(row(5, H, 'MERGED-UNVERIFIED', 'land', 'x'))).reason, `no verifier PASS for #5 at ${H.slice(0, 8)}`],
  ['api: the Commonly origin is accepted', checkApi('https://api.commonly.me').ok, true],
  ['api: another host is refused, so a file cannot choose where the bearer goes', checkApi('https://example.test').ok, false],
  ['api: another https host is accepted only when JM_ALLOW_API_ORIGIN equals it', checkApi('https://example.test', 'https://example.test').ok, true],
  ['bun config: a .env.local in the checkout is refused', checkNoBunConfig(['Makefile', '.env.local']).ok, false],
  ['bun config: a bunfig.toml is refused', checkNoBunConfig(['bunfig.toml']).ok, false],
  // trust gate: GitHub identity, never commit authors
  ...(() => {
    const trusted = parseLogins('Lily, ada ,');
    const pr = { isCrossRepository: false, author: 'lily' };
    const t = (o = {}) => checkTrust({ pr, pushers: ['lily', 'ada'], trusted, ...o });
    return [
      ['trust: same repo, trusted author, only trusted pushers', t().ok, true],
      ['trust: a missing isCrossRepository field is untrusted', t({ pr: { author: 'lily' } }).ok, false],
      ['trust: an empty push-access list refuses (the query went wrong)', t({ pushers: [] }).ok, false],
      ['trust: a cross-repository PR is refused', t({ pr: { ...pr, isCrossRepository: true } }).ok, false],
      ['trust: an untrusted PR author is refused', t({ pr: { ...pr, author: 'mallory' } }).ok, false],
      ['trust: an untrusted push-capable collaborator is refused and named', t({ pushers: ['lily', 'eve'] }).reason.includes("'eve'"), true],
      ['trust: an unset list refuses', checkTrust({ pr, pushers: ['lily'], trusted: parseLogins('') }).reason, 'JM_TRUSTED_LOGINS is not set; refusing to run any PR code'],
      ['trust: the refusal says the PR is not executed here', t({ pr: { ...pr, isCrossRepository: true } }).reason.startsWith('untrusted PR: not executed on this host, and nowhere else today (CI is off). Re-author it through the loop to land it.'), true],
    ];
  })(),
  // CI is a second witness only when it exists
  ['checks: none at all is fine', checkRuns([]).ok, true],
  ['checks: a failure refuses and names the run', checkRuns([{ name: 'a', conclusion: 'success' }, { name: 'lint', conclusion: 'failure' }]).reason.includes('lint'), true],
  // the land checkout
  ...(() => {
    const M = 'b'.repeat(40);
    const c = (o = {}) => checkCheckout({ head: M, mainTip: M, porcelain: '', remoteV: `jm\thttps://github.com/${R}.git (fetch)`, config: '', ...o });
    return [
      ['checkout: HEAD at main and clean is accepted', c().ok, true],
      ['checkout: HEAD not at main is refused', c({ head: H }).ok, false],
      ['checkout: HEAD elsewhere is fine only under --bootstrap', c({ head: H, bootstrap: true }).ok, true],
      ['checkout: a dirty tree under --bootstrap is refused', c({ head: H, bootstrap: true, porcelain: ' M Makefile' }).ok, false],
      ['preflight-only: a non-main HEAD is refused (the PR tree must not run its own Makefile)', c({ head: H }).ok, false],
      ['checkout: a dirty tree is refused', c({ porcelain: ' M scripts/land.mjs' }).ok, false],
      ['checkout: a remote URL with userinfo is refused', c({ remoteV: `jm\thttps://tok@github.com/${R}.git (fetch)` }).ok, false],
      ['checkout: an http extraheader is refused', c({ config: 'http.https://github.com/.extraheader AUTHORIZATION: basic x' }).ok, false],
    ];
  })(),
  // the real merge result decides what is protected, even where the merge base lies (a criss-cross history)
  ...(() => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'check-crisscross-'));
    const sh = (script) => execFileSync('bash', ['-ec', script], { cwd: dir, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
    sh(`export GIT_CONFIG_GLOBAL=/dev/null GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@l GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@l
      n=0; git() { n=$((n+1)); GIT_COMMITTER_DATE="2026-01-01T00:00:0$n" GIT_AUTHOR_DATE="2026-01-01T00:00:0$n" command git "$@"; } # fixed dates (P1 the newest, so it is the merge base git picks): ties are otherwise a coin toss
      c() { git commit -qam "$1"; }; set_f() { echo "$1" > scripts/f; c "f=$1"; }
      git init -q -b main; mkdir scripts; echo X > scripts/f; git add -A; c base
      git checkout -qb pr; echo 1 > g; git add -A; n=49; c P1; n=1; P1=$(git rev-parse HEAD)
      git checkout -q main; set_f Y; git checkout -q pr; git merge -q --no-edit main
      git checkout -q main; git merge -q --no-edit $P1; git checkout -q pr; set_f X`);
    const [tip, head] = [sh('git rev-parse main'), sh('git rev-parse pr')];
    const old = filesFromNameStatus(sh(`git diff --name-status -M $(git merge-base ${tip} ${head}) ${head}`)).map((f) => f.filename).join();
    const now = mergeResult((a) => execFileSync('git', a, { cwd: dir, encoding: 'utf8' }).trim(), tip, head).files.map((f) => f.filename).join();
    fs.rmSync(dir, { recursive: true, force: true });
    return [['criss-cross: the merge-base diff sees nothing of scripts/ (the old method)', old, ''], ['criss-cross: the merge result shows scripts/f changing', now, 'scripts/f']];
  })(),
  // the ledger is never read loosely
  ['ledger: a space-separated FAIL row makes the ledger unreadable', throws(() => parseLedger(`${HEADER}\n5 ${H} FAIL opus-verifier now x`)), true],
  ['ledger: a short-sha FAIL row makes the ledger unreadable', throws(() => parseLedger(`${HEADER}\n${['5', 'abc1234', 'FAIL', 'opus-verifier', 'now', 'x'].join('\t')}`)), true],
  ['ledger: blank and # lines are skipped', parseLedger(`# note\n\n${row(5, H, 'PASS', 'opus-verifier').join('\t')}`).length, 1],
  // PR output is quoted
  ['output: bidi overrides and line separators are stripped', quoteOutput('a\u202eb\u2066c\u2028d\u2029e'), 'preflight| abcde'],
  ['output: control characters are stripped, newline and tab kept', quoteOutput('a\u001b[2Jb\r\nc\td\u0007\n'), 'preflight| a[2Jb\npreflight| c\td'],
  // remotes
  ['remote: a URL with userinfo is not matched', remoteFor(rem(['jm', `https://tok@github.com/${R}`]), R).error, `no git remote matches ${R}`],
  ['remote: two matches refused', Boolean(remoteFor(rem(['a', `https://github.com/${R}`], ['b', `git@github.com:${R}.git`]), R).error), true],
  // main tip
  ['main unchanged is accepted', checkMain('1'.repeat(40), '1'.repeat(40)).ok, true],
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
  ['land.mjs: the preflight child has no stdin, its own session, quoted output and a group kill', /detached: true/.test(landSrc) && /stdio: \['ignore', 'pipe', 'pipe'\]/.test(landSrc) && /process\.kill\(-child\.pid/.test(landSrc) && /quoteOutput\(/.test(landSrc), true],
  ['land.mjs: the trust gate runs before anything is fetched', landSrc.indexOf('trustGate(pr);\n  const mainTip') > 0, true],
  ['land.mjs: verify-preflight is held to the same checkout rule as land', landSrc.includes('checkoutGate(mainTip, bootstrap)'), true],
  ['land.mjs: the preflight child has a wall-clock limit', /LIMIT_MS/.test(landSrc) && /setTimeout/.test(landSrc), true],
  ['land.mjs: the trust gate runs again before the merge', landSrc.indexOf('trustGate(fresh)') < landSrc.indexOf("gh('pr', 'merge'") && landSrc.includes('trustGate(fresh)'), true],
  ['land.mjs: the base is re-read right before the merge', landSrc.indexOf('checkHead(viewPr(n), head)') < landSrc.indexOf("gh('pr', 'merge'"), true],
);

const bad = cases.filter(([, got, want]) => got !== want).map(([n]) => n);
if (bad.length) { console.error(`check-land: ${bad.length} of ${cases.length} wrong\n  ${bad.join('\n  ')}`); process.exit(1); }
console.log(`check-land: ${cases.length} cases ok`);
