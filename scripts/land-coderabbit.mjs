// The CodeRabbit path of land (docs/development/landing.md, "The CodeRabbit path"): pure decisions over parsed GitHub API data. Land reads the
// data itself and checks every condition here; nothing is taken from a ledger row. Any missing or malformed field refuses. Cases: scripts/check-land-coderabbit.mjs.
import { isVerifier } from './land-decide.mjs';

const no = (reason) => ({ ok: false, reason });
const h8 = (s) => String(s).slice(0, 8);
// The App's user as the REST API gives it (reviews: login `coderabbitai[bot]`, type Bot) and as GraphQL gives it (author __typename Bot, login
// `coderabbitai`, the same account without the suffix). Exact and case-sensitive; a display name is never read.
export const isCodeRabbit = (u) => !!u && typeof u === 'object' && u.type === 'Bot' && u.login === 'coderabbitai[bot]';
export const isCodeRabbitAuthor = (a) => !!a && typeof a === 'object' && a.__typename === 'Bot' && a.login === 'coderabbitai';

// (a) The one local review happened on this PR's history: a verifier verdict (PASS or FAIL) for the PR at another head that is a commit of
// main..head. The latest such row is cited. `range` is `git rev-list <main tip>..<head>`.
export function checkEarlierReview(rows, { pr, head, range }) {
  const mine = new Set(range.map((s) => String(s).toLowerCase()));
  const hit = rows.filter((r) => r.pr === String(pr) && r.head !== head && ['PASS', 'FAIL'].includes(r.verdict) && isVerifier(r.by) && mine.has(r.head)).at(-1);
  return hit ? { ok: true, earlier: hit.head } : no(`#${pr} has no earlier verifier verdict at a head in this PR's history, so the one local review is missing`);
}
// (c) CodeRabbit reviewed this exact head, in one of two ways. Either a submitted CodeRabbit review whose commit_id is the head (COMMENTED and
// APPROVED count; DISMISSED and PENDING are ignored), or, because an incremental review that finds nothing files no review and only edits
// the summary comment, that comment's hidden coverage marker plus its status line (checkSummary). CHANGES_REQUESTED at the head refuses either way.
const CLEAN_LINE = 'No actionable comments were generated in the recent review.';
const MARKER = /<!--\s*final_review_risk_coverage:([\s\S]*?)-->/g;
const HEX40 = /^[0-9a-f]{40}$/;
function checkSummary(comments, head) {
  if (!Array.isArray(comments) || comments.some((c) => !c || typeof c !== 'object' || typeof c.body !== 'string')) return no(`CodeRabbit has not reviewed ${h8(head)}, and its summary comment could not be read (the comments reply is not a list of comments)`);
  // Only comments by CodeRabbit count; a person quoting the marker is ignored.
  const marked = comments.filter((c) => isCodeRabbit(c.user)).map((c) => ({ c, found: [...c.body.matchAll(MARKER)].map((m) => m[1]) })).filter((x) => x.found.length);
  if (marked.length !== 1 || marked[0].found.length !== 1) return no(`CodeRabbit has not reviewed ${h8(head)}, and ${marked.length} CodeRabbit comment(s) carry a coverage marker (exactly one is needed)`);
  let m;
  try { m = JSON.parse(marked[0].found[0]); } catch { return no('the CodeRabbit summary comment has a coverage marker that is not valid JSON'); }
  const ids = [m?.sourceCommitId, m?.coveredCommitId].map((x) => (typeof x === 'string' ? x.toLowerCase() : ''));
  if (!ids.every((x) => HEX40.test(x) && x === head) || m.kind !== 'reviewed') return no(`the CodeRabbit summary comment's coverage marker is not a review of ${h8(head)} (source, covered and kind must be that head, that head and "reviewed")`);
  const visible = marked[0].c.body.replace(/<!--[\s\S]*?-->/g, '');
  return visible.split('\n').some((l) => l.trim() === CLEAN_LINE) ? { ok: true } : no(`the CodeRabbit summary comment has no status line "${CLEAN_LINE}" outside HTML comments`);
}
export function checkReviews(reviews, head, comments) {
  if (!Array.isArray(reviews) || reviews.some((r) => !r || typeof r !== 'object' || typeof r.commit_id !== 'string' || typeof r.state !== 'string')) return no('CodeRabbit reviews: the reply is not a list of reviews');
  const mine = reviews.filter((r) => isCodeRabbit(r.user) && r.commit_id.toLowerCase() === head);
  if (mine.some((r) => r.state === 'CHANGES_REQUESTED')) return no(`CodeRabbit requested changes at ${h8(head)}`);
  if (mine.some((r) => ['COMMENTED', 'APPROVED'].includes(r.state))) return { ok: true, via: 'review' };
  const s = checkSummary(comments, head);
  return s.ok ? { ok: true, via: 'summary' } : s;
}
// Severity from the first non-empty line of a thread's first comment, where CodeRabbit writes it (`_⚠️ Potential issue_ | _🟠 Major_`).
// Low only when that line holds nothing but known label words and every severity word in it is Minor, Trivial or Nitpick; anything else
// (Major, Critical, a mix, an unknown word, no line) counts as Major.
const LABELS = ['potential', 'issue', 'refactor', 'suggestion', 'nitpick', 'trivial', 'minor', 'major', 'critical'];
const LOW = ['minor', 'trivial', 'nitpick'];
export function severityOf(body) {
  const first = String(body ?? '').replace(/<!--[\s\S]*?-->/g, '').split('\n').map((l) => l.trim()).find(Boolean) ?? '';
  const words = (first.match(/[a-z]+/gi) ?? []).map((w) => w.toLowerCase());
  const sev = words.filter((w) => ['major', 'critical', ...LOW].includes(w));
  return words.every((w) => LABELS.includes(w)) && sev.length && sev.every((w) => LOW.includes(w)) ? 'low' : 'major';
}
const wellFormed = (t) => t && typeof t === 'object' && typeof t.isResolved === 'boolean' && typeof t.isOutdated === 'boolean'
  && Array.isArray(t.comments?.nodes) && t.comments.pageInfo?.hasNextPage === false && t.comments.nodes.every((c) => c && typeof c === 'object');
