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
const rev = (list, head = HEAD, comments) => checkReviews(list, head, comments);
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
// Trimmed copy of CodeRabbit's summary comment on PR 30 (issue comment 6727814247), in the format CodeRabbit moved to: no status block; the markers
// run review_stack_entry, walkthrough (change_assessment inside), final_review_risk, pre_merge_checks_walkthrough, tips. Its text is cut to
// placeholders. The marker covers an older head than the PR's (PR30_HEAD), the usual case once a PR is pushed again.
const PR30_HEAD = '0439187fb040eb6bffd5c654fc0516d0e8459ff7';
const PR30_COVERED = 'aa871e3a8301bd475d190dd74da2264c04beed80';
const newShape = ({ risk = `<!-- final_review_risk_coverage:{"sourceCommitId":"${PR30_COVERED}","coveredCommitId":"${PR30_COVERED}","kind":"reviewed"} -->`, head = GEN } = {}) => head
  + '<!-- review_stack_entry_start -->\n\n<a href="https://example.test/change-stack">Review stack</a>\n\n<!-- review_stack_entry_end -->\n'
  + '<!-- walkthrough_start -->\n\n<details>\n<summary>Walkthrough</summary>\nplaceholder text\n'
  + `<!-- change_assessment_start -->\n<!-- change_assessment_commit:"${PR30_COVERED}" -->\n<!-- change_assessment_priority:{"version":1,"urgency":"p1","manual":false} -->\nplaceholder\n<!-- change_assessment_end -->\n`
  + '</details>\n<!-- walkthrough_end -->\n'
  + `<!-- final_review_risk_start -->\n<!-- final_review_risk_label\n**Merge Risk:** **Low** · up to \`aa871\`\n-->\n${risk}\n\nplaceholder text\n<!-- final_review_risk_end -->\n`
  + '<!-- pre_merge_checks_walkthrough_start -->\nplaceholder\n<!-- pre_merge_checks_walkthrough_end -->\n<!-- tips_start -->\nplaceholder\n<!-- tips_end -->\n';
