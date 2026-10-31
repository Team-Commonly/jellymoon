#!/usr/bin/env bun
// Tests the CodeRabbit path's pure decisions (scripts/land-coderabbit.mjs, and checkLedger's path in land-decide.mjs). No network: the thread and
// commit fixtures have the shape of GitHub's GraphQL and REST replies. Land-level cases are in check-land-e2e.mjs.
import { checkLedger, parseLedger } from './land-decide.mjs';
import {
  checkCodeRabbit, checkCommits, checkEarlierReview, checkReviews, checkThreads, isCodeRabbit, isCodeRabbitAuthor, severityOf, threadPage,
} from './land-coderabbit.mjs';

const [HEAD, OLD, FIX, MAIN_C] = ['a', 'b', 'c', 'd'].map((c) => c.repeat(40));
const TRUSTED = ['lilyshen0722', 'samxu01'];
const ledger = (...rows) => parseLedger(['pr\thead\tverdict\tby\twhen\tnote', ...rows.map((r) => r.join('\t'))].join('\n'));
const row = (head, verdict, by, pr = 5) => [pr, head, verdict, by, '2026-10-30T10:00:00Z', ''];
const led = (...rows) => checkLedger(ledger(...rows), { pr: '5', head: HEAD });
const RANGE = [OLD, HEAD]; // main..head: the reviewed head and this one
const rows = ledger(row(OLD, 'PASS', 'sonnet-verifier'), row(HEAD, 'PASS', 'coderabbit'));
const early = (r, o = {}) => checkEarlierReview(r, { pr: 5, head: HEAD, range: RANGE, ...o });

// REST review as the API gives it (the subset land reads), GraphQL thread as reviewThreads gives it.
const review = (o = {}) => ({ user: { login: 'coderabbitai[bot]', type: 'Bot' }, commit_id: HEAD, state: 'COMMENTED', ...o });
const rev = (list, head = HEAD) => checkReviews(list, head);
const BOT = { __typename: 'Bot', login: 'coderabbitai' };
const user = (login) => ({ __typename: 'User', login });
const comment = (author, body) => ({ author, body });
const thread = (comments, o = {}) => ({ isResolved: false, isOutdated: false, comments: { pageInfo: { hasNextPage: false }, nodes: comments }, ...o });
const MAJOR = '_⚠️ Potential issue_ | _🟠 Major_\n\n**The check can be skipped.**';
const MINOR = '_⚠️ Potential issue_ | _🟡 Minor_\n\nA wording problem.';
const thr = (list) => checkThreads(list, TRUSTED);
const info = (sha, author = 'lilyshen0722', committer = 'lilyshen0722') => ({ sha, author: { login: author }, committer: { login: committer } });
const cmt = (infos, range = RANGE) => checkCommits(range, infos, TRUSTED);
const good = { rows, pr: 5, head: HEAD, range: RANGE, reviews: [review()], threads: [], infos: RANGE.map((s) => info(s)), trusted: TRUSTED };
const all = (o = {}) => checkCodeRabbit({ ...good, ...o });
const gql = (nodes, next = null) => ({ data: { repository: { pullRequest: { reviewThreads: { pageInfo: { hasNextPage: !!next, endCursor: next }, nodes } } } } });
const throws = (f) => { try { f(); return false; } catch { return true; } };

