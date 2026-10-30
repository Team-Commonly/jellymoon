#!/usr/bin/env bun
// Tests the pure decisions behind scripts/land.mjs (scripts/land-decide.mjs). No network, no gh. Own step in `make test`.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync, spawnSync } from 'node:child_process';
import {
  landedNote, mergeAction, mergeTree,
  acquireLock, appendLedger, checkApi, checkCheckout, checkHead, checkLedger, checkMain, checkNoBunConfig, checkPodVerdict, checkRuns, checkTrust, cursorOf,
  checkNode, entropyBase, pushersOf, trackThrowaway, landAction, parseLedger, parseLogins, quoteOutput, parseReviewers, readToken, remoteFor, removeTree, scrub, stripControls, takePage,
} from './land-decide.mjs';

const H = 'a'.repeat(40);
const H2 = 'c'.repeat(40);
const HEADER = 'pr\thead\tverdict\tby\twhen\tnote';
const ledger = (...rows) => parseLedger([HEADER, ...rows.map((r) => r.join('\t'))].join('\n'));
const row = (pr, head, verdict, by, note = '') => [pr, head, verdict, by, '2026-10-29T10:00:00Z', note];
const throws = (f) => { try { f(); return false; } catch { return true; } };
const entries = [{ id: 'id-orion', name: 'Orion' }, { id: 'id-vega', name: 'Vega' }]; // ids are the pod user's immutable id; usernames are labels
let t = 0;
const msg = (username, content, o = {}) => ({ userId: { _id: `id-${username}`, username }, content, createdAt: `2026-10-29T10:${String(++t).padStart(2, '0')}:00Z`, ...o });
const verdict = (pr, head, gate = 'PASS') => `jellymoon#${pr} ${head}\nlooks right\nGATE: ${gate}`;
const pod = (msgs, pr = '5', head = H, o = {}) => checkPodVerdict(msgs, { pr, head, entries, ...o });
const led = (rows, head = H) => checkLedger(rows, { pr: '5', head });
const open = (o = {}) => ({ number: 5, state: 'OPEN', baseRefName: 'main', headRefOid: H, ...o });
const R = 'Team-Commonly/jellymoon';
const rem = (...l) => l.map(([n, u]) => `${n}\t${u} (fetch)\n${n}\t${u} (push)`).join('\n');