// (d) No thread CodeRabbit opened is unresolved, not outdated and Major, unless a trusted login replied in it. `threads` are the PR's
// reviewThreads nodes (GraphQL), all pages.
export function checkThreads(threads, trusted) {
  if (!Array.isArray(threads) || !threads.every(wellFormed)) return no('CodeRabbit threads: the reply is not a complete list of review threads');
  const open = threads.filter((t) => !t.isResolved && !t.isOutdated && isCodeRabbitAuthor(t.comments.nodes[0]?.author));
  const unanswered = open.filter((t) => severityOf(t.comments.nodes[0].body) !== 'low'
    && !t.comments.nodes.slice(1).some((c) => c.author?.__typename === 'User' && typeof c.author.login === 'string' && trusted.includes(c.author.login.toLowerCase())));
  return unanswered.length ? no(`${unanswered.length} CodeRabbit thread(s) marked Major or Critical (or not readable as Minor) are unresolved and unanswered`) : { ok: true };
}
// (e) Every commit of main..head has a GitHub author AND committer login in the trusted list. `infos` are the API's commit objects; a commit
// the list does not return, or with a null login, refuses.
export function checkCommits(range, infos, trusted) {
  if (!range.length) return no('main..head holds no commits; cannot check their authors');
  const byRef = new Map((Array.isArray(infos) ? infos : []).filter((i) => i && typeof i.sha === 'string').map((i) => [i.sha.toLowerCase(), i]));
  for (const sha of range) {
    for (const role of ['author', 'committer']) {
      const login = byRef.get(sha)?.[role]?.login;
      if (typeof login !== 'string' || !trusted.includes(login.toLowerCase())) return no(`commit ${h8(sha)} has no ${role} login in JM_TRUSTED_LOGINS (a commit from an app or an unknown account is never landed)`);
    }
  }
  return { ok: true };
}
// All of (a), (c), (d), (e). (b), no FAIL at this head from anyone, is checkLedger's, which runs first.
export function checkCodeRabbit({ rows, pr, head, range, reviews, comments, threads, infos, trusted }) {
  const earlier = checkEarlierReview(rows, { pr, head, range });
  if (!earlier.ok) return earlier;
  const c = checkReviews(reviews, head, comments);
  const bad = [c, checkThreads(threads, trusted), checkCommits(range, infos, trusted)].find((x) => !x.ok);
  return bad ?? { ok: true, note: `coderabbit at ${h8(head)}${c.via === 'summary' ? ' (summary comment)' : ''}, after the review at ${h8(earlier.earlier)}` };
}
// One page of the reviewThreads GraphQL reply: its nodes and the next cursor (null on the last page). A reply with errors, no data or no
// pagination info throws, and land refuses.
export function threadPage(reply) {
  const t = reply?.data?.repository?.pullRequest?.reviewThreads;
  if (!t || !Array.isArray(t.nodes) || typeof t.pageInfo?.hasNextPage !== 'boolean') throw new Error('CodeRabbit threads: unexpected reply');
  if (t.pageInfo.hasNextPage && typeof t.pageInfo.endCursor !== 'string') throw new Error('CodeRabbit threads: a next page with no cursor');
  return { nodes: t.nodes, next: t.pageInfo.hasNextPage ? t.pageInfo.endCursor : null };
}