const PR30_SUMMARY = newShape();
const pr30 = (list = [{ user: BOTU, body: PR30_SUMMARY }], reviews = [], head = PR30_HEAD) => checkReviews(reviews, head, list);
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
  ['c2: the marker covers another commit: a gap, passes with no wait', cmts([withLayout({ risk: marker({ sourceCommitId: OLD, coveredCommitId: OLD }) })]).via, 'gap'],
  ['c2: source equals the head but covered is another commit: a gap', cmts([withLayout({ risk: marker({ coveredCommitId: OLD }) })]).via, 'gap'],
  ['c2: covered equals the head but source is another commit: a gap', cmts([withLayout({ risk: marker({ sourceCommitId: OLD }) })]).via, 'gap'],
  ['c2: kind other than reviewed is a gap: nothing was reviewed at this head', cmts([withLayout({ risk: marker({ kind: 'skipped' }) })]).via, 'gap'],
  ['c2: the same comment by a User (a human quoting it) is no CodeRabbit comment: a gap, never a summary', cmts([sum({ user: HUMAN })]).via, 'gap'],
  ['c2: a User with the bot login does not count as a summary', cmts([sum({ user: { login: 'coderabbitai[bot]', type: 'User' } })]).via, 'gap'],
  ['c2: the same comment by another Bot (github-actions[bot]) does not count as a summary', cmts([sum({ user: { login: 'github-actions[bot]', type: 'Bot' } })]).via, 'gap'],
  ['c2: another Bot\'s copy beside no CodeRabbit comment is a gap, and beside the real summary is ignored', [cmts([sum({ user: { login: 'github-actions[bot]', type: 'Bot' } })]).via, cmts([sum(), sum({ id: 2, user: { login: 'github-actions[bot]', type: 'Bot' } })]).via].join(), 'gap,summary'],
  ['c2: a not-clean status line (comments posted) at the head passes: the one review already happened', cmts([withLayout({ status: 'Actionable comments posted: 2' })]).via, 'summary'],
  ['c2: the status line only inside an HTML comment is not clean, and passes', cmts([withLayout({ status: `<!-- ${LINE} -->` })]).via, 'summary'],
  ['c2: the status line inside a multi-line HTML comment is not clean, and passes', cmts([withLayout({ status: `<!--\n${LINE}\n-->` })]).via, 'summary'],
  ['c2: a status line that is not a line of its own is not clean, and passes', cmts([withLayout({ status: `Note: ${LINE} (not really)` })]).via, 'summary'],
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
  ['c2: the reason names a missing marker', /marker/.test(cmts([withLayout({ risk: 'none' })]).reason), true],
  ['c2: a bad review list still refuses even with a good comment', checkReviews({ message: 'x' }, HEAD, [sum()]).ok, false],
  ['c2: d and e still run on the summary path', [all({ reviews: [], comments: [sum()], threads: [thread([comment(BOT, MAJOR)])] }).ok, all({ reviews: [], comments: [sum()], infos: [info(OLD), info(HEAD, 'mallory')] }).ok].join(), 'false,false'],
  ['c2: a is still required on the summary path', all({ reviews: [], comments: [sum()], rows: ledger(row(HEAD, 'PASS', 'coderabbit')) }).ok, false],
  // (c') only the summary comment, and only its own blocks, is trusted (echoes of steered text must not count)
  ['c3: a reply comment holding marker and line beside an unmarked summary refuses', cmts([withLayout({ risk: 'no marker yet' }), reply(`${LINE}\n${marker()}`)]).ok, false],
  ['c3: a reply holding the marker refuses outright, even beside a good summary', cmts([sum(), reply(marker())]).ok, false],
  ['c3: a reply holding the marker refuses with no summary at all', cmts([reply(`${LINE}\n${marker()}`)]).ok, false],
  ['c3: a rate-limited summary echoing marker and line is a gap, not a review: passes with no wait', cmts([withLayout({ head: `${GEN}<!-- This is an auto-generated comment: rate limited by coderabbit.ai -->\n` })]).via, 'gap'],
  ['c3: a skip-review summary echoing marker and line is a gap: passes', cmts([withLayout({ head: `${GEN}<!-- This is an auto-generated comment: skip review by coderabbit.ai -->\n` })]).via, 'gap'],
  ['c3: marker and line inside a code fence are quoted text: no readable status block, a gap, never a summary', cmts([withLayout({ status: '```\n' + LINE + '\n```', risk: '```\n' + marker() + '\n```' })]).via, 'gap'],
  ['c3: marker in the walkthrough, none in the risk block, refuses', cmts([withLayout({ walk: marker(), risk: 'nothing' })]).ok, false],
  ['c3: the line in the walkthrough, a different line in the status block: not clean, passes', cmts([withLayout({ walk: LINE, status: 'Actionable comments posted: 1' })]).via, 'summary'],
  ['c3: a marker echoed in the walkthrough beside the real one refuses (two markers in the comment)', cmts([withLayout({ walk: marker() })]).ok, false],
  ['c3: two summary comments refuse', cmts([sum(), sum({ id: 2 })]).ok, false],
  ['c3: the status block after the walkthrough start cannot be parsed: a gap, never a summary', cmts([sum({ body: layout({ order: 'status-late' }) })]).via, 'gap'],
  ['c3: a body without the generated header refuses', cmts([withLayout({ head: '' })]).ok, false],
  ['c3: a body with the generated line but not the summarize header refuses', cmts([withLayout({ head: '<!-- coderabbit-ai-generated:v1 -->\n' })]).ok, false],
  ['c3: leading whitespace before the generated line is allowed', cmts([withLayout({ head: `\n  ${GEN}` })]).ok, true],
  ['c3: a repeated block delimiter (an echoed fake block) refuses', cmts([sum({ body: layout() + '<!-- final_review_risk_start -->\n' + marker() + '\n<!-- final_review_risk_end -->\n' })]).ok, false],
  ['c3: each walkthrough or risk delimiter appearing twice refuses, even with no second marker', ['<!-- walkthrough_start -->', '<!-- final_review_risk_start -->', '<!-- final_review_risk_end -->'].map((d) => cmts([sum({ body: layout() + d + '\n' })]).ok).join(), 'false,false,false'],
  ['c3: a status delimiter appearing twice makes the status block unreadable: a gap, never a summary', ['<!-- recent_review_status_start -->', '<!-- recent_review_status_end -->'].map((d) => cmts([sum({ body: layout() + d + '\n' })]).via).join(), 'gap,gap'],
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
  // LCR3R: the gap shapes pass at once with no ask and no clock; the land note makes them countable
  ['r: no review and no comments at all is a gap, passes with no ask', rev([], HEAD, []).via, 'gap'],
  ['r: no review and no CodeRabbit comment among the comments is a gap', rev([], HEAD, [{ user: HUMAN, body: 'looks fine' }]).via, 'gap'],
  ['r: a gap passes through checkCodeRabbit with no ask, and the note says no CodeRabbit review at this head', all({ reviews: [], comments: [] }).note, `coderabbit at ${HEAD.slice(0, 8)} (no CodeRabbit review at this head), after the review at ${OLD.slice(0, 8)}`],
  ['r: a rate-limited summary passes through checkCodeRabbit with the same note', /no CodeRabbit review at this head/.test(all({ reviews: [], comments: [withLayout({ head: `${GEN}<!-- This is an auto-generated comment: rate limited by coderabbit.ai -->\n` })] }).note), true],
  ['r: a summary whose marker covers another commit passes with the same note', /no CodeRabbit review at this head/.test(all({ reviews: [], comments: [withLayout({ risk: marker({ sourceCommitId: OLD, coveredCommitId: OLD }) })] }).note), true],
  ['r: a not-clean summary at the head with no unanswered Major lands, and the note is the summary way', all({ reviews: [], comments: [withLayout({ status: 'Actionable comments posted: 2' })] }).note, `coderabbit at ${HEAD.slice(0, 8)} (summary comment), after the review at ${OLD.slice(0, 8)}`],
  ['r: a not-clean summary at the head beside an unanswered Major thread refuses', all({ reviews: [], comments: [withLayout({ status: 'Actionable comments posted: 1' })], threads: [thread([comment(BOT, MAJOR)])] }).ok, false],
  ['r: a gap beside an unanswered Major thread refuses', all({ reviews: [], comments: [], threads: [thread([comment(BOT, MAJOR)])] }).ok, false],
  ['r: a gap beside an unanswered Critical thread refuses', all({ reviews: [], comments: [], threads: [thread([comment(BOT, '_⚠️ Potential issue_ | _🔴 Critical_')])] }).ok, false],
  ['r: a gap beside a Major thread a trusted login answered lands', all({ reviews: [], comments: [], threads: [thread([comment(BOT, MAJOR), comment(user('maintainer-a'), 'fixed in abc')])] }).ok, true],
  ['r: CHANGES_REQUESTED at the head refuses with no comments, with a gap, and with a clean summary', [all({ reviews: [review({ state: 'CHANGES_REQUESTED' })], comments: [] }).ok, all({ reviews: [review({ state: 'CHANGES_REQUESTED' })], comments: undefined }).ok, all({ reviews: [review({ state: 'CHANGES_REQUESTED' })], comments: [sum()] }).ok].join(), 'false,false,false'],
  ['r: CHANGES_REQUESTED at the head is not a gap', rev([review({ state: 'CHANGES_REQUESTED' })], HEAD, []).via, undefined],
  ['r: a failed comments read (null, undefined, not a list) refuses', [all({ reviews: [], comments: null }).ok, all({ reviews: [], comments: undefined }).ok, all({ reviews: [], comments: { message: 'rate limited' } }).ok].join(), 'false,false,false'],
  ['r: a failed comments read is not a gap', rev([], HEAD, null).via, undefined],
  ['r: a reviews reply that is not a list refuses even with no comments', all({ reviews: { message: 'x' }, comments: [] }).ok, false],
  ['r: two summaries still refuse', rev([], HEAD, [sum(), sum({ id: 2 })]).ok, false],
  ['r: a marker in a reply still refuses beside a gap-shaped summary', rev([], HEAD, [withLayout({ head: `${GEN}<!-- This is an auto-generated comment: rate limited by coderabbit.ai -->\n` }), reply(marker())]).ok, false],
  ['r: a malformed marker still refuses', rev([], HEAD, [withLayout({ risk: '<!-- final_review_risk_coverage:{"sourceCommitId": -->' })]).ok, false],
  ['r: a repeated-key marker still refuses', rev([], HEAD, [withLayout({ risk: `<!-- final_review_risk_coverage:{"sourceCommitId":"${HEAD}","coveredCommitId":"${HEAD}","kind":"skipped","kind":"reviewed"} -->` })]).ok, false],
  ['r: a gap does not excuse (a) or (e): a missing earlier review and an untrusted commit still refuse', [all({ reviews: [], comments: [], rows: ledger(row(HEAD, 'PASS', 'coderabbit')) }).ok, all({ reviews: [], comments: [], infos: [info(OLD), info(HEAD, 'mallory')] }).ok].join(), 'false,false'],
  // LCR4: CodeRabbit's summary lost its status block (PR 30). A summary that cannot tell us about this head is a gap, not a refusal
  ['n: the PR 30 summary shape (no status block, marker covers an older head) is a gap', pr30().via, 'gap'],
  ['n: the PR 30 shape passes through checkCodeRabbit with the no-review note', all({ head: PR30_HEAD, range: [OLD, PR30_HEAD], infos: [info(OLD), info(PR30_HEAD)], reviews: [], comments: [{ user: BOTU, body: PR30_SUMMARY }] }).note, `coderabbit at ${PR30_HEAD.slice(0, 8)} (no CodeRabbit review at this head), after the review at ${OLD.slice(0, 8)}`],
  ['n: the PR 30 shape, a review at the older head only, is still a gap at the new head', pr30(undefined, [review({ commit_id: PR30_COVERED })]).via, 'gap'],
  ['n: the PR 30 shape plus CHANGES_REQUESTED at the head refuses', pr30(undefined, [review({ state: 'CHANGES_REQUESTED', commit_id: PR30_HEAD })]).ok, false],
  ['n: the PR 30 shape plus an unanswered Major thread refuses', all({ reviews: [], comments: [{ user: BOTU, body: PR30_SUMMARY }], threads: [thread([comment(BOT, MAJOR)])] }).ok, false],
  ['n: the PR 30 shape plus a Major thread a trusted login answered lands', all({ reviews: [], comments: [{ user: BOTU, body: PR30_SUMMARY }], threads: [thread([comment(BOT, MAJOR), comment(user('maintainer-a'), 'fixed in abc')])] }).ok, true],
  ['n: the PR 30 shape with kind skipped is a gap', pr30([{ user: BOTU, body: newShape({ risk: `<!-- final_review_risk_coverage:{"sourceCommitId":"${PR30_HEAD}","coveredCommitId":"${PR30_HEAD}","kind":"skipped"} -->` }) }]).via, 'gap'],
  ['n: the PR 30 shape whose marker covers the head itself reads as today (walkthrough and risk parse): a summary pass', pr30([{ user: BOTU, body: newShape({ risk: `<!-- final_review_risk_coverage:{"sourceCommitId":"${PR30_HEAD}","coveredCommitId":"${PR30_HEAD}","kind":"reviewed"} -->` }) }]).via, 'summary'],
  ['n: a non-bot comment holding the PR 30 shape is ignored and can never make the head pass as a summary', [pr30([{ user: HUMAN, body: PR30_SUMMARY }]).via, pr30([{ user: { login: 'github-actions[bot]', type: 'Bot' }, body: PR30_SUMMARY }]).via].join(), 'gap,gap'],
  ['n: two bot summaries in the PR 30 shape refuse', pr30([{ user: BOTU, body: PR30_SUMMARY }, { id: 2, user: BOTU, body: PR30_SUMMARY }]).ok, false],
  ['n: a CodeRabbit reply holding the marker refuses beside the PR 30 shape', pr30([{ user: BOTU, body: PR30_SUMMARY }, reply(marker())]).ok, false],
  ['n: the PR 30 shape with its walkthrough or risk block missing, and no status block, is a gap', [newShape().replace('<!-- walkthrough_start -->', ''), newShape().replace('<!-- final_review_risk_end -->', '')].map((b) => pr30([{ user: BOTU, body: b }]).via).join(), 'gap,gap'],
  ['n: the PR 30 shape with two markers, or a bad marker, and no status block, is a gap (nothing to forge)', [newShape({ risk: marker() + marker() }), newShape({ risk: '<!-- final_review_risk_coverage:{"sourceCommitId": -->' })].map((b) => pr30([{ user: BOTU, body: b }]).via).join(), 'gap,gap'],
  ['n: a status block that cannot be parsed (only its start delimiter) is a gap', cmts([sum({ body: layout().replace('<!-- recent_review_status_end -->\n', '') })]).via, 'gap'],
  ['n: the old well-formed clean summary keeps passing as a summary', [cmts([sum()]).via, checkReviews([], PR13_HEAD, [{ user: BOTU, body: PR13_SUMMARY }]).via].join(), 'summary,summary'],
  ['n: a well-formed status block with a marker that is not valid JSON still refuses', cmts([withLayout({ risk: '<!-- final_review_risk_coverage:{"sourceCommitId": -->' })]).ok, false],
  ['n: a gap shape does not excuse a failed reviews read', checkReviews({ message: 'x' }, PR30_HEAD, [{ user: BOTU, body: PR30_SUMMARY }]).ok, false],
  // all together
  ['all: a clean head passes and the note names the path and both heads', all().note, `coderabbit at ${HEAD.slice(0, 8)}, after the review at ${OLD.slice(0, 8)}`],
  ['all: each of a, c, d, e failing alone refuses', [all({ rows: ledger(row(HEAD, 'PASS', 'coderabbit')) }), all({ reviews: [review({ state: 'CHANGES_REQUESTED' })] }), all({ threads: [thread([comment(BOT, MAJOR)])] }), all({ infos: [info(OLD), info(HEAD, 'mallory')] })].map((c) => c.ok).join(), 'false,false,false,false'],
  ['all: a commit from main (not in main..head) is never read', all({ range: [OLD, HEAD], infos: [info(OLD), info(HEAD), info(MAIN_C, 'mallory')] }).ok, true],
];
const bad = cases.filter(([, got, want]) => got !== want).map(([n]) => n);
if (bad.length) { console.error(`check-land-coderabbit: ${bad.length} of ${cases.length} wrong\n  ${bad.join('\n  ')}`); process.exit(1); }
console.log(`check-land-coderabbit: ${cases.length} cases ok`);