const cases = [
  // GitHub vs local head, base
  ['head mismatch refused', checkHead(open(), 'b'.repeat(40)).ok, false],
  ['a CLOSED PR is refused (not only MERGED)', checkHead(open({ state: 'CLOSED' }), H).ok, false],
  ['a MERGED PR is refused', checkHead(open({ state: 'MERGED' }), H).ok, false],
  ['a PR not based on main is refused', checkHead(open({ baseRefName: 'p05' }), H).ok, false],
  // ledger
  ['verifier PASS at the head accepted', led(ledger(row(5, H, 'PASS', 'opus-verifier'))).ok, true],
  ['a PASS at another head does not count (a stale-head PASS never lands)', led(ledger(row(5, H2, 'PASS', 'opus-verifier'))).ok, false],
  ['a PASS by a lookalike (prefix or suffix on the verifier name) is refused', [`x-opus-verifier`, `opus-verifier-2`].map((by) => led(ledger(row(5, H, 'PASS', by))).ok).join(), 'false,false'],
  ['an unknown verdict at the head blocks even beside a PASS', led(ledger(row(5, H, 'PASS', 'opus-verifier'), row(5, H, 'MAYBE', 'opus-verifier'))).ok, false],
  ['a PASS for another PR does not count', led(ledger(row(6, H, 'PASS', 'opus-verifier'))).ok, false],
  ['a PASS for #5 is not read as #50 or #15', led(ledger(row(50, H, 'PASS', 'opus-verifier'), row(15, H, 'PASS', 'opus-verifier'))).ok, false],
  ['a FAIL at another head does not block this head', led(ledger(row(5, H2, 'FAIL', 'opus-verifier'), row(5, H, 'PASS', 'opus-verifier'))).ok, true],
  ['a later FAIL at the head refuses after a PASS', led(ledger(row(5, H, 'PASS', 'opus-verifier'), row(5, H, 'FAIL', 'sonnet-verifier'))).reason.includes('a FAIL blocks that head'), true],
  ['a FAIL row with a note still blocks (every real FAIL carries one)', led(ledger(row(5, H, 'FAIL', 'opus-verifier', 'found a real bug in X'))).reason.includes('a FAIL blocks that head'), true],
  ['the latest PASS is the primary one', led(ledger(row(5, H, 'PASS', 'opus-verifier'), row(5, H, 'PASS', 'sonnet-verifier'))).primary.by, 'sonnet-verifier'],
  ['a PASS by a non-verifier is refused', led(ledger(row(5, H, 'PASS', 'coordinator'))).ok, false],
  ['a coordinator PASS beside a verifier PASS still refuses', led(ledger(row(5, H, 'PASS', 'opus-verifier'), row(5, H, 'PASS', 'coordinator'))).ok, false],
  ['no verdict shopping: a FAIL is not superseded by a later PASS from the same verifier', led(ledger(row(5, H, 'FAIL', 'opus-verifier'), row(5, H, 'PASS', 'opus-verifier'))).ok, false],
  ['no verdict shopping: a FAIL is not superseded by another verifier\'s PASS', led(ledger(row(5, H, 'FAIL', 'opus-verifier'), row(5, H, 'PASS', 'sonnet-verifier'))).ok, false],
  // pod verdict
  ['a listed reviewer PASS naming the PR and full head passes', pod([msg('orion', verdict(5, H))]).ok, true],
  ['a non-reviewer quoting "Written by Orion" is refused', pod([msg('mallory', `Written by Orion, a Commonly agent\n${verdict(5, H)}`)]).ok, false],
  ['a verdict line that is not the first line is refused', pod([msg('orion', `hello\n${verdict(5, H)}`)]).ok, false],
  ['a verdict quoting the head mid-line is refused', pod([msg('orion', `see ${verdict(5, H)}`)]).ok, false],
  ['a hidden-comment first line does not count', pod([msg('orion', `<!-- jellymoon#5 ${H} -->\nGATE: PASS`)]).ok, false],
  ['a verdict at another head is refused', pod([msg('orion', verdict(5, H2))]).ok, false],
  ['a verdict naming another PR is refused', pod([msg('orion', verdict(50, H))]).ok, false],
  ['a #11 PASS citing #10 cannot override the reviewer\'s earlier #10 FAIL',
    pod([msg('orion', verdict(10, H, 'FAIL')), msg('orion', `jellymoon#11 ${H2}\nbase jellymoon#10 at ${H}\nGATE: PASS`)], '10', H).ok, false],
  ['a reviewer\'s later FAIL at the head refuses', pod([msg('orion', verdict(5, H)), msg('orion', verdict(5, H, 'FAIL'))]).ok, false],
  ['another reviewer\'s FAIL refuses beside a PASS', pod([msg('orion', verdict(5, H)), msg('vega', verdict(5, H, 'FAIL'))]).ok, false],
  ['a PASS followed by prose is refused', pod([msg('orion', `${verdict(5, H)}\nscratch that`)]).ok, false],
  ['on equal timestamps the newer message (lower index, newest first) decides: FAIL over an older PASS', pod([msg('orion', verdict(5, H, 'FAIL'), { createdAt: '2026-10-29T12:00:00Z' }), msg('orion', verdict(5, H), { createdAt: '2026-10-29T12:00:00Z' })]).ok, false],
  ['on equal timestamps the newer message decides: PASS over an older FAIL', pod([msg('orion', verdict(5, H), { createdAt: '2026-10-29T12:00:00Z' }), msg('orion', verdict(5, H, 'FAIL'), { createdAt: '2026-10-29T12:00:00Z' })]).ok, true],
  // idempotency
  ['an OPEN PR is to be checked', landAction({ number: 5, state: 'OPEN', headRefOid: H }, ledger()), 'check'],
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
    const trusted = parseLogins('Alice, orion ,');
    const pr = { isCrossRepository: false, author: 'alice' };
    const t = (o = {}) => checkTrust({ pr, pushers: ['alice', 'orion'], trusted, ...o });
    return [
      ['trust: same repo, trusted author, only trusted pushers', t().ok, true],
      ['trust: the PR author compares lowercased', t({ pr: { ...pr, author: 'ALICE' }, pushers: ['Orion'] }).ok, true],
      ['trust: a PR with no author is refused', t({ pr: { isCrossRepository: false } }).ok, false],
      ['trust: an empty-string pusher is a stranger', t({ pushers: ['alice', ''] }).ok, false],
      ['trust: a missing isCrossRepository field is untrusted', t({ pr: { author: 'alice' } }).ok, false],
      ['trust: an empty push-access list refuses (the query went wrong)', t({ pushers: [] }).ok, false],
      ['trust: a cross-repository PR is refused', t({ pr: { ...pr, isCrossRepository: true } }).ok, false],
      ['trust: an untrusted PR author is refused', t({ pr: { ...pr, author: 'mallory' } }).ok, false],
      ['trust: an untrusted push-capable collaborator is refused and named', t({ pushers: ['alice', 'eve'] }).reason.includes('an account that can push to the repo is not trusted: eve'), true],
      ['trust: the author message is its own', t({ pr: { ...pr, author: 'mallory' } }).reason.includes("author 'mallory' is not trusted"), true],
      ['trust: an unset list refuses', checkTrust({ pr, pushers: ['alice'], trusted: parseLogins('') }).reason, 'JM_TRUSTED_LOGINS is not set; refusing to run any PR code'],
      ['trust: the refusal says the PR is not executed here', t({ pr: { ...pr, isCrossRepository: true } }).reason.startsWith('untrusted PR: not executed on this host. Re-author it through the loop to land it.'), true],
    ];
  })(),
  // CI is a second witness only when it exists
  ['checks: none at all is fine', checkRuns([]).ok, true],
  ['checks: all success is fine', checkRuns([{ name: 'a', conclusion: 'success' }, { name: 'b', conclusion: 'success' }]).ok, true],
  ['checks: neutral, skipped and stale runs are not failures', checkRuns([{ name: 'a', conclusion: 'neutral' }, { name: 'b', conclusion: 'skipped' }, { name: 'c', conclusion: 'stale' }]).ok, true],
  ['checks: a run name in a refusal is stripped of escapes and bidi marks', checkRuns([{ name: 'x\u202e\u200fy', conclusion: 'failure' }]).reason.includes('xy (failure)'), true],
  ['checks: an unfinished run (conclusion null) is not a failure', checkRuns([{ name: 'a', conclusion: null }]).ok, true],
  ['checks: more runs than were read refuses', checkRuns([{ name: 'a', conclusion: 'success' }], 101).ok, false],
  ['checks: a failure refuses and names the run', checkRuns([{ name: 'a', conclusion: 'success' }, { name: 'lint', conclusion: 'failure' }]).reason.includes('lint'), true],
  // the land checkout
  ...(() => {
    const M = 'b'.repeat(40);
    const c = (o = {}) => checkCheckout({ head: M, mainTip: M, porcelain: '', remoteV: `jm\thttps://github.com/${R}.git (fetch)`, config: '', ...o });
    return [
      ['checkout: HEAD at main and clean is accepted', c().ok, true],
      ['checkout: HEAD not at main is refused', c({ head: H }).ok, false],
      ['checkout: HEAD elsewhere is fine only under --bootstrap', c({ head: H, bootstrap: true }).ok, true],
      ['checkout: --bootstrap at the PR head is accepted', c({ head: H, prHead: H.toUpperCase(), bootstrap: true }).ok, true],
      ['checkout: --bootstrap anywhere but the PR head is refused', c({ head: H, prHead: H2, bootstrap: true }).ok, false],
      ['checkout: a dirty tree under --bootstrap is refused', c({ head: H, bootstrap: true, porcelain: ' M Makefile' }).ok, false],
      ['preflight-only: a non-main HEAD is refused (the PR tree must not run its own Makefile)', c({ head: H }).ok, false],
      ['checkout: an assume-unchanged file (h) is refused', c({ lsv: 'H a\nh scripts/land.mjs' }).ok, false],
      ['checkout: a skip-worktree file (S) is refused', c({ lsv: 'S scripts/land.mjs' }).ok, false],
      ['checkout: a plain tracked listing is accepted', c({ lsv: 'H a\nH b' }).ok, true],
      ['checkout: a diff from HEAD is refused', c({ diffDirty: true }).ok, false],
      ['checkout: untracked files (porcelain with showUntrackedFiles=all) are refused', c({ porcelain: '?? scripts/new/x.mjs' }).ok, false],
      ['checkout: a dirty tree is refused', c({ porcelain: ' M scripts/land.mjs' }).ok, false],
      ['checkout: an ssh remote with user:password is refused', c({ remoteV: `jm\tssh://u:p@github.com/${R} (push)` }).ok, false],
      ['checkout: an ssh remote without a password is accepted', c({ remoteV: `jm\tssh://git@github.com/${R} (push)` }).ok, true],
      ['checkout: an scp-form remote (git@github.com:R.git) is accepted', c({ remoteV: `jm\t${'git@github.com:'}${R}.git (push)` }).ok, true],
      ['checkout: a remote URL with userinfo is refused', c({ remoteV: `jm\thttps://tok@github.com/${R}.git (fetch)` }).ok, false],
      ['checkout: an http extraheader is refused', c({ config: 'http.https://github.com/.extraheader AUTHORIZATION: basic x' }).ok, false],
    ];
  })(),
  // the ledger is never read loosely
  ['ledger: a space-separated FAIL row makes the ledger unreadable', throws(() => parseLedger(`${HEADER}\n5 ${H} FAIL opus-verifier now x`)), true],
  ['ledger: a row with 7 columns is unreadable', throws(() => parseLedger(`${HEADER}\n${[...row(5, H, 'PASS', 'opus-verifier'), 'extra'].join('\t')}`)), true],
  ['ledger: a short-sha FAIL row makes the ledger unreadable', throws(() => parseLedger(`${HEADER}\n${['5', 'abc1234', 'FAIL', 'opus-verifier', 'now', 'x'].join('\t')}`)), true],
  ...['010', '10.0', 'PR10'].map((v) => [`ledger: a PR column of '${v}' makes the ledger unreadable`, throws(() => parseLedger(row(v, H, 'PASS', 'opus-verifier').join('\t'))), true]),
  ['ledger: blank and # lines are skipped', parseLedger(`# note\n\n${row(5, H, 'PASS', 'opus-verifier').join('\t')}`).length, 1],
  // the entropy ratchet base and a branch name
  ['entropy base: the tip as fetched', entropyBase('abc', { bootstrap: false, mainHasEntropy: true }), 'abc'],
  ['entropy base: bootstrap once main HAS entropy.json gets the tip, so the ratchet applies', entropyBase('abc', { bootstrap: true, mainHasEntropy: true }), 'abc'],
  ['entropy base: none only for the bootstrap land of a main without entropy.json', entropyBase('abc', { bootstrap: true, mainHasEntropy: false }), 'none'],
  ['entropy base: a non-bootstrap land never gets none', entropyBase('abc', { bootstrap: false, mainHasEntropy: false }), 'abc'],
  ['output: U+200E, U+200F and U+061C are stripped from quoted preflight output', quoteOutput('a\u200eb\u200fc\u061cd'), 'preflight| abcd'],
  // the dry run
  ['merge tree: the first line of merge-tree is the tree', mergeTree(() => 'abc123\nrest', H, H2), 'abc123'],
  ['merge tree: a conflict is a refusal naming both commits', throws(() => mergeTree(() => { throw new Error('x'); }, H, H2)), true],
  ['dry run: stops before the merge', mergeAction({ dryRun: true }), 'stop'],
  ['a real land goes on to the merge', mergeAction({ dryRun: false }), 'merge'],
  ['landed note: a merge commit with the tested tree is just the sha', landedNote('abc', 't1', 't1'), 'abc'],
  ['landed note: a different tree is recorded as TREE-MISMATCH with both trees', landedNote('abc', 't1', 't2'), 'abc TREE-MISMATCH tested t1 merged t2'],
  ['output: a branch name with escapes is stripped', stripControls('x\u001b[31my'), 'x[31my'],
  ['pod: a forged username with a different id is rejected', pod([msg('orion', verdict(5, H), { userId: { _id: 'id-forged', username: 'orion' } })]).ok, false],
  ['pod: the right id with any username is accepted', pod([msg('orion', verdict(5, H), { userId: { _id: 'id-orion', username: 'alice-example' } })]).ok, true],
  ['pod: a forged later PASS (other id, same name) does not override the real FAIL', pod([msg('orion', verdict(5, H, 'FAIL')), msg('orion', verdict(5, H), { userId: { _id: 'id-forged', username: 'orion' } })]).ok, false],
  ['pod: the latest verdict is decided per id (a later PASS from the same id wins)', pod([msg('orion', verdict(5, H, 'FAIL'), { createdAt: '2026-10-29T10:00:00Z' }), msg('orion', verdict(5, H), { createdAt: '2026-10-29T11:00:00Z' })]).ok, true],
  ['pod: a prefix of a listed id is not the reviewer', pod([msg('orion', verdict(5, H), { userId: { _id: 'id-orio', username: 'orion' } })]).ok, false],
  ['pod: a case variant of a listed id is not the reviewer', pod([msg('orion', verdict(5, H), { userId: { _id: 'ID-ORION', username: 'orion' } })]).ok, false],
  ['pod: a string userId (not a populated user) is ignored', pod([msg('orion', verdict(5, H), { userId: 'id-orion' })]).ok, false],
  ['pod: no configured reviewers refuses and names JM_VERDICT_REVIEWERS', pod([msg('orion', verdict(5, H))], '5', H, { entries: [] }).reason.includes('JM_VERDICT_REVIEWERS'), true],
  ['reviewers: id:Name pairs parse; blanks and malformed entries are dropped', JSON.stringify(parseReviewers(' a1:Alice Example, ,bad,:x,b2:Vega ')), '[{"id":"a1","name":"Alice Example"},{"id":"b2","name":"Vega"}]'],
  ['reviewers: unset or empty gives none', parseReviewers(undefined).length + parseReviewers(' , ').length, 0],
  ['trust: a list of only blanks and commas is unset', parseLogins(' , ,').length, 0],
  ['trust: blanks inside a list are dropped, the rest kept', parseLogins('a, ,b').join(), 'a,b'],
  ['trust: an author with no login never passes, even if the list had a blank', checkTrust({ pr: { isCrossRepository: false, author: undefined }, pushers: ['a'], trusted: parseLogins('a, ,b') }).ok, false],
  ['trust: an empty author login never passes', checkTrust({ pr: { isCrossRepository: false, author: '' }, pushers: ['a'], trusted: parseLogins('a, ,b') }).ok, false],
  ['output: C1 controls (U+0080 to U+009F) are stripped', stripControls('a\u0080b\u009bc\u009fd'), 'abcd'],
  // check-entropy itself: none passes only when named; an unreadable named base fails, and a PR cannot name one (land sets the env)
  ...(() => {
    const entropy = (base) => spawnSync(process.execPath, [fileURLToPath(new URL('./check-entropy.mjs', import.meta.url))], { encoding: 'utf8', env: { PATH: process.env.PATH, HOME: process.env.HOME, JM_ENTROPY_BASE: base } }).status;
    return [
      ['entropy: bootstrap with no base file passes with base none', entropy('none'), 0],
      ['entropy: a normal land with no base file is refused (the tip is unreadable)', entropy('0'.repeat(40)), 1],
      ['entropy: a readable base runs the ratchet and passes', entropy('HEAD'), 0],
    ];
  })(),
  // land's runtime
  ['node: 20 is accepted', checkNode('20.0.0').ok, true],
  ['node: 18 is refused with a clear message', checkNode('18.19.0').reason, 'land needs Node 20 or newer; this is Node 18.19.0'],
  // the push list comes from the unfiltered collaborator list
  ['pushers: a non-admin writer counts, a reader does not, an admin counts', pushersOf('{"login":"w","push":true}\n{"login":"r","push":false}\n{"login":"a","push":true}\n{"login":"n","push":null}').join(), 'w,a'],
  ['output: zero-width and bidi marks are stripped', stripControls('a\u200bb\u200fc\ufeffd\u061ce'), 'abcde'],
  // PR output is quoted
  ['output: bidi overrides and line separators are stripped', quoteOutput('a\u202eb\u2066c\u2028d\u2029e'), 'preflight| abcde'],
  ['output: control characters are stripped, newline and tab kept', quoteOutput('a\u001b[2Jb\r\nc\td\u0007\n'), 'preflight| a[2Jb\npreflight| c\td'],
  // remotes
  ['remote: a repo with the same prefix is not matched', remoteFor(rem(['x', `https://github.com/${R}-old.git`]), R).error, `no git remote matches ${R}`],
  ['remote: a URL with userinfo is not matched', remoteFor(rem(['jm', `https://tok@github.com/${R}`]), R).error, `no git remote matches ${R}`],
  ['remote: an https remote matches and is named', remoteFor(rem(['jm', `https://github.com/${R}.git`]), R).remote, 'jm'],
  ['remote: with two remotes the matching one is picked', remoteFor(rem(['up', 'https://github.com/other/thing.git'], ['jm', `git@github.com:${R}.git`]), R).remote, 'jm'],
  ['cursor: a message with no time makes the paging cursor unusable', cursorOf([{ createdAt: '2026-10-29T10:00:00Z' }, { content: 'no time' }]), null],
  ['cursor: with times, one millisecond past the oldest', cursorOf([{ createdAt: '2026-10-29T10:00:00.000Z' }, { createdAt: '2026-10-29T11:00:00.000Z' }]), '2026-10-29T10:00:00.001Z'],
  ['remote: two matches refused', Boolean(remoteFor(rem(['a', `https://github.com/${R}`], ['b', `git@github.com:${R}.git`]), R).error), true],
  // main tip
  ['main moved is refused and names both tips', checkMain('1'.repeat(40), '2'.repeat(40)).ok, false],
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

