#!/usr/bin/env bun
// Tests the CodeRabbit path's pure decisions (scripts/land-coderabbit.mjs, and checkLedger's path in land-decide.mjs). No network: the thread and
// commit fixtures have the shape of GitHub's GraphQL and REST replies. Land-level cases are in check-land-e2e.mjs.
import { checkLedger, parseLedger } from './land-decide.mjs';
import {
  checkCodeRabbit, checkCommits, checkEarlierReview, checkReviews, checkThreads, isCodeRabbit, isCodeRabbitAuthor, severityOf, threadPage,
} from './land-coderabbit.mjs';

const [HEAD, OLD, FIX, MAIN_C] = ['a', 'b', 'c', 'd'].map((c) => c.repeat(40));
const TRUSTED = ['maintainer-a', 'maintainer-b'];
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
const info = (sha, author = 'maintainer-a', committer = 'maintainer-a') => ({ sha, author: { login: author }, committer: { login: committer } });
const cmt = (infos, range = RANGE) => checkCommits(range, infos, TRUSTED);
const good = { rows, pr: 5, head: HEAD, range: RANGE, reviews: [review()], comments: [], threads: [], infos: RANGE.map((s) => info(s)), trusted: TRUSTED };
const all = (o = {}) => checkCodeRabbit({ ...good, ...o });
const gql = (nodes, next = null) => ({ data: { repository: { pullRequest: { reviewThreads: { pageInfo: { hasNextPage: !!next, endCursor: next }, nodes } } } } });
// Trimmed copy of CodeRabbit's summary comment on PR 13 (issue comment 6725328362) at its head: the two header lines, the status block, the
// walkthrough delimiters and the risk block, with the walkthrough, tips and checks cut out.
const PR13_HEAD = 'd5ca009199e228676ad54fff308ad586814f0cce';
const PR13_SUMMARY = "<!-- coderabbit-ai-generated:v1 -->\n\n<!-- This is an auto-generated comment: summarize by coderabbit.ai -->\n<!-- recent_review_status_start -->\n\nNo actionable comments were generated in the recent review.\n\n<!-- recent_review_status_end -->\n<!-- walkthrough_start -->\n<!-- change_assessment_commit:\"d5ca009199e228676ad54fff308ad586814f0cce\" -->\n<!-- walkthrough_end -->\n<!-- final_review_risk_start -->\n<!-- final_review_risk_label\n**Merge Risk:** **⚪ Minimal** · up to `d5ca0`\n-->\n<!-- final_review_risk_coverage:{\"sourceCommitId\":\"d5ca009199e228676ad54fff308ad586814f0cce\",\"coveredCommitId\":\"d5ca009199e228676ad54fff308ad586814f0cce\",\"kind\":\"reviewed\"} -->\n\nThe documented corrections appear in the reviewed change, and no actionable merge-blocking issue remains in the supplied evidence.\n<!-- final_review_risk_end -->\n";
const LINE = 'No actionable comments were generated in the recent review.';
const GEN = '<!-- coderabbit-ai-generated:v1 -->\n\n<!-- This is an auto-generated comment: summarize by coderabbit.ai -->\n';
const marker = (o = {}) => `<!-- final_review_risk_coverage:${JSON.stringify({ sourceCommitId: HEAD, coveredCommitId: HEAD, kind: 'reviewed', ...o })} -->`;
const BOTU = { login: 'coderabbitai[bot]', type: 'Bot' };
const HUMAN = { login: 'maintainer-a', type: 'User' };
// A summary body in CodeRabbit's layout: header, status block, walkthrough, risk block. Each part can be replaced.
const layout = ({ head = GEN, status = LINE, walk = 'files changed', risk = marker(), order = 'normal' } = {}) => {
  const st = `<!-- recent_review_status_start -->\n\n${status}\n\n<!-- recent_review_status_end -->\n`;
  const wk = `<!-- walkthrough_start -->\n${walk}\n<!-- walkthrough_end -->\n`;
  const rk = `<!-- final_review_risk_start -->\n${risk}\n\nsome text\n<!-- final_review_risk_end -->\n`;
  return head + (order === 'status-late' ? wk + st + rk : st + wk + rk);
};
const sum = (o = {}) => ({ id: 1, user: BOTU, body: layout(), ...o });
const withLayout = (l, o = {}) => sum({ body: layout(l), ...o });
const reply = (text, o = {}) => ({ id: 9, user: BOTU, body: `<!-- coderabbit-ai-generated:v1 -->\n\n<!-- This is an auto-generated reply by CodeRabbit -->\n${text}\n`, ...o });
const cmts = (list, reviews = [], head = HEAD) => checkReviews(reviews, head, list);
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
  ['c: a review by a human at the head does not count', rev([review({ user: { login: 'maintainer-a', type: 'User' } })]).ok, false],
  ['c: a User with the bot\'s login does not count', rev([review({ user: { login: 'coderabbitai[bot]', type: 'User' } })]).ok, false],
  ['c: CHANGES_REQUESTED at the head refuses even beside a COMMENTED', rev([review(), review({ state: 'CHANGES_REQUESTED' })]).ok, false],
  ['c: a DISMISSED or PENDING review alone is not a review', [rev([review({ state: 'DISMISSED' })]).ok, rev([review({ state: 'PENDING' })]).ok].join(), 'false,false'],
  ['c: a review with no commit_id refuses the whole read', rev([review(), { user: { login: 'coderabbitai[bot]', type: 'Bot' }, state: 'COMMENTED' }]).ok, false],
  ['c: a reply that is not a list refuses', rev({ message: 'rate limited' }).ok, false],
  ['c: the head compares case-insensitively', rev([review({ commit_id: HEAD.toUpperCase() })]).ok, true],
  // (c') a clean incremental review files no review; CodeRabbit's summary comment carries the proof
  ['c2: the PR 13 summary comment (real shape) satisfies (c) with no review at the head', checkReviews([], PR13_HEAD, [{ user: BOTU, body: PR13_SUMMARY }]).ok, true],
  ['c2: the result says which way (c) was met', [checkReviews([review()], HEAD, []).via, cmts([sum()]).via].join(), 'review,summary'],
  ['c2: a good summary passes through checkCodeRabbit with no review, and the note names the way', all({ reviews: [], comments: [sum()] }).note, `coderabbit at ${HEAD.slice(0, 8)} (summary comment), after the review at ${OLD.slice(0, 8)}`],
  ['c2: the marker covers another commit', cmts([withLayout({ risk: marker({ sourceCommitId: OLD, coveredCommitId: OLD }) })]).ok, false],
  ['c2: source equals the head but covered is another commit', cmts([withLayout({ risk: marker({ coveredCommitId: OLD }) })]).ok, false],
  ['c2: covered equals the head but source is another commit', cmts([withLayout({ risk: marker({ sourceCommitId: OLD }) })]).ok, false],
  ['c2: kind other than reviewed', cmts([withLayout({ risk: marker({ kind: 'skipped' }) })]).ok, false],
  ['c2: the same comment by a User (a human quoting it)', cmts([sum({ user: HUMAN })]).ok, false],
  ['c2: a User with the bot login does not count', cmts([sum({ user: { login: 'coderabbitai[bot]', type: 'User' } })]).ok, false],
  ['c2: the same comment by another Bot (github-actions[bot]) does not count', cmts([sum({ user: { login: 'github-actions[bot]', type: 'Bot' } })]).ok, false],
  ['c2: another Bot\'s copy beside no CodeRabbit comment refuses, and beside the real summary is ignored', [cmts([sum({ user: { login: 'github-actions[bot]', type: 'Bot' } })]).ok, cmts([sum(), sum({ id: 2, user: { login: 'github-actions[bot]', type: 'Bot' } })]).ok].join(), 'false,true'],
  ['c2: the status line is missing', cmts([withLayout({ status: 'Actionable comments posted: 2' })]).ok, false],
  ['c2: the status line only inside an HTML comment', cmts([withLayout({ status: `<!-- ${LINE} -->` })]).ok, false],
  ['c2: the status line inside a multi-line HTML comment', cmts([withLayout({ status: `<!--\n${LINE}\n-->` })]).ok, false],
  ['c2: the status line must be a line of its own', cmts([withLayout({ status: `Note: ${LINE} (not really)` })]).ok, false],
  ['c2: two markers in the risk block name the marker count', [cmts([withLayout({ risk: `${marker()}\n${marker()}` })]).ok, /2 coverage markers/.test(cmts([withLayout({ risk: `${marker()}\n${marker()}` })]).reason)].join(), 'false,true'],
  ['c2: malformed JSON in the marker', cmts([withLayout({ risk: '<!-- final_review_risk_coverage:{"sourceCommitId": -->' })]).ok, false],
  ['c2: a marker that is JSON but not an object', cmts([withLayout({ risk: `<!-- final_review_risk_coverage:"${HEAD}" -->` })]).ok, false],
  ['c2: a duplicate key in the marker refuses (the last key would win)', cmts([withLayout({ risk: `<!-- final_review_risk_coverage:{"sourceCommitId":"${HEAD}","coveredCommitId":"${HEAD}","kind":"skipped","kind":"reviewed"} -->` })]).ok, false],
  ['c2: the comments reply is not an array', [cmts({ message: 'rate limited' }).ok, cmts(undefined).ok, cmts(null).ok].join(), 'false,false,false'],
  ['c2: a comment that is not an object refuses', cmts([sum(), null]).ok, false],
  ['c2: a CHANGES_REQUESTED review at the head refuses beside a good marker', cmts([sum()], [review({ state: 'CHANGES_REQUESTED' })]).ok, false],
  ['c2: a CHANGES_REQUESTED review at another head does not block', cmts([sum()], [review({ state: 'CHANGES_REQUESTED', commit_id: OLD })]).ok, true],
  ['c2: ids compare case-insensitively', cmts([withLayout({ risk: marker({ sourceCommitId: HEAD.toUpperCase(), coveredCommitId: HEAD.toUpperCase() }) })]).ok, true],
  ['c2: ids that are not 40 hex refuse', cmts([withLayout({ risk: marker({ sourceCommitId: HEAD.slice(0, 8), coveredCommitId: HEAD.slice(0, 8) }) })], [], HEAD.slice(0, 8)).ok, false],
  ['c2: a submitted review at the head still satisfies (c) with no comments read', checkReviews([review()], HEAD, undefined).ok, true],
  ['c2: the reasons name what was missing', [cmts([withLayout({ risk: 'none' })]).reason, cmts([withLayout({ status: 'x' })]).reason].map((r) => (/marker/.test(r) ? 'marker' : /status line/.test(r) ? 'line' : r)).join(), 'marker,line'],
  ['c2: a bad review list still refuses even with a good comment', checkReviews({ message: 'x' }, HEAD, [sum()]).ok, false],
  ['c2: d and e still run on the summary path', [all({ reviews: [], comments: [sum()], threads: [thread([comment(BOT, MAJOR)])] }).ok, all({ reviews: [], comments: [sum()], infos: [info(OLD), info(HEAD, 'mallory')] }).ok].join(), 'false,false'],
  ['c2: a is still required on the summary path', all({ reviews: [], comments: [sum()], rows: ledger(row(HEAD, 'PASS', 'coderabbit')) }).ok, false],
  // (c') only the summary comment, and only its own blocks, is trusted (echoes of steered text must not count)
  ['c3: a reply comment holding marker and line beside an unmarked summary refuses', cmts([withLayout({ risk: 'no marker yet' }), reply(`${LINE}\n${marker()}`)]).ok, false],
  ['c3: a reply holding the marker refuses outright, even beside a good summary', cmts([sum(), reply(marker())]).ok, false],
  ['c3: a reply holding the marker refuses with no summary at all', cmts([reply(`${LINE}\n${marker()}`)]).ok, false],
  ['c3: a rate-limited summary echoing marker and line refuses', cmts([withLayout({ head: `${GEN}<!-- This is an auto-generated comment: rate limited by coderabbit.ai -->\n` })]).ok, false],
  ['c3: a skip-review summary echoing marker and line refuses', cmts([withLayout({ head: `${GEN}<!-- This is an auto-generated comment: skip review by coderabbit.ai -->\n` })]).ok, false],
  ['c3: marker and line inside a code fence refuse', cmts([withLayout({ status: '```\n' + LINE + '\n```', risk: '```\n' + marker() + '\n```' })]).ok, false],
  ['c3: marker in the walkthrough, none in the risk block, refuses', cmts([withLayout({ walk: marker(), risk: 'nothing' })]).ok, false],
  ['c3: the line in the walkthrough, a different line in the status block, refuses', cmts([withLayout({ walk: LINE, status: 'Actionable comments posted: 1' })]).ok, false],
  ['c3: a marker echoed in the walkthrough beside the real one refuses (two markers in the comment)', cmts([withLayout({ walk: marker() })]).ok, false],
  ['c3: two summary comments refuse', cmts([sum(), sum({ id: 2 })]).ok, false],
  ['c3: the status block after the walkthrough start refuses', cmts([sum({ body: layout({ order: 'status-late' }) })]).ok, false],
  ['c3: a body without the generated header refuses', cmts([withLayout({ head: '' })]).ok, false],
  ['c3: a body with the generated line but not the summarize header refuses', cmts([withLayout({ head: '<!-- coderabbit-ai-generated:v1 -->\n' })]).ok, false],
  ['c3: leading whitespace before the generated line is allowed', cmts([withLayout({ head: `\n  ${GEN}` })]).ok, true],
  ['c3: a repeated block delimiter (an echoed fake block) refuses', cmts([sum({ body: layout() + '<!-- final_review_risk_start -->\n' + marker() + '\n<!-- final_review_risk_end -->\n' })]).ok, false],
  ['c3: a risk block before the walkthrough refuses', cmts([sum({ body: GEN + '<!-- recent_review_status_start -->\n' + LINE + '\n<!-- recent_review_status_end -->\n<!-- final_review_risk_start -->\n' + marker() + '\n<!-- final_review_risk_end -->\n<!-- walkthrough_start -->\nx\n<!-- walkthrough_end -->\n' })]).ok, false],
  ['c3: an unmarked CodeRabbit reply beside the real summary is ignored', cmts([sum(), reply('Review rate limited.')]).ok, true],
  // (d) severity and threads
  ['d: severity: Major and Critical are major', [MAJOR, '_⚠️ Potential issue_ | _🔴 Critical_\n\nx'].map(severityOf).join(), 'major,major'],
  ['d: severity: Minor, Trivial and Nitpick are low', [MINOR, '_🧹 Nitpick_ | _🔵 Trivial_\n\nx', '_🛠️ Refactor suggestion_ | _🟡 Minor_'].map(severityOf).join(), 'low,low,low'],
  ['d: severity: text land cannot read counts as major (fail closed)', ['', 'This will break prod.', '_⚠️ Potential issue_', 'Minor: this drops all data', '_🟡 Minor_ | _🟠 Major_', '_🟡 Minor and also fine_', undefined].map(severityOf).join(), 'major,major,major,major,major,major,major'],
  ['d: severity: only the first non-empty line is read, a hidden comment before it is dropped', [severityOf('\n\n<!-- x -->\n_🟡 Minor_\n\nMajor'), severityOf('<!-- Minor -->\nsome text')].join(), 'low,major'],
  ['d: an unresolved Major thread with no reply refuses', thr([thread([comment(BOT, MAJOR)])]).ok, false],
  ['d: a Critical thread with no reply refuses', thr([thread([comment(BOT, '_⚠️ Potential issue_ | _🔴 Critical_')])]).ok, false],
  ['d: an unreadable-severity thread with no reply refuses', thr([thread([comment(BOT, 'please look at this')])]).ok, false],
  ['d: a Major thread with a reply from a trusted login passes', thr([thread([comment(BOT, MAJOR), comment(user('Maintainer-A'), 'fixed in abc')])]).ok, true],
  ['d: a reply from an untrusted login does not answer it', thr([thread([comment(BOT, MAJOR), comment(user('mallory'), 'false positive')])]).ok, false],
  ['d: CodeRabbit\'s own follow-up does not answer it', thr([thread([comment(BOT, MAJOR), comment(BOT, 'still applies')])]).ok, false],
  ['d: a Bot account whose login is a trusted name does not answer it', thr([thread([comment(BOT, MAJOR), comment({ __typename: 'Bot', login: 'maintainer-b' }, 'ok')])]).ok, false],
  ['d: a resolved Major thread passes', thr([thread([comment(BOT, MAJOR)], { isResolved: true })]).ok, true],
  ['d: an outdated Major thread passes', thr([thread([comment(BOT, MAJOR)], { isOutdated: true })]).ok, true],
  ['d: an unresolved Minor thread passes', thr([thread([comment(BOT, MINOR)])]).ok, true],
  ['d: a thread opened by a person is not CodeRabbit\'s', thr([thread([comment(user('maintainer-a'), MAJOR)])]).ok, true],
  ['d: a thread opened by a deleted account is not CodeRabbit\'s', thr([thread([comment(null, MAJOR)])]).ok, true],
  ['d: one unanswered Major among answered and Minor ones refuses', thr([thread([comment(BOT, MINOR)]), thread([comment(BOT, MAJOR), comment(user('maintainer-b'), 'x')]), thread([comment(BOT, MAJOR)])]).ok, false],
  ['d: no threads passes', thr([]).ok, true],
  ['d: a thread with more comments than were read refuses', thr([thread([comment(BOT, MINOR)], { comments: { pageInfo: { hasNextPage: true }, nodes: [comment(BOT, MINOR)] } })]).ok, false],
  ['d: a malformed thread (no isResolved, no comments) refuses', [thr([{ isOutdated: false, comments: { pageInfo: { hasNextPage: false }, nodes: [] } }]).ok, thr([{ isResolved: false, isOutdated: false }]).ok, thr(null).ok].join(), 'false,false,false'],
  ['d: threadPage reads a page and its cursor', JSON.stringify(threadPage(gql([thread([])], 'cur'))).includes('"next":"cur"'), true],
  ['d: threadPage on the last page has no cursor', threadPage(gql([])).next, null],
  ['d: threadPage throws on a GraphQL error reply, a missing pageInfo, or a next page with no cursor', [throws(() => threadPage({ errors: [{ message: 'x' }], data: null })), throws(() => threadPage({ data: { repository: { pullRequest: { reviewThreads: { nodes: [] } } } } })), throws(() => threadPage({ data: { repository: { pullRequest: { reviewThreads: { pageInfo: { hasNextPage: true }, nodes: [] } } } } }))].join(), 'true,true,true'],
  // (e) every commit's author and committer are trusted logins
  ['e: trusted author and committer on every commit pass', cmt(RANGE.map((s) => info(s))).ok, true],
  ['e: an untrusted author refuses', cmt([info(OLD), info(HEAD, 'mallory')]).ok, false],
  ['e: an untrusted committer refuses', cmt([info(OLD), info(HEAD, 'maintainer-a', 'mallory')]).ok, false],
  ['e: an app as author or committer refuses', [cmt([info(OLD), info(HEAD, 'coderabbitai[bot]')]).ok, cmt([info(OLD), info(HEAD, 'maintainer-b', 'web-flow')]).ok].join(), 'false,false'],
  ['e: a null author login refuses (an unmatched email)', cmt([info(OLD), { sha: HEAD, author: { login: null }, committer: { login: 'maintainer-b' } }]).ok, false],
  ['e: a null committer, a missing author and an empty login refuse', [{ sha: HEAD, author: { login: 'maintainer-b' }, committer: { login: null } }, { sha: HEAD, committer: { login: 'maintainer-b' } }, info(HEAD, '')].map((i) => cmt([info(OLD), i]).ok).join(), 'false,false,false'],
  ['e: a commit the API did not return refuses', cmt([info(OLD)]).ok, false],
  ['e: a commit read under another sha does not stand in for it', cmt([info(OLD), info(FIX)]).ok, false],
  ['e: logins compare case-insensitively', cmt(RANGE.map((s) => info(s, 'Maintainer-A', 'MAINTAINER-B'))).ok, true],
  ['e: an empty range or a non-list refuses', [cmt([], []).ok, cmt(null).ok].join(), 'false,false'],
  // all together
  ['all: a clean head passes and the note names the path and both heads', all().note, `coderabbit at ${HEAD.slice(0, 8)}, after the review at ${OLD.slice(0, 8)}`],
  ['all: each of a, c, d, e failing alone refuses', [all({ rows: ledger(row(HEAD, 'PASS', 'coderabbit')) }), all({ reviews: [review({ commit_id: OLD })] }), all({ threads: [thread([comment(BOT, MAJOR)])] }), all({ infos: [info(OLD), info(HEAD, 'mallory')] })].map((c) => c.ok).join(), 'false,false,false,false'],
  ['all: a commit from main (not in main..head) is never read', all({ range: [OLD, HEAD], infos: [info(OLD), info(HEAD), info(MAIN_C, 'mallory')] }).ok, true],
];
const bad = cases.filter(([, got, want]) => got !== want).map(([n]) => n);
if (bad.length) { console.error(`check-land-coderabbit: ${bad.length} of ${cases.length} wrong\n  ${bad.join('\n  ')}`); process.exit(1); }
console.log(`check-land-coderabbit: ${cases.length} cases ok`);
