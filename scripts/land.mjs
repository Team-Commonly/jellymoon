#!/usr/bin/env node
// make land PR=<n>: the only merge path (docs/development/landing.md). Thin shell-outs to git, gh and the pod API;
// every decision is a pure function in land-decide.mjs, tested by check-land.mjs. Read-only unless merging:
// --dry-run writes no ledger row. PR code runs here only after the trust gate (checkTrust); the preflight is a
// correctness check, not a security boundary.
//   land.mjs <pr> [--dry-run] [--bootstrap]
//   land.mjs <pr> --preflight-only [--bootstrap]   (verifiers: the trust gate and the merge-result preflight only; needs JM_REPO and JM_TRUSTED_LOGINS)
// Run it from a clean checkout of main: the accepted reviewers come from operator env, never from the repo.
import { execFileSync, spawn, spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {
  acquireLock, appendLedger as writeLedger, checkApi, checkCheckout, checkNode, checkNoBunConfig, checkHead, checkLedger, checkMain, checkPodVerdict, checkRuns, landedNote, mergeAction,
  checkTrust, cursorOf, entropyBase, landAction, parseLedger, parseLogins, pushersOf, trackThrowaway, mergeTree, quoteOutput, stripControls, parseReviewers, readToken, remoteFor,
  scrub, scanEnv, takePage, requiredChecks, ciProof, landRef,
} from './land-decide.mjs';
import { scanRange } from './land-scan.mjs';
import { checkCodeRabbit, checkEarlierReview, checkReviews, threadPage } from './land-coderabbit.mjs';
const REQUIRED = ['JM_REPO', 'JM_TRUSTED_LOGINS', 'JM_LEDGER', 'JM_COMMONLY_API', 'JM_COMMONLY_TOKEN_FILE', 'JM_VERDICT_POD', 'JM_VERDICT_REVIEWERS'];
// All I/O goes through one seam, `deps`, so check-land-e2e.mjs can run main() against a scripted fake: exec(cmd, args, opts) for every
// git and gh call (stdout, throws on failure), run(cmd, args, opts) for the range scan ({ status, stdout, stderr }, never throws), fetch for the pod read, spawn for the preflight child, env, argv and the Node version.
export class Refusal extends Error {}
let IO;
let REPO = ''; // every gh call names it: a worktree can carry remotes for several repos
let DRY = false;
let SECRET = ''; // the bearer token once read; scrubbed from anything printed
const out = (cmd, args, opts = {}) => IO.exec(cmd, args, opts);
const gh = (...args) => out('gh', args);
const say = (m) => console.log(stripControls(m)); // every string land prints is stripped: GitHub text (a branch name, a url) can carry escapes
const refuse = (m) => { throw new Refusal(stripControls(scrub(m, SECRET))); };
const readLedger = () => parseLedger(fs.existsSync(IO.env.JM_LEDGER) ? fs.readFileSync(IO.env.JM_LEDGER, 'utf8') : '');
const appendLedger = (pr, head, verdict, by, note) => writeLedger(IO.env.JM_LEDGER, [pr, head, verdict, by, new Date().toISOString(), note], { dry: DRY });
function viewPr(n) {
  const pr = JSON.parse(gh('pr', 'view', String(n), '--repo', REPO, '--json', 'url,number,state,baseRefName,headRefName,headRefOid,createdAt,isCrossRepository,author'));
  if (pr.url !== `https://github.com/${REPO}/pull/${n}`) refuse(`#${n} resolved to ${pr.url}, not ${REPO}`);
  return pr;
}
// One land at a time per clone (two clones can land at once): an mkdir lock in the git common dir; see acquireLock.
function lock() {
  const dir = path.join(path.resolve(out('git', ['rev-parse', '--git-common-dir'])), 'jm-land.lock');
  const got = acquireLock(dir);
  if (!got.ok) refuse(got.reason);
  const free = () => fs.rmSync(dir, { recursive: true, force: true });
  process.on('exit', free);
  return () => { process.off('exit', free); free(); };
}
// Fetches main and (optionally) the PR head from the one remote that is JM_REPO.
const MAIN_REF = 'refs/heads/main'; // never bare `main`: a tag named main wins git's DWIM
function fetchRef(ref) {
  const m = remoteFor(out('git', ['remote', '-v']), REPO);
  if (m.error) refuse(m.error);
  out('git', ['fetch', '--quiet', m.remote, ref]);
  return out('git', ['rev-parse', 'FETCH_HEAD']);
}
const why = (e) => scrub(String(e.stderr || e.message).split('\n').find(Boolean) || 'unknown', SECRET);
const BASE = ['-c', 'core.hooksPath=/dev/null', '-c', 'user.name=land', '-c', 'user.email=land@localhost'];
// Runs ./init.sh (which runs make preflight) in the throwaway worktree: env -i, stdin /dev/null, a NEW session (no
// controlling tty), output captured and quoted, a wall-clock limit, and the whole process group killed when it returns.
const LIMIT_MS = 30 * 60 * 1000;
function runInit(wt, env) {
  return new Promise((done) => {
    let text = '';
    let over = false;
    let ended = false;
    const child = IO.spawn('env', ['-i', ...Object.entries(env).map(([k, v]) => `${k}=${v}`), 'bash', '-c', './init.sh'], { cwd: wt, detached: true, stdio: ['ignore', 'pipe', 'pipe'] });
    const take = (b) => { if (text.length < 2e6) text += b.toString('utf8'); };
    child.stdout.on('data', take);
    child.stderr.on('data', take);
    const kill = () => { try { IO.kill(-child.pid, 'SIGKILL'); } catch { /* the group is already gone */ } };
    const grace = IO.graceMs ?? 5000;
    let late;
    process.prependListener('exit', kill); // a crash or Ctrl-C of land must not leave the group running, and it dies before its dir is removed
    const timer = setTimeout(() => { over = true; kill(); late = setTimeout(() => end(-1), grace); }, IO.limitMs ?? LIMIT_MS); // an escapee holding the pipes cannot stall us
    const end = (code) => {
      if (ended) return;
      ended = true;
      clearTimeout(timer);
      clearTimeout(late);
      kill();
      process.off('exit', kill);
      if (text) console.error(scrub(quoteOutput(text), SECRET));
      if (over) return done({ ok: false, reason: `./init.sh did not finish in ${LIMIT_MS / 60000} minutes; killed` });
      return done(code === 0 ? { ok: true } : { ok: false, reason: 'clean-shell ./init.sh (baseline preflight) failed on the merge result' });
    };
    child.on('error', () => end(-1));
    // 'exit' is the child itself; 'close' waits for every holder of its pipes. Kill the group at exit so a descendant holding a pipe cannot stall land.
    child.on('exit', (code) => { kill(); late = setTimeout(() => end(code ?? -1), grace); });
    child.on('close', end);
  });
}
// Throwaway worktree at main's tip, the PR head merged in as GitHub would (no repo hooks), then ./init.sh under env -i with
// HOME and TMPDIR inside the throwaway dir and system dirs only on PATH. Hygiene, not isolation.
async function preflight(mainTip, head, bootstrap, expectTree) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'jm-land-'));
  const wt = path.join(dir, 'wt');
  for (const d of ['home', 'tmp']) fs.mkdirSync(path.join(dir, d));
  const release = trackThrowaway(dir, wt, (c, a) => out(c, a));
  const env = { HOME: path.join(dir, 'home'), TMPDIR: path.join(dir, 'tmp'), PATH: '/usr/bin:/bin:/usr/sbin:/sbin', TERM: 'dumb', JM_ENTROPY_BASE: entropyBase(mainTip, { bootstrap, mainHasEntropy: has(mainTip, 'entropy.json') }) }; // main's tip as fetched, never a ref the PR could move
  let result = { ok: true };
  let tree = '';
  try {
    try { out('git', ['-c', 'core.hooksPath=/dev/null', 'worktree', 'add', '--quiet', '--detach', wt, mainTip]); } catch (e) { result = { ok: false, reason: `git worktree add failed: ${why(e)}` }; }
    if (result.ok) try { out('git', ['-C', wt, ...BASE, 'merge', '--no-ff', '--no-edit', head]); } catch {
      result = { ok: false, reason: `merging ${head.slice(0, 8)} into main ${mainTip.slice(0, 8)} conflicts` };
    }
    if (result.ok) tree = out('git', ['-C', wt, 'rev-parse', 'HEAD^{tree}']);
    if (result.ok && expectTree && tree !== expectTree) result = { ok: false, reason: `the preflight's merge (tree ${tree.slice(0, 8)}) is not the merge result the decision used (${expectTree.slice(0, 8)})` };
    if (result.ok) result = await runInit(wt, env);
  } catch (e) { result = { ok: false, reason: `preflight failed: ${why(e)}` }; }
  release();
  return { ...result, tree };
}
// Pod messages newest-first, paged back past `since` (ms), deduped by id. Fails closed if the page cap is hit first,
// or if a page boundary cannot be paged past (see takePage).
async function podMessages(since) {
  SECRET = readToken(IO.env.JM_COMMONLY_TOKEN_FILE);
  const all = [];
  const seen = new Set();
  let before = '';
  for (let page = 0; page < 30; page += 1) {
    const url = `${IO.env.JM_COMMONLY_API}/api/messages/${IO.env.JM_VERDICT_POD}?limit=200${before}`;
    let res;
    try { res = await IO.fetch(url, { headers: { Authorization: `Bearer ${SECRET}` } }); } catch (e) { return refuse(`pod messages: request failed (${e.name})`); }
    if (!res.ok) refuse(`pod messages: HTTP ${res.status}`);
    let msgs;
    try { msgs = await res.json(); } catch (e) { return refuse(`pod messages: unreadable reply (${e.name})`); }
    if (!Array.isArray(msgs) || msgs.some((m) => !m || typeof m !== 'object' || Array.isArray(m))) refuse('pod messages: the reply is not a list of messages');
    const { fresh, stuck } = takePage(seen, msgs, 200);
    if (stuck) refuse('pod messages: a page boundary could hide a message; cannot prove no older FAIL');
    if (!fresh.length) return all;
    all.push(...fresh);
    const cursor = cursorOf(msgs);
    if (!cursor) refuse('pod messages: a message has no timestamp; cannot page');
    if (Date.parse(cursor) - 1 < since) return all;
    before = `&before=${encodeURIComponent(cursor)}`;
  }
  return refuse('pod messages: page cap reached before paging back past the PR\'s creation; cannot prove no older FAIL');
}
const has = (ref, file) => { try { out('git', ['cat-file', '-e', `${ref}:${file}`]); return true; } catch { return false; } };
// The first land of the gate is --bootstrap and only while main has no land.mjs; after that --bootstrap is refused. Both paths.
function bootstrapGate(mainTip, bootstrap) {
  if (has(mainTip, 'scripts/land.mjs') === bootstrap) refuse(bootstrap ? 'main already has scripts/land.mjs; --bootstrap is only for the first land of the gate' : 'main has no scripts/land.mjs yet; the first land of the gate needs --bootstrap and is judged by people');
}
// Trust gate: runs before anything of the PR is fetched or executed. Trust is GitHub identity: not cross-repository, the
// author and every account with push access are in JM_TRUSTED_LOGINS (an operator env var; a PR cannot widen it).
function trustGate(pr) {
  const pushers = pushersOf(gh('api', '--paginate', `repos/${REPO}/collaborators?affiliation=all&per_page=100`, '--jq', '.[] | {login, push: .permissions.push}'));
  const t = checkTrust({ pr: { isCrossRepository: pr.isCrossRepository, author: pr.author?.login }, pushers, trusted: parseLogins(IO.env.JM_TRUSTED_LOGINS) });
  if (!t.ok) refuse(`#${pr.number}: ${t.reason}`);
}
// Accepted reviewers, from operator env only: checked before the token is read or the pod fetched. Unset is caught with the other
// variables; a value with no valid `<id>:<Name>` entry is malformed.
const reviewers = () => {
  const list = parseReviewers(IO.env.JM_VERDICT_REVIEWERS);
  if (!list.length) refuse('JM_VERDICT_REVIEWERS is malformed: it has no valid <id>:<Name> entry');
  return list;
};
// The commit-range secret scan (gitleaks git mode, -m, markers ignored), run by scanRange (land-scan.mjs): land runs the checkout's own
// installed go with an argv it builds (its own gitleaks config included), not make. It only refuses. Before the scan, main's tree and every
// range commit's tree are read for a file that would steer the scan (checkScanConfig), and the checkout's .tools is checked. Clean is positive evidence read from the scan's own
// exit status AND stderr (see scanResult): exit 0, no hit or error line, "no leaks found", and a scanned count equal to the commits gitleaks
// reads (those whose diff has a text hunk, see textCommits). A range that touches a path ending in a backslash is refused first, because the
// count cannot show which commits were read. A scan that did not read the range proves nothing, so it refuses too.
const rangeScan = (mainTip, head) => {
  const top = out('git', ['rev-parse', '--show-toplevel']);
  const git = (a) => out('git', a, { env: scanEnv(IO.env), maxBuffer: 1 << 28 }); // the scan's own git environment, so both count the same way
  const r = scanRange({ range: `${mainTip}..${head}`, top, git, run: IO.run, env: IO.env });
  if (r.result === 'clean') return { ok: true };
  if (r.result === 'refused') return { ok: false, reason: r.reason };
  return { ok: false, reason: r.result === 'hit' ? 'a secret was found in the range; remove it from history and push a new head' : 'the range secret scan failed to run, or did not read the whole range; it proves nothing, so land refuses' };
};
// The CodeRabbit path's reads, all from GitHub and git at this moment, never from the ledger row: the PR's commits, CodeRabbit's reviews, and
// the review threads (GraphQL, paginated).
const jsonLines = (text) => text.split('\n').filter(Boolean).map((l) => JSON.parse(l));
const THREADS = 'query($owner:String!,$name:String!,$n:Int!,$after:String){repository(owner:$owner,name:$name){pullRequest(number:$n){reviewThreads(first:100,after:$after){pageInfo{hasNextPage endCursor}nodes{isResolved isOutdated comments(first:100){pageInfo{hasNextPage}nodes{author{__typename login}body}}}}}}}';
function reviewThreads(n) {
  const [owner, name] = REPO.split('/');
  const nodes = [];
  let after = [];
  for (let page = 0; page < 20; page += 1) {
    const got = threadPage(JSON.parse(gh('api', 'graphql', '-f', `query=${THREADS}`, '-f', `owner=${owner}`, '-f', `name=${name}`, '-F', `n=${n}`, ...after)));
    nodes.push(...got.nodes);
    if (!got.next) return nodes;
    after = ['-f', `after=${got.next}`];
  }
  return refuse('CodeRabbit threads: more than 2000 review threads; cannot read them all');
}
function codeRabbitGate(rows, n, head, mainTip) {
  const range = out('git', ['rev-list', `${mainTip}..${head}`], { maxBuffer: 1 << 26 }).split('\n').filter(Boolean).map((s) => s.toLowerCase());
  if (range.length > 250) refuse(`${range.length} commits in main..head; the CodeRabbit path reads at most 250`);
  const early = checkEarlierReview(rows, { pr: n, head, range }); // no network read if the one local review is missing
  if (!early.ok) return early;
  const reviews = jsonLines(gh('api', '--paginate', `repos/${REPO}/pulls/${n}/reviews?per_page=100`, '--jq', '.[] | {user: {login: .user.login, type: .user.type}, commit_id, state}'));
  // A clean incremental review files no review, only an edit of CodeRabbit's summary comment: read the comments only when no review at the head settles (c).
  const readComments = () => { try { return jsonLines(gh('api', '--paginate', `repos/${REPO}/issues/${n}/comments?per_page=100`, '--jq', '.[] | {user: {login: .user.login, type: .user.type}, body}')); } catch { return null; } }; // a failed read is no list: (c) refuses
  const comments = checkReviews(reviews, head).ok ? undefined : readComments();
  const infos = range.map((sha) => JSON.parse(gh('api', `repos/${REPO}/commits/${sha}`, '--jq', '{sha, author: {login: .author.login}, committer: {login: .committer.login}}')));
  return checkCodeRabbit({ rows, pr: n, head, range, reviews, comments, threads: reviewThreads(n), infos, trusted: parseLogins(IO.env.JM_TRUSTED_LOGINS) });
}
// Every ledger, CI and pod check at this moment. Run before the preflight and again right before the merge.
async function decide(pr, mainTip) { // mainTip is the tip fetched for this land
  const n = String(pr.number);
  const head = pr.headRefOid.toLowerCase();
  const rows = readLedger();
  const led = checkLedger(rows, { pr: n, head });
  if (!led.ok) return led;
  let via = `verifier PASS by ${led.primary.by}`;
  const runs = JSON.parse(gh('api', `repos/${REPO}/commits/${head}/check-runs?per_page=100`, '--jq', '{total: .total_count, runs: [.check_runs[] | {id, name, conclusion, started_at}]}'));
  const ci = checkRuns(runs.runs, runs.total);
  if (!ci.ok) return ci;
  // The secret scan can only REFUSE; a clean scan waives nothing. Every land needs the ledger PASS above and the reviewer's verdict below.
  const scan = rangeScan(mainTip, head);
  if (!scan.ok) return scan;
  if (led.path === 'coderabbit') { // the row only names the path: land reads (a) and (c) to (e) itself
    const cr = codeRabbitGate(rows, n, head, mainTip);
    if (!cr.ok) return cr;
    via = cr.note;
  }
  const gate = checkPodVerdict(await podMessages(Date.parse(pr.createdAt)), { pr: n, head, entries: reviewers() });
  if (!gate.ok) return gate;
  const tree = treeOf(mainTip, head);
  return { ok: true, notes: [via, gate.reason], tree, via };
}
// The tree `merge-tree --write-tree` gives for main's tip plus the head: what the decision judged and the preflight must reproduce.
const treeOf = (mainTip, head) => mergeTree((a) => out('git', a, { maxBuffer: 1 << 28 }), mainTip, head);
// Reads CI instead of the preflight. CI attaches its check runs to the commit it ran for, so no run ever proves a PR's merge result. Land builds that
// merge commit itself (the decision's tree; main's tip and the head as parents), pushes it to refs/heads/land/<n> (ci.yml runs on `land/**`) and waits
// for every required check on THAT commit. Short of a green proof it falls back to the local preflight; a land commit whose tree is not the merge
// result refuses. The only refs land ever pushes or deletes are land/<n>.
const [POLL_MS, CAP_MS] = [20 * 1000, 15 * 60 * 1000];
const oneLine = (m) => String(m).replace(/\s+/g, ' ').trim();
let MADE = null; // { remote, ref }: the land ref this run created or reused, deleted on every exit
const requiredNow = () => {
  try {
    const rules = jsonLines(gh('api', '--paginate', `repos/${REPO}/rules/branches/main?per_page=100`, '--jq', '.[]'));
    const classic = () => { try { return JSON.parse(gh('api', `repos/${REPO}/branches/main/protection/required_status_checks`)); } catch { return null; } }; // none or unreadable: no list
    return requiredChecks({ rules, protection: requiredChecks({ rules }) ? null : classic() });
  } catch { return null; }
};
// One read of the land commit: its tree as GitHub has it and its check runs. GET only. A failed read is no proof, not an error.
function ciRead(sha) {
  try {
    const ciTree = gh('api', `repos/${REPO}/git/commits/${sha}`, '--jq', '.tree.sha');
    const r = JSON.parse(gh('api', `repos/${REPO}/commits/${sha}/check-runs?per_page=100`, '--jq', '{total: .total_count, runs: [.check_runs[] | {name, status, conclusion, app: .app.slug}]}'));
    return { ciTree, runs: r.runs, total: r.total };
  } catch (e) { return { error: `CI could not be read: ${why(e)}` }; }
}
// Polls every POLL_MS up to CAP_MS (counted in sleeps, not by the clock). A failure, a cancel, a bad tree or the cap ends it.
async function ciWait(sha, tree, required) {
  const [poll, cap] = [IO.pollMs ?? POLL_MS, IO.capMs ?? CAP_MS];
  for (let waited = 0; ; waited += poll) {
    const got = ciRead(sha); const p = got.error ? { ok: false, kind: 'unread', reason: got.error } : ciProof({ required, tree, ciTree: got.ciTree, runs: got.runs, total: got.total });
    if (p.ok || p.kind !== 'wait') return p;
    if (waited >= cap) return { ok: false, kind: 'cap', reason: `CI did not finish in ${Math.round(cap / 60000)} minutes (${p.reason})` };
    await IO.sleep(poll);
  }
}
// The CI path: { ok, note } or { ok: false, reason } (then the caller runs the preflight). Throws a Refusal if the land commit is not the merge result.
async function ciLand(n, mainTip, head, tree) {
  const required = requiredNow();
  if (!required) return { ok: false, reason: 'the required checks could not be read from the branch rules' };
  const ref = landRef(n);
  let sha; try { sha = out('git', ['commit-tree', tree, '-p', mainTip, '-p', head, '-m', `land #${n}`]); } catch (e) { return { ok: false, reason: `could not build the land commit: ${why(e)}` }; }
  if (!/^[0-9a-f]{40}$/.test(sha) || out('git', ['rev-parse', `${sha}^{tree}`]) !== tree) refuse(`#${n}: the land commit is not the merge result (tree ${tree.slice(0, 8)}); nothing merged`);
  if (DRY) { say(`dry run: would push ${sha.slice(0, 8)} (tree ${tree.slice(0, 8)}) to ${ref} and wait up to ${Math.round((IO.capMs ?? CAP_MS) / 60000)} minutes for ${required.length} required checks`); return { ok: false, reason: 'dry run: nothing is pushed' }; }
  try {
    const remote = remoteFor(out('git', ['remote', '-v']), REPO).remote;
    const there = out('git', ['ls-remote', remote, ref]).split(/\s+/)[0];
    if (there && !/^[0-9a-f]{40}$/.test(there)) return { ok: false, reason: `${ref} could not be read` };
    // A leftover is reused only when it IS the land commit: the merge tree AND exactly main's tip then the head as parents (dco reads the parents); else overwritten under a lease on it
    const reuse = !!there && gh('api', `repos/${REPO}/git/commits/${there}`, '--jq', '([.tree.sha] + [.parents[].sha]) | join(" ")') === [tree, mainTip, head].join(' ');
    if (reuse) sha = there; else out('git', ['push', '--quiet', `--force-with-lease=${ref}:${there}`, remote, `${sha}:${ref}`]);
    MADE = { remote, ref }; process.on('exit', ciClean); // Ctrl-C and kill end through process.exit (see the CLI), so the ref goes on signals too; a signal runs only after this block, so none lands between the push and MADE
  } catch (e) { return { ok: false, reason: `could not push ${ref}: ${why(e)}` }; }
  say(`#${n}: pushed ${sha.slice(0, 8)} to ${ref}; waiting for ${required.length} required checks`);
  const p = await ciWait(sha, tree, required);
  if (p.kind === 'tree') refuse(`#${n}: ${p.reason} (nothing merged)`);
  return p;
}
// Deletes the land ref this run made, and only that: a ref under refs/heads/land/. A failure is said and does not stop the land.
function ciClean() {
  process.off('exit', ciClean); const made = MADE;
  MADE = null; if (!made) return;
  try {
    if (!/^refs\/heads\/land\/\d+$/.test(made.ref)) throw new Error('not a land ref');
    out('git', ['push', '--quiet', made.remote, '--delete', made.ref]);
  } catch (e) { console.error(`land: could not delete ${made.ref}: ${why(e)}`); }
}
const FLAGS = ['--dry-run', '--bootstrap', '--preflight-only'];
const USAGE = 'usage: land.mjs <pr> [--dry-run] [--bootstrap] | <pr> --preflight-only [--bootstrap]';
// The checkout this runs from, checked before the first fetch (mainTip null) and again after it: a clean tree at main's tip (only --bootstrap may sit elsewhere), no credentials in its git config.
const ok = (cmd, a) => { try { out(cmd, a); return true; } catch { return false; } }; // did the command exit 0
function checkoutGate(mainTip, bootstrap, prHead = null) {
  const config = (() => { try { return out('git', ['config', '--get-regexp', 'extraheader']); } catch { return ''; } })();
  const c = checkCheckout({ head: out('git', ['rev-parse', 'HEAD']), mainTip, prHead, porcelain: out('git', ['-c', 'status.showUntrackedFiles=all', 'status', '--porcelain']), remoteV: out('git', ['remote', '-v']), config, lsv: out('git', ['ls-files', '-v']), diffDirty: !ok('git', ['diff', '--quiet', 'HEAD']), bootstrap });
  if (!c.ok) refuse(c.reason);
  const b = checkNoBunConfig([...fs.readdirSync(out('git', ['rev-parse', '--show-toplevel'])), ...fs.readdirSync(IO.cwd)]); // the filesystem, not porcelain: ignored files count
  if (!b.ok) refuse(b.reason);
}
// Verifiers: the trust gate and the merge-result preflight, nothing else. No ledger, no pod, no merge.
async function preflightOnly(n, bootstrap, hold) {
  hold(lock());
  const pr = viewPr(n);
  trustGate(pr);
  const mainTip = fetchRef(MAIN_REF);
  checkoutGate(mainTip, bootstrap, pr.headRefOid);
  const h = checkHead(pr, fetchRef(`pull/${n}/head`));
  if (!h.ok) refuse(h.reason);
  bootstrapGate(mainTip, bootstrap);
  const pf = await preflight(mainTip, pr.headRefOid.toLowerCase(), bootstrap, treeOf(mainTip, pr.headRefOid.toLowerCase()));
  if (!pf.ok) refuse(`#${n}: ${pf.reason}`);
  say(`#${n}: merge result preflight green (main ${mainTip.slice(0, 8)} + ${pr.headRefOid.slice(0, 8)})`);
}
export async function main(deps) {
  IO = deps;
  REPO = IO.env.JM_REPO;
  SECRET = '';
  MADE = null;
  const args = IO.argv;
  DRY = args.includes('--dry-run');
  let release = () => {};
  try { await run(args, (r) => { release = r; }); } catch (e) { throw e instanceof Refusal ? e : new Refusal(stripControls(scrub(String(e.message).split('\n')[0], SECRET))); } finally { release(); }
}
async function run(args, hold) {
  const nv = checkNode(IO.nodeVersion);
  if (!nv.ok) refuse(nv.reason);
  const only = args.includes('--preflight-only');
  const missing = (only ? ['JM_REPO', 'JM_TRUSTED_LOGINS'] : REQUIRED).find((v) => !IO.env[v]);
  if (missing) refuse(`environment variable ${missing} is not set`);
  if (!only) reviewers();
  const api = only ? { ok: true } : checkApi(IO.env.JM_COMMONLY_API, IO.env.JM_ALLOW_API_ORIGIN);
  if (!api.ok) refuse(api.reason);
  const nums = args.filter((a) => /^\d+$/.test(a));
  if (nums.length !== 1 || args.some((a) => !/^\d+$/.test(a) && !FLAGS.includes(a)) || (only && args.some((a) => a === '--dry-run'))) refuse(USAGE);
  checkoutGate(null, args.includes('--bootstrap'));
  if (only) return preflightOnly(nums[0], args.includes('--bootstrap'), hold);
  const n = nums[0];
  const opts = { bootstrap: args.includes('--bootstrap') };
  hold(lock());
  const pr = viewPr(n);
  const action = landAction(pr, readLedger());
  if (action === 'already-landed') return say(`#${n} already landed`);
  if (action === 'record-only') {
    console.error(`land: warning: #${n} was merged without a LANDED row; ${DRY ? 'dry run, not recording it' : 'recording it as MERGED-UNVERIFIED'}`);
    return appendLedger(n, pr.headRefOid.toLowerCase(), 'MERGED-UNVERIFIED', 'land', 'merged out of band; recorded without any check');
  }
  trustGate(pr);
  const mainTip = fetchRef(MAIN_REF);
  checkoutGate(mainTip, opts.bootstrap, pr.headRefOid);
  const h = checkHead(pr, fetchRef(`pull/${n}/head`));
  if (!h.ok) refuse(h.reason);
  bootstrapGate(mainTip, opts.bootstrap);
  const head = pr.headRefOid.toLowerCase();
  const d = await decide(pr, mainTip);
  if (!d.ok) refuse(`#${n}: ${d.reason} (nothing merged)`);
  try { // the land ref (if any) is deleted on every exit below
    const proof = opts.bootstrap ? { ok: false, reason: 'the bootstrap land is judged by people' } : await ciLand(n, mainTip, head, d.tree); // the bootstrap land never uses CI
    if (!proof.ok) console.error(`land: no CI proof (${oneLine(proof.reason)}); running the local preflight`);
    const pf = proof.ok ? { ok: true, tree: d.tree } : await preflight(mainTip, head, opts.bootstrap, d.tree);
    if (!pf.ok) refuse(`#${n}: ${pf.reason} (nothing merged)`);
    const route = proof.ok ? proof.note : `local preflight (${oneLine(proof.reason)})`;
    say(`#${n}: ok, ${[...d.notes, proof.ok ? route : `${route}: merge result preflight green`].join('; ')}`);
    if (mergeAction({ dryRun: DRY }) === 'stop') return say(`dry run: would land #${n} (merge commit, main ${mainTip.slice(0, 8)}); merged nothing`);
    const again = await decide(pr, mainTip);
    if (!again.ok) refuse(`#${n}: ${again.reason} (changed before the merge; nothing merged)`);
    const fresh = viewPr(n); // --match-head-commit pins only the head: the trust gate, base and state are re-read too
    trustGate(fresh);
    const now = checkHead(fresh, head);
    if (!now.ok) refuse(`${now.reason} (changed before the merge; nothing merged)`);
    const moved = checkMain(mainTip, fetchRef(MAIN_REF)); // last look at main, right before the merge
    if (!moved.ok) refuse(moved.reason);
    gh('pr', 'merge', n, '--repo', REPO, '--merge', '--match-head-commit', head);
    const sha = JSON.parse(gh('pr', 'view', n, '--repo', REPO, '--json', 'mergeCommit')).mergeCommit?.oid ?? 'unknown';
    const merged = sha === 'unknown' ? 'unknown' : (() => { try { fetchRef(sha); return out('git', ['rev-parse', `${sha}^{tree}`]); } catch { return 'unreadable'; } })();
    const note = landedNote(sha, pf.tree, merged, `${d.via}; ${route}`);
    if (pf.tree !== merged) console.error(`land: WARNING: #${n} merged as ${sha} but its tree is not the tested merge result (${pf.tree} tested, ${merged} merged); the base or main moved in the last window. It cannot be undone; look at it now.`);
    appendLedger(n, head, 'LANDED', 'land', note);
    if (opts.bootstrap) appendLedger(n, head, 'BOOTSTRAP', 'land', 'first land of the gate, judged by people');
    say(`#${n} landed as ${sha}; branch ${stripControls(pr.headRefName)} is now safe to delete (not deleted)`);
  } finally { ciClean(); }
}
// The real run seam: status, stdout and stderr of a finished command, on success as well as failure. status is null when the command was killed
// by a signal or never started. The range scan is read from this stderr, so it must not be dropped on a zero exit.
export const realRun = (cmd, args, opts = {}) => {
  const r = spawnSync(cmd, args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'], maxBuffer: 1 << 26, ...opts });
  return { status: r.status, stdout: r.stdout ?? '', stderr: r.stderr ?? '' };
};
// The CLI: the real I/O. Ctrl-C and kill run the exit handlers that kill the preflight's process group and release the lock.
if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  process.on('SIGINT', () => process.exit(130));
  process.on('SIGTERM', () => process.exit(143));
  process.on('SIGHUP', () => process.exit(129));
  const exec = (cmd, args, opts = {}) => execFileSync(cmd, args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'], ...opts }).trim();
  main({ exec, run: realRun, fetch, spawn, kill: process.kill, sleep: (ms) => new Promise((r) => setTimeout(r, ms)), env: process.env, argv: process.argv.slice(2), nodeVersion: process.versions.node, cwd: '.' })
    .catch((e) => { console.error(`land: refused: ${e.message}`); process.exit(1); });
}