// Comments are stripped, so a disconnected line (`// x();`) does not satisfy a source check. Everything else about land.mjs is in check-land-e2e.mjs.
const landSrc = fs.readFileSync(new URL('./land.mjs', import.meta.url), 'utf8').split('\n').map((l) => l.replace(/(^|\s)\/\/.*$/, '')).join('\n');

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
  ['token: no malformed-file error echoes any part of the contents', bads.every((b) => !/abc123|jmu_|note|extra/.test(b.error)), true],
  ['token: scrub removes the token from a message', scrub(`Bearer ${SECRET} rejected`, SECRET).includes('abc123'), false],
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

// What check-land-e2e.mjs cannot reach: the CLI entry (signals, the real spawn, the 30-minute limit, the group kill) and the imports.
const run = spawnSync(process.execPath, [fileURLToPath(new URL('./land.mjs', import.meta.url))], { encoding: 'utf8', env: { PATH: process.env.PATH } });
cases.push(
  ['land.mjs: the preflight child has a wall-clock limit', /LIMIT_MS/.test(landSrc) && /setTimeout/.test(landSrc), true],
  ['land.mjs: SIGHUP exits 129 like the others', /SIGHUP[^\n]*exit\(129\)/.test(landSrc), true],
  ['land.mjs: preflight output is scrubbed and the group is killed on return', landSrc.includes('scrub(quoteOutput(text), SECRET)') && /clearTimeout\(timer\);\s*clearTimeout\(late\);\s*kill\(\);/.test(landSrc), true],
  ['land.mjs: SIGINT and SIGTERM exit, which kills the group and releases the lock', /SIGINT[^\n]*exit\(130\)/.test(landSrc) && /SIGTERM[^\n]*exit\(143\)/.test(landSrc), true],
  ['land.mjs: the throwaway dir is tracked and the group kill runs first', landSrc.includes('trackThrowaway(dir, wt,') && landSrc.includes("process.prependListener('exit', kill)"), true],
  ['land.mjs imports resolve: run with no arguments it reaches main and refuses', run.status === 1 && !/export named|SyntaxError|ReferenceError/i.test(run.stderr) && run.stderr.includes('land: refused'), true],
);