const cases = [
  // the ledger row only names the path; (b) no FAIL at this head from anyone
  ['ledger: a coderabbit PASS alone names the coderabbit path', led(row(HEAD, 'PASS', 'coderabbit')).path, 'coderabbit'],
  ['ledger: a verifier PASS stays the verifier path', led(row(HEAD, 'PASS', 'opus-verifier')).path, 'verifier'],
  ['ledger: a verifier PASS beside a coderabbit PASS is the verifier path', led(row(HEAD, 'PASS', 'opus-verifier'), row(HEAD, 'PASS', 'coderabbit')).path, 'verifier'],
  ['ledger: a coderabbit FAIL refuses', led(row(HEAD, 'FAIL', 'coderabbit')).ok, false],
  ['ledger: a verifier FAIL at the head refuses a coderabbit PASS (b)', led(row(HEAD, 'FAIL', 'opus-verifier'), row(HEAD, 'PASS', 'coderabbit')).ok, false],
  ['ledger: a FAIL by anyone after the coderabbit PASS refuses (b)', led(row(HEAD, 'PASS', 'coderabbit'), row(HEAD, 'FAIL', 'someone-else')).ok, false],
  ['ledger: a FAIL at another head does not block this head', led(row(OLD, 'FAIL', 'opus-verifier'), row(HEAD, 'PASS', 'coderabbit')).ok, true],
  ['ledger: a coderabbit PASS at another head does not count', led(row(OLD, 'PASS', 'coderabbit')).ok, false],
  ['ledger: a lookalike (coderabbit-bot, coderabbitai) PASS refuses', ['coderabbit-bot', 'coderabbitai', 'x-coderabbit'].map((by) => led(row(HEAD, 'PASS', by)).ok).join(), 'false,false,false'],
  ['ledger: another non-verifier PASS still refuses', led(row(HEAD, 'PASS', 'someone-else')).ok, false],
  // (a) the one local review happened on this PR's history
  ['a: an earlier verifier PASS in the range is the review, and is cited', early(rows).earlier, OLD],
  ['a: an earlier verifier FAIL in the range counts too (the fix is the new head)', early(ledger(row(OLD, 'FAIL', 'haiku-verifier'))).ok, true],
  ['a: no earlier row refuses', early(ledger(row(HEAD, 'PASS', 'coderabbit'))).ok, false],
  ['a: a row at this head is not an earlier review', early(ledger(row(HEAD, 'PASS', 'opus-verifier'))).ok, false],
  ['a: a head that is not in main..head is not this PR\'s history', early(rows, { range: [HEAD] }).ok, false],
  ['a: a row of another PR does not count', early(ledger(row(OLD, 'PASS', 'opus-verifier', 6))).ok, false],
  ['a: an earlier row by a non-verifier does not count', early(ledger(row(OLD, 'PASS', 'coderabbit'), row(OLD, 'PASS', 'someone-else'))).ok, false],
  ['a: checkCodeRabbit refuses without it even when c, d and e hold', all({ rows: ledger(row(HEAD, 'PASS', 'coderabbit')) }).ok, false],
  // (c) CodeRabbit reviewed the exact head
  ['c: isCodeRabbit is the Bot type with the exact login', [{ login: 'coderabbitai[bot]', type: 'Bot' }, { login: 'coderabbitai[bot]', type: 'User' }, { login: 'CodeRabbitAI[bot]', type: 'Bot' }, { login: 'coderabbitai', type: 'Bot' }, { login: 'evil[bot]', type: 'Bot' }, null].map(isCodeRabbit).join(), 'true,false,false,false,false,false'],
  ['c: isCodeRabbitAuthor is the GraphQL Bot with login coderabbitai', [BOT, user('coderabbitai'), { __typename: 'Bot', login: 'coderabbitai[bot]' }, { __typename: 'Bot', login: 'other' }, null].map(isCodeRabbitAuthor).join(), 'true,false,false,false,false'],
  ['c: a COMMENTED review at the head passes', rev([review()]).ok, true],
  ['c: an APPROVED review at the head passes', rev([review({ state: 'APPROVED' })]).ok, true],
  ['c: a review on an older head is refused', rev([review({ commit_id: OLD })]).ok, false],
  ['c: no reviews is refused', rev([]).ok, false],
  ['c: a review by a human at the head does not count', rev([review({ user: { login: 'lilyshen0722', type: 'User' } })]).ok, false],
  ['c: a User with the bot\'s login does not count', rev([review({ user: { login: 'coderabbitai[bot]', type: 'User' } })]).ok, false],
  ['c: CHANGES_REQUESTED at the head refuses even beside a COMMENTED', rev([review(), review({ state: 'CHANGES_REQUESTED' })]).ok, false],
  ['c: a DISMISSED or PENDING review alone is not a review', [rev([review({ state: 'DISMISSED' })]).ok, rev([review({ state: 'PENDING' })]).ok].join(), 'false,false'],
  ['c: a review with no commit_id refuses the whole read', rev([review(), { user: { login: 'coderabbitai[bot]', type: 'Bot' }, state: 'COMMENTED' }]).ok, false],
  ['c: a reply that is not a list refuses', rev({ message: 'rate limited' }).ok, false],
  ['c: the head compares case-insensitively', rev([review({ commit_id: HEAD.toUpperCase() })]).ok, true],
  // (d) severity and threads
  ['d: severity: Major and Critical are major', [MAJOR, '_⚠️ Potential issue_ | _🔴 Critical_\n\nx'].map(severityOf).join(), 'major,major'],
  ['d: severity: Minor, Trivial and Nitpick are low', [MINOR, '_🧹 Nitpick_ | _🔵 Trivial_\n\nx', '_🛠️ Refactor suggestion_ | _🟡 Minor_'].map(severityOf).join(), 'low,low,low'],
  ['d: severity: text land cannot read counts as major (fail closed)', ['', 'This will break prod.', '_⚠️ Potential issue_', 'Minor: this drops all data', '_🟡 Minor_ | _🟠 Major_', '_🟡 Minor and also fine_', undefined].map(severityOf).join(), 'major,major,major,major,major,major,major'],
  ['d: severity: only the first non-empty line is read, a hidden comment before it is dropped', [severityOf('\n\n<!-- x -->\n_🟡 Minor_\n\nMajor'), severityOf('<!-- Minor -->\nsome text')].join(), 'low,major'],
  ['d: an unresolved Major thread with no reply refuses', thr([thread([comment(BOT, MAJOR)])]).ok, false],
  ['d: a Critical thread with no reply refuses', thr([thread([comment(BOT, '_⚠️ Potential issue_ | _🔴 Critical_')])]).ok, false],
  ['d: an unreadable-severity thread with no reply refuses', thr([thread([comment(BOT, 'please look at this')])]).ok, false],
  ['d: a Major thread with a reply from a trusted login passes', thr([thread([comment(BOT, MAJOR), comment(user('LilyShen0722'), 'fixed in abc')])]).ok, true],
  ['d: a reply from an untrusted login does not answer it', thr([thread([comment(BOT, MAJOR), comment(user('mallory'), 'false positive')])]).ok, false],
  ['d: CodeRabbit\'s own follow-up does not answer it', thr([thread([comment(BOT, MAJOR), comment(BOT, 'still applies')])]).ok, false],
  ['d: a Bot account whose login is a trusted name does not answer it', thr([thread([comment(BOT, MAJOR), comment({ __typename: 'Bot', login: 'samxu01' }, 'ok')])]).ok, false],
  ['d: a resolved Major thread passes', thr([thread([comment(BOT, MAJOR)], { isResolved: true })]).ok, true],
  ['d: an outdated Major thread passes', thr([thread([comment(BOT, MAJOR)], { isOutdated: true })]).ok, true],
  ['d: an unresolved Minor thread passes', thr([thread([comment(BOT, MINOR)])]).ok, true],
  ['d: a thread opened by a person is not CodeRabbit\'s', thr([thread([comment(user('lilyshen0722'), MAJOR)])]).ok, true],
  ['d: a thread opened by a deleted account is not CodeRabbit\'s', thr([thread([comment(null, MAJOR)])]).ok, true],
  ['d: one unanswered Major among answered and Minor ones refuses', thr([thread([comment(BOT, MINOR)]), thread([comment(BOT, MAJOR), comment(user('samxu01'), 'x')]), thread([comment(BOT, MAJOR)])]).ok, false],
  ['d: no threads passes', thr([]).ok, true],
  ['d: a thread with more comments than were read refuses', thr([thread([comment(BOT, MINOR)], { comments: { pageInfo: { hasNextPage: true }, nodes: [comment(BOT, MINOR)] } })]).ok, false],
  ['d: a malformed thread (no isResolved, no comments) refuses', [thr([{ isOutdated: false, comments: { pageInfo: { hasNextPage: false }, nodes: [] } }]).ok, thr([{ isResolved: false, isOutdated: false }]).ok, thr(null).ok].join(), 'false,false,false'],
  ['d: threadPage reads a page and its cursor', JSON.stringify(threadPage(gql([thread([])], 'cur'))).includes('"next":"cur"'), true],
  ['d: threadPage on the last page has no cursor', threadPage(gql([])).next, null],
  ['d: threadPage throws on a GraphQL error reply, a missing pageInfo, or a next page with no cursor', [throws(() => threadPage({ errors: [{ message: 'x' }], data: null })), throws(() => threadPage({ data: { repository: { pullRequest: { reviewThreads: { nodes: [] } } } } })), throws(() => threadPage({ data: { repository: { pullRequest: { reviewThreads: { pageInfo: { hasNextPage: true }, nodes: [] } } } } }))].join(), 'true,true,true'],
  // (e) every commit's author and committer are trusted logins
  ['e: trusted author and committer on every commit pass', cmt(RANGE.map((s) => info(s))).ok, true],
  ['e: an untrusted author refuses', cmt([info(OLD), info(HEAD, 'mallory')]).ok, false],
  ['e: an untrusted committer refuses', cmt([info(OLD), info(HEAD, 'lilyshen0722', 'mallory')]).ok, false],
  ['e: an app as author or committer refuses', [cmt([info(OLD), info(HEAD, 'coderabbitai[bot]')]).ok, cmt([info(OLD), info(HEAD, 'samxu01', 'web-flow')]).ok].join(), 'false,false'],
  ['e: a null author login refuses (an unmatched email)', cmt([info(OLD), { sha: HEAD, author: { login: null }, committer: { login: 'samxu01' } }]).ok, false],
  ['e: a null committer, a missing author and an empty login refuse', [{ sha: HEAD, author: { login: 'samxu01' }, committer: { login: null } }, { sha: HEAD, committer: { login: 'samxu01' } }, info(HEAD, '')].map((i) => cmt([info(OLD), i]).ok).join(), 'false,false,false'],
  ['e: a commit the API did not return refuses', cmt([info(OLD)]).ok, false],
  ['e: a commit read under another sha does not stand in for it', cmt([info(OLD), info(FIX)]).ok, false],
  ['e: logins compare case-insensitively', cmt(RANGE.map((s) => info(s, 'LilyShen0722', 'SAMXU01'))).ok, true],
  ['e: an empty range or a non-list refuses', [cmt([], []).ok, cmt(null).ok].join(), 'false,false'],
  // all together
  ['all: a clean head passes and the note names the path and both heads', all().note, `coderabbit at ${HEAD.slice(0, 8)}, after the review at ${OLD.slice(0, 8)}`],
  ['all: each of a, c, d, e failing alone refuses', [all({ rows: ledger(row(HEAD, 'PASS', 'coderabbit')) }), all({ reviews: [review({ commit_id: OLD })] }), all({ threads: [thread([comment(BOT, MAJOR)])] }), all({ infos: [info(OLD), info(HEAD, 'mallory')] })].map((c) => c.ok).join(), 'false,false,false,false'],
  ['all: a commit from main (not in main..head) is never read', all({ range: [OLD, HEAD], infos: [info(OLD), info(HEAD), info(MAIN_C, 'mallory')] }).ok, true],
];
const bad = cases.filter(([, got, want]) => got !== want).map(([n]) => n);
if (bad.length) { console.error(`check-land-coderabbit: ${bad.length} of ${cases.length} wrong\n  ${bad.join('\n  ')}`); process.exit(1); }
console.log(`check-land-coderabbit: ${cases.length} cases ok`);