// A signal removes the throwaway dir and its git worktree (land's handlers call exit, which runs trackThrowaway's cleanup).
{
  const sdir = fs.mkdtempSync(path.join(os.tmpdir(), 'check-signal-'));
  const env = { ...process.env, GIT_CONFIG_GLOBAL: '/dev/null', GIT_AUTHOR_NAME: 't', GIT_AUTHOR_EMAIL: 't@l', GIT_COMMITTER_NAME: 't', GIT_COMMITTER_EMAIL: 't@l' };
  const g = (a, cwd = path.join(sdir, 'repo')) => execFileSync('git', a, { cwd, encoding: 'utf8', env });
  fs.mkdirSync(path.join(sdir, 'repo'));
  g(['init', '-q']); fs.writeFileSync(path.join(sdir, 'repo', 'f'), 'x'); g(['add', '-A']); g(['commit', '-qm', 'a']);
  const lib = path.join(path.dirname(fileURLToPath(import.meta.url)), 'land-decide' + '.mjs'); // not a literal URL: knip would count the whole module as used
  const results = ['SIGINT', 'SIGTERM', 'SIGHUP'].map((sig) => {
    const dir = path.join(sdir, `throw-${sig}`);
    const wt = path.join(dir, 'wt');
    fs.mkdirSync(dir);
    g(['worktree', 'add', '--quiet', '--detach', wt]);
    const code = `import cp from 'node:child_process'; import { trackThrowaway } from ${JSON.stringify(lib)};
      process.on('SIGINT', () => process.exit(130)); process.on('SIGTERM', () => process.exit(143)); process.on('SIGHUP', () => process.exit(129));
      trackThrowaway(${JSON.stringify(dir)}, ${JSON.stringify(wt)}, (c, a) => cp.execFileSync(c, a, { stdio: 'ignore' })); process.kill(process.pid, ${JSON.stringify(sig)}); setTimeout(() => {}, 5000);`;
    const r = spawnSync(process.execPath, ['-e', code], { cwd: path.join(sdir, 'repo'), encoding: 'utf8', env });
    return [r.status, fs.existsSync(dir), g(['worktree', 'list', '--porcelain']).includes('throw-')];
  });
  cases.push(
    ['signal: SIGINT exits 130, removes the dir and the worktree', results[0].join(), '130,false,false'],
    ['signal: SIGTERM exits 143, removes the dir and the worktree', results[1].join(), '143,false,false'],
    ['signal: SIGHUP exits 129, removes the dir and the worktree', results[2].join(), '129,false,false'],
  );
  fs.rmSync(sdir, { recursive: true, force: true });
}

// A tag named main wins git's DWIM for a bare `main`; the explicit ref still gives the branch.
{
  const tdir2 = fs.mkdtempSync(path.join(os.tmpdir(), 'check-tag-'));
  const env = { ...process.env, GIT_CONFIG_GLOBAL: '/dev/null', GIT_AUTHOR_NAME: 't', GIT_AUTHOR_EMAIL: 't@l', GIT_COMMITTER_NAME: 't', GIT_COMMITTER_EMAIL: 't@l' };
  const g = (cwd, ...a) => execFileSync('git', a, { cwd, encoding: 'utf8', env }).trim();
  const [up, me] = [path.join(tdir2, 'up'), path.join(tdir2, 'me')];
  fs.mkdirSync(up); g(up, 'init', '-q', '-b', 'main'); g(up, 'commit', '-q', '--allow-empty', '-m', 'one'); const branchTip = g(up, 'rev-parse', 'HEAD');
  g(up, 'commit', '-q', '--allow-empty', '-m', 'two'); g(up, 'tag', 'main'); const tagTip = g(up, 'rev-parse', 'HEAD'); g(up, 'reset', '-q', '--hard', branchTip);
  fs.mkdirSync(me); g(me, 'init', '-q'); g(me, 'remote', 'add', 'up', up);
  const fetched = (ref) => { g(me, 'fetch', '--quiet', 'up', ref); return g(me, 'rev-parse', 'FETCH_HEAD'); };
  cases.push(
    ['tag named main: a bare `main` fetch picks the tag (the hole)', fetched('main'), tagTip],
    ['tag named main: refs/heads/main still gives the branch', fetched('refs/heads/main'), branchTip],
  );
  fs.rmSync(tdir2, { recursive: true, force: true });
}

const bad = cases.filter(([, got, want]) => got !== want).map(([n]) => n);
if (bad.length) { console.error(`check-land: ${bad.length} of ${cases.length} wrong\n  ${bad.join('\n  ')}`); process.exit(1); }
console.log(`check-land: ${cases.length} cases ok`);
