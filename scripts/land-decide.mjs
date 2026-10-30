// Pure decisions behind scripts/land.mjs (docs/development/landing.md). Every function takes parsed data and
// returns { ok, reason } (or a plain value); the shell-outs live in land.mjs. Tests: scripts/check-land.mjs.
import fs from 'node:fs';
import path from 'node:path';
const KNOWN = ['PASS', 'FAIL', 'LANDED', 'MERGED-UNVERIFIED', 'BOOTSTRAP'];
const VERIFIER = /^(opus|sonnet|haiku|fable)-verifier$/;
const ref = (v) => String(v).trim().replace(/^#/, '');
// Rows are normalised (trim, case). The first line is dropped only if it IS the header; blank and `#` lines are skipped. Any
// other line must have exactly 6 tab columns and a 40-hex head, or the whole ledger is unreadable: a row is never skipped silently.
export function parseLedger(text) {
  const lines = text.split('\n').filter((l) => l.trim() && !l.startsWith('#'));
  const rows = /^pr\t/i.test(lines[0] || '') ? lines.slice(1) : lines;
  return rows.map((l, i) => {
    const cols = l.split('\t');
    if (cols.length !== 6 || !/^#?[1-9]\d*$/.test(cols[0].trim()) || !/^[0-9a-f]{40}$/i.test(cols[1].trim())) throw new Error(`ledger is unreadable: row ${i + 1} is not 6 tab-separated columns with a plain PR number and a 40-hex head`);
    const [pr, head, verdict, by, when, note] = cols;
    return { pr: ref(pr), head: head.trim().toLowerCase(), verdict: verdict.trim().toUpperCase(), by: by.trim().toLowerCase(), when: when.trim(), note: note.trim() };
  });
}
// Accepted reviewers come from operator env JM_VERDICT_REVIEWERS, `<id>:<Name>,<id>:<Name>`, never from the repo. The id is the pod user's
// immutable id; a username is a mutable label any pod member can set on an agent they install.
export const parseReviewers = (text) => String(text || '').split(',').map((e) => e.trim()).filter(Boolean).map((e) => {
  const at = e.indexOf(':');
  return at < 1 ? {} : { id: e.slice(0, at).trim(), name: e.slice(at + 1).trim() };
}).filter((e) => e.id && e.name && e.id.indexOf(':') < 0);
// The verdict is the LAST non-empty line, HTML comments removed, emphasis and list or heading markers stripped. Only
// exactly `GATE: PASS` (case-insensitive) passes. Returns 'PASS', 'BLOCK', or null (nothing to judge). An indented line is code and blocks.
export function verdictOf(text) {
  const last = text.replace(/<!--[\s\S]*?-->/g, '').split('\n').filter((l) => l.trim()).pop();
  if (last === undefined) return null;
  if (/^(\t| {4})/.test(last)) return 'BLOCK';
  const line = last.replace(/[*_`~]/g, '').trim().replace(/^(#{1,6}|[-+]|\d+\.)\s+/, '');
  return /^GATE: PASS$/i.test(line) ? 'PASS' : 'BLOCK';
}
const family = (by) => VERIFIER.exec(by || '')?.[1] ?? null;
const at = (rows, pr, head) => rows.filter((r) => r.pr === ref(pr) && r.head === head.toLowerCase());
// Fails closed. At (pr, head): an unrecognised verdict blocks; a verdict row by a non-verifier blocks; ANY FAIL blocks,
// whoever wrote it (no verdict shopping: the fix is a new head); the latest verdict row must be a PASS.
export function checkLedger(rows, { pr, head }) {
  const mine = at(rows, pr, head);
  const odd = mine.find((r) => !KNOWN.includes(r.verdict));
  if (odd) return { ok: false, reason: `#${pr} has an unrecognised verdict '${odd.verdict}' at ${head.slice(0, 8)}` };
  const verdicts = mine.filter((r) => ['PASS', 'FAIL'].includes(r.verdict));
  const notV = verdicts.find((r) => !family(r.by));
  if (notV) return { ok: false, reason: `#${pr} has a ${notV.verdict} by '${notV.by}', which is not a <model>-verifier` };
  const failed = verdicts.find((r) => r.verdict === 'FAIL');
  if (failed) return { ok: false, reason: `#${pr} has a FAIL by ${failed.by} at ${head.slice(0, 8)}; a FAIL blocks that head, so push a new head` };
  const primary = verdicts.at(-1);
  if (primary?.verdict !== 'PASS') return { ok: false, reason: `no verifier PASS for #${pr} at ${head.slice(0, 8)}` };
  return { ok: true, primary };
}
const msgUser = (m) => (m.userId && typeof m.userId === 'object' && typeof m.userId._id === 'string' && m.userId._id ? m.userId._id : null); // a string userId is not a populated user: ignored
const msgTime = (m) => Date.parse(m.createdAt ?? m.created_at) || 0;
// A message counts for #N only if its FIRST non-empty line is exactly `jellymoon#<N> <head>` and its server-stamped
// author id is a listed reviewer's. The latest such message per reviewer id decides (messages arrive newest first, so on a
// tie the lower index is the newer one); text is never an identity.
export function checkPodVerdict(messages, { pr, head, entries }) {
  if (!entries.length) return { ok: false, reason: 'JM_VERDICT_REVIEWERS is not set; every change needs the reviewer verdict and land cannot know whose it is' };
  const byUser = new Map(entries.map((e) => [e.id, e.name]));
  const first = `jellymoon#${ref(pr)} ${head.toLowerCase()}`;
  const latestBy = new Map();
  messages.forEach((m, i) => {
    const id = msgUser(m);
    const name = byUser.get(id);
    const line = (m.content || '').split('\n').map((l) => l.trim()).find(Boolean);
    if (!name || line?.toLowerCase() !== first) return;
    const prev = latestBy.get(id);
    if (!prev || msgTime(m) > msgTime(prev.m) || (msgTime(m) === msgTime(prev.m) && i < prev.i)) latestBy.set(id, { m, i, name });
  });
  const pass = [];
  const block = [];
  for (const { m, name } of latestBy.values()) (verdictOf(m.content) === 'PASS' ? pass : block).push(name);
  if (block.length) return { ok: false, reason: `pod verdict blocked by ${block.join(', ')}` };
  if (!pass.length) return { ok: false, reason: `no accepted reviewer's pod message starts with 'jellymoon#${ref(pr)} <full head>'` };
  return { ok: true, reason: `pod verdict PASS (${pass.join(', ')})` };
}
export function checkHead(pr, local) {
  if (pr.state !== 'OPEN') return { ok: false, reason: `#${pr.number} is ${pr.state}` };
  if (pr.baseRefName !== 'main') return { ok: false, reason: `#${pr.number} is based on ${pr.baseRefName}, not main; land one PR retargeted to main` };
  if (String(local).toLowerCase() !== pr.headRefOid.toLowerCase()) return { ok: false, reason: `#${pr.number} head on GitHub ${pr.headRefOid.slice(0, 8)} differs from the local fetch ${String(local).slice(0, 8)}` };
  return { ok: true };
}
// Idempotency: what to do with this PR. MERGED-UNVERIFIED (merged out of band, recorded with no check) ends it too.
export function landAction(pr, rows) {
  if (pr.state !== 'MERGED') return 'check';
  return at(rows, pr.number, pr.headRefOid).some((r) => ['LANDED', 'MERGED-UNVERIFIED'].includes(r.verdict)) ? 'already-landed' : 'record-only';
}
export const checkMain = (before, after) => (before === after ? { ok: true }
  : { ok: false, reason: `main moved from ${before.slice(0, 8)} to ${after.slice(0, 8)} during the run; run land again` });
const API_ORIGIN = 'https://api.commonly.me';
export function checkApi(url, allow = '') {
  const u = String(url).replace(/\/$/, '');
  return u === API_ORIGIN || (allow && u === allow.replace(/\/$/, '') && /^https:\/\//.test(u)) ? { ok: true }
    : { ok: false, reason: `JM_COMMONLY_API must be ${API_ORIGIN} (or equal JM_ALLOW_API_ORIGIN, https only): the bearer token goes only to a host the operator chose` };
}
// Bun loads .env* and bunfig.toml from the cwd into unset variables (names compare case-insensitively: APFS). This guards the land
// checkout's cwd, where the operator might run bun or make by habit; a file there must never choose a variable or a host.
export const checkNoBunConfig = (names) => { const bad = names.find((n) => /^\.env/i.test(n) || /^bunfig\.toml$/i.test(n)); return bad ? { ok: false, reason: `${bad} exists in this checkout; remove it (bun would load it as configuration)` } : { ok: true }; };
// The one git remote whose URL is the target repo (https or ssh, with or without .git), from `git remote -v`.
export function remoteFor(remoteV, repo) {
  const esc = repo.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const url = new RegExp(`^(https://github\\.com/|git@github\\.com:|ssh://git@github\\.com/)${esc}(\\.git)?/?$`, 'i');
  const names = [...new Set(remoteV.split('\n').map((l) => l.split(/\s+/)).filter(([, u]) => u && url.test(u)).map(([n]) => n))];
  if (names.length === 1) return { remote: names[0] };
  return { error: names.length ? `more than one git remote matches ${repo}: ${names.join(', ')}` : `no git remote matches ${repo}` };
}
// Removes a throwaway tree. Go's module cache is read-only, so make it writable first. Never throws: a failed
// cleanup warns with the directory and the caller's result stands.
export function removeTree(dir, { rm = (d) => fs.rmSync(d, { recursive: true, force: true }), warn = console.error } = {}) {
  const writable = (d) => {
    try {
      fs.chmodSync(d, 0o700);
      for (const e of fs.readdirSync(d, { withFileTypes: true })) if (e.isDirectory() && !e.isSymbolicLink()) writable(`${d}/${e.name}`); else if (!e.isSymbolicLink()) fs.chmodSync(`${d}/${e.name}`, 0o600);
    } catch { /* best effort */ }
  };
  try { writable(dir); rm(dir); return true; } catch (e) { warn(`land: warning: could not remove ${dir}: ${e.message.split('\n')[0]}`); return false; }
}
// A bearer token is one printable token. Anything else is refused WITHOUT quoting the file: a malformed file once
// echoed its token into a fetch error.
export function readToken(file) {
  const t = fs.readFileSync(file, 'utf8').trim();
  if (!/^[\x21-\x7e]+$/.test(t)) throw new Error('token file is not a single printable token (contents not shown)');
  return t;
}
export const scrub = (text, secret) => (secret ? String(text).split(secret).join('[token]') : String(text));
// Pod paging. The next cursor is the oldest time + 1 ms so messages sharing that millisecond come back and are
// deduped by id; a full page that adds nothing new cannot be paged past, so the caller fails closed.
export function takePage(seen, msgs, limit) {
  const fresh = msgs.filter((m) => { const k = m.id ?? m._id ?? JSON.stringify(m); return !seen.has(k) && seen.add(k); });
  return { fresh, stuck: msgs.length >= limit && fresh.length === 0 };
}
export function cursorOf(msgs) {
  const ts = msgs.map(msgTime);
  return ts.some((x) => !x) ? null : new Date(Math.min(...ts) + 1).toISOString();
}
// --dry-run writes nothing: the ledger line is skipped.
export function appendLedger(file, cols, { dry = false } = {}) {
  if (dry) return false;
  const exists = fs.existsSync(file);
  const lead = exists && !fs.readFileSync(file, 'utf8').endsWith('\n') ? '\n' : '';
  fs.appendFileSync(file, `${exists ? '' : 'pr\thead\tverdict\tby\twhen\tnote\n'}${lead}${cols.join('\t')}\n`);
  return true;
}
// One land at a time per clone: an mkdir lock holding the PID. A dead owner, or a lock dir with no readable pid, is
// stale; it is taken over by renaming the stale dir to a name of our own (atomic: one taker wins), then mkdir.
export function acquireLock(dir, { pid = process.pid, alive = (p) => { try { process.kill(p, 0); return true; } catch (e) { return e.code !== 'ESRCH'; } } } = {}) {
  const take = () => { fs.mkdirSync(dir); fs.writeFileSync(path.join(dir, 'pid'), String(pid)); };
  try { take(); return { ok: true }; } catch (e) { if (e.code !== 'EEXIST') throw e; }
  let owner = NaN;
  try { owner = Number(fs.readFileSync(path.join(dir, 'pid'), 'utf8').trim()); } catch { /* no pid file: stale */ }
  if (owner && alive(owner)) return { ok: false, reason: `another land is running (pid ${owner})` };
  try { fs.renameSync(dir, `${dir}.stale-${pid}`); } catch { return { ok: false, reason: 'another land took the stale lock first' }; }
  fs.rmSync(`${dir}.stale-${pid}`, { recursive: true, force: true });
  try { take(); return { ok: true }; } catch { return { ok: false, reason: 'could not take the land lock' }; }
}
// The trust gate: this host runs only code whose authors are trusted. Trust is GitHub identity, never a commit author string.
// pr: { isCrossRepository, author }; pushers: logins with push access to the repo; trusted: JM_TRUSTED_LOGINS (operator env).
const REFUSE = 'untrusted PR: not executed on this host. Re-author it through the loop to land it.';
// Accounts that can push: `collaborators?affiliation=all` lines of {login, push}. The permission=push filter was seen returning admins only.
export const pushersOf = (text) => text.split('\n').filter(Boolean).map((l) => JSON.parse(l)).filter((c) => c.push === true).map((c) => c.login);
export const parseLogins = (text) => [...new Set(String(text || '').split(',').map((l) => l.trim().toLowerCase()).filter(Boolean))];
export function checkTrust({ pr, pushers, trusted }) {
  const no = (why) => ({ ok: false, reason: `${REFUSE} (${why})` });
  if (!trusted.length) return { ok: false, reason: 'JM_TRUSTED_LOGINS is not set; refusing to run any PR code' };
  if (pr.isCrossRepository !== false) return no('cross-repository, or not known to be same-repo');
  if (!pushers.length) return no('the push-access list is empty, so the query went wrong');
  const author = String(pr.author || '').toLowerCase();
  if (!trusted.includes(author)) return no(`author '${author || 'unknown'}' is not trusted`);
  const strangers = pushers.map((l) => String(l).toLowerCase()).filter((l) => !trusted.includes(l)); // .some-style: an empty login is a stranger too
  return strangers.length ? { ok: false, reason: `${REFUSE} (an account that can push to the repo is not trusted: ${strangers[0]})` } : { ok: true };
}
// How gitleaks v8.30.1 counts commits ("<n> commits scanned."): its git source runs `git log -p -U0 <log-opts>` and yields one fragment per
// hunk (TextFragment) of every file that is not deleted and not a non-archive binary (sources/git.go, Fragments); the detector adds a
// fragment's commit to the count when the fragment arrives, even one with no added lines (detect.go, DetectSource: addCommit before the
// empty check). So a commit counts exactly when its diff has a hunk in a file git does not show as deleted. An empty commit, a pure rename, a
// mode change, an empty file, a deleted file, and a binary (git prints no hunk for it) yield nothing. `log` is `git log -m -p -U0
// --format=%x01%H <range>` and `all` is `git rev-list <range>`. Returns the count gitleaks should print.
// The count says how many commits were read, never which: land-scan.mjs refuses the one shape where the numbers agree over different commits.
export function textCommits(log, all) {
  const read = new Set();
  let sha = '';
  let deleted = false;
  for (const l of String(log).split('\n')) { // a diff line starts with + - space or backslash, so these prefixes are headers
    if (l.startsWith('\u0001')) sha = l.slice(1);
    else if (l.startsWith('diff --git ')) deleted = false;
    else if (l.startsWith('deleted file mode ')) deleted = true;
    else if (l.startsWith('@@ ') && !deleted && sha) read.add(sha);
  }
  return { expected: all.filter((c) => read.has(c)).length };
}
// The one reading of the range scan's result, shared by land and by the fixtures that test the recipe. Clean is positive evidence, all of:
// exit status 0; no `exit status 3` line (gitleaks' --exit-code 3, which `go run` reports that way); no gitleaks ERR/FTL/PNC log line and no
// "failed to scan"; the "no leaks found" line; and exactly one "<n> commits scanned." line with n equal to `expected`, the number of commits
// gitleaks should read, which the caller derives itself with textCommits (above), outside make. A hit is a nonzero exit with the hit line.
// Everything else, including a zero exit that printed the hit line or an error (a recipe that swallowed the failure) or scanned fewer
// commits than the range holds, is a failure, and a failure refuses. gitleaks colours its log even into a pipe, so the escape codes are
// stripped before reading.
export const scanResult = ({ status, stderr, expected }) => {
  const text = String(stderr || '').replace(/\u001b\[[0-9;]*m/g, '');
  const hitLine = /^exit status 3$/m.test(text);
  if (typeof status === 'number' && status !== 0 && hitLine) return 'hit';
  const counts = [...text.matchAll(/^\S+ INF (\d+) commits? scanned\.$/gm)].map((m) => Number(m[1]));
  const errLine = /^\S+ (ERR|FTL|PNC) /m.test(text) || /failed to scan/.test(text);
  const read = counts.length === 1 && counts[0] === expected; // === also rules out an expected that is missing, NaN, negative, fractional or a string
  return status === 0 && !hitLine && !errLine && /^\S+ INF no leaks found$/m.test(text) && read ? 'clean' : 'failure';
};
// A name folded the way a case-insensitive filesystem may compare it. APFS opens `.TOOLS` for `.tools`, and also takes ſ (long s) for s, the
// Kelvin sign for k, and ß or ẞ for ss (measured by probing every BMP code point against the letters of the names below; no code point is
// ignored inside a name). NFKC and a lower, upper, lower round trip fold each of those the same way; they fold more besides, which only
// refuses more.
export const foldName = (name) => name.normalize('NFKC').toLowerCase().toUpperCase().toLowerCase();
// What a tree carries that changes what the range scan runs or reads. The scan runs in a checkout of main, so these, committed to main, change
// every later land: land checks main's tree and the tree of every commit in the range (land-scan.mjs), so a PR cannot bring one onto main.
// - a path whose FIRST component is `.tools`: land runs go from `<checkout>/.tools/bin`, so a tracked `.tools` (a symlink, or a force-added
//   `.tools/bin/go`) would choose the go land runs. Never allowed, whatever the blob.
// - a path with any component named `.gitleaks.toml` or `.gitleaksignore` (a directory counts: gitleaks only asks whether
//   `<source>/.gitleaks.toml` exists, then loads `.gitleaks.json`, `.yaml` and the rest beside it), or whose top-level entry is `.gitleaks`
//   or starts with `.gitleaks.` (a config gitleaks could discover), or a `.gitattributes` that switches files out of the diff (`-diff`,
//   `!diff`, `binary`, a `!` pattern). Allowed only when its blob id is the one this repo ships: SHIPPED maps a path to that id, and this
//   repo ships none. Land passes its own --config, so discovery does not run; this keeps every tree land checks free of what it would find.
// `tree` is `git ls-tree -r -z --full-tree <commit>`, blob(id) reads a blob, `where` names the tree in the reason. Names compare folded.
const SHIPPED = {};
const SWITCHES = new Set(['-diff', '!diff', 'binary']);
const switchesOff = (line) => { const [pat, ...attrs] = line.trim().split(/\s+/); return !pat.startsWith('#') && (pat.startsWith('!') || attrs.some((a) => SWITCHES.has(a))); };
export function checkScanConfig(tree, blob, shipped = SHIPPED, where = 'main') {
  for (const entry of String(tree).split('\0').filter(Boolean)) {
    const [meta, file] = [entry.slice(0, entry.indexOf('\t')), entry.slice(entry.indexOf('\t') + 1)];
    const id = meta.split(' ')[2];
    const parts = file.split('/').map(foldName);
    if (parts[0] === '.tools') return { ok: false, reason: `${where} carries ${file}; land runs go from the checkout's own .tools, so a tracked .tools would choose the scanner; remove it` };
    const name = parts.at(-1);
    const bad = parts.some((p) => p === '.gitleaks.toml' || p === '.gitleaksignore') || parts[0] === '.gitleaks' || parts[0].startsWith('.gitleaks.')
      || (name === '.gitattributes' && blob(id).split('\n').some((l) => l.trim() && switchesOff(l)));
    if (bad && shipped[file] !== id) return { ok: false, reason: `${where} carries ${file}, which changes what the range scan reads; remove it or ship it from this repo` };
  }
  return { ok: true };
}
// The environment the range scan runs in (git and `go run`): an allowlist, so the operator's GITLEAKS_CONFIG, GOFLAGS, GIT_DIR or git config
// cannot change what it reads. PATH, HOME and TMPDIR are kept for the toolchain and git. A tracked go.work, or a go.mod with a `toolchain`
// line, could otherwise make `go run` fetch and run another Go release (measured), so GOTOOLCHAIN is pinned to the installed one and
// GOWORK is off; the module is named with an explicit version, so no tracked go.mod, vendor directory or replace line is consulted.
const SCAN_ENV_KEEP = ['PATH', 'HOME', 'TMPDIR', 'GOPATH', 'GOCACHE', 'GOMODCACHE'];
export const scanEnv = (env) => ({
  ...Object.fromEntries(SCAN_ENV_KEEP.filter((k) => env[k] !== undefined).map((k) => [k, env[k]])),
  GOTOOLCHAIN: 'local',
  GOWORK: 'off',
  GIT_CONFIG_GLOBAL: '/dev/null',
  GIT_CONFIG_NOSYSTEM: '1',
});
// Check runs on the head are a second witness only when they exist: none at all is fine, a failed one refuses.
const BAD_CONCLUSION = ['failure', 'timed_out', 'cancelled', 'startup_failure', 'action_required'];
export function checkRuns(runs, total = runs.length) {
  if (total > runs.length) return { ok: false, reason: `${total} check runs on the head; cannot read them all` };
  const bad = runs.filter((r) => BAD_CONCLUSION.includes(r.conclusion));
  return bad.length ? { ok: false, reason: `check run failed on the head: ${bad.map((r) => `${stripControls(String(r.name))} (${r.conclusion})`).join(', ')}` } : { ok: true };
}
// The land and verify checkout: the tree is clean and HEAD is main's tip (only an explicit --bootstrap may sit elsewhere, and then only at the PR head), because
// the scripts in this tree run before the trust gate decides. mainTip null: before the fetch, HEAD is compared later. Credentials in the
// checkout's git config would travel into every fetch, so a remote URL with userinfo or an http extraheader refuses.
const USERINFO = /^https?:\/\/[^/\s]*@|^[a-z+]+:\/\/[^/@\s]*:[^/@\s]*@/i;
// porcelain is `status --porcelain` with showUntrackedFiles=all; lsv is `ls-files -v`; diffDirty is a non-empty `diff HEAD`.
export function checkCheckout({ head, mainTip, prHead = null, porcelain, remoteV, config, lsv = '', diffDirty = false, bootstrap = false }) {
  if (bootstrap && mainTip !== null && prHead && head.toLowerCase() !== prHead.toLowerCase()) return { ok: false, reason: `--bootstrap runs from a checkout of the PR head ${prHead.slice(0, 8)}, but this checkout is at ${head.slice(0, 8)}` };
  if (!bootstrap && mainTip !== null && head !== mainTip) return { ok: false, reason: `this checkout is at ${head.slice(0, 8)}, not main's tip ${mainTip.slice(0, 8)}; land from a checkout of main` };
  if (porcelain.trim()) return { ok: false, reason: 'this checkout has uncommitted or untracked files; land from a clean checkout of main' };
  if (lsv.split('\n').some((l) => /^[a-zS] /.test(l))) return { ok: false, reason: 'a tracked file is marked assume-unchanged or skip-worktree, which hides edits from git status; clear the flag' };
  if (diffDirty) return { ok: false, reason: 'this checkout differs from HEAD; land from a clean checkout of main' };
  if (remoteV.split('\n').some((l) => USERINFO.test(l.split(/\s+/)[1] || ''))) return { ok: false, reason: 'a git remote URL carries credentials (userinfo); remove them' };
  if (/^http\..*extraheader/im.test(config)) return { ok: false, reason: 'the git config carries an http extraheader (a credential); remove it' };
  return { ok: true };
}
// The tree `git merge-tree --write-tree` gives for main's tip plus the head: what GitHub's merge would produce, and what the preflight
// must reproduce. run(args) is `git` with those args. A conflict throws.
export function mergeTree(run, mainTip, head) {
  try { return run(['merge-tree', '--write-tree', mainTip, head]).split('\n')[0].trim(); } catch { throw new Error(`${head.slice(0, 8)} does not merge cleanly into main ${mainTip.slice(0, 8)}`); }
}
// PR code's output reaches the operator's terminal only through this: control characters (escape sequences, \r) are
// stripped except \n and \t, and every line is marked, so nothing it prints can pass for land's own words.
export const stripControls = (text) => text.replace(/[\u0000-\u0008\u000b-\u001f\u007f-\u009f\u061c\u200b-\u200f\u2028\u2029\u202a-\u202e\u2066-\u2069\ufeff]/g, '');
export const quoteOutput = (text) => stripControls(text).replace(/\n$/, '').split('\n').map((l) => `preflight| ${l}`).join('\n');
// The entropy ratchet's base: main's tip as fetched. Only the bootstrap land, when main has no entropy.json yet, runs without one.
export const entropyBase = (mainTip, { bootstrap, mainHasEntropy }) => (bootstrap && !mainHasEntropy ? 'none' : mainTip);

// Land's own runtime: the oldest Node whose behaviour land.mjs relies on (docs/adr/0001-dev-tooling.md).
const NODE_MIN = 20;
export const checkNode = (version, min = NODE_MIN) => (Number(String(version).split('.')[0]) >= min ? { ok: true }
  : { ok: false, reason: `land needs Node ${min} or newer; this is Node ${version}` });

// The throwaway preflight dir and its git worktree go away on every exit path: normal return, a signal (the handlers call exit), a crash.
export function trackThrowaway(dir, wt, run) {
  const git = (...a) => { try { run('git', a); } catch { /* already gone */ } };
  const clean = () => { try { run('chmod', ['-R', 'u+w', dir]); } catch { /* gone */ } git('worktree', 'remove', '--force', wt); git('worktree', 'prune'); removeTree(dir); };
  process.on('exit', clean);
  return () => { process.off('exit', clean); clean(); };
}
// A dry run decides everything and merges nothing.
export const mergeAction = ({ dryRun }) => (dryRun ? 'stop' : 'merge');
// The ledger note for a landed PR: GitHub's merge commit must have the tree that was tested. A mismatch (main or the base moved in the
// window between the last re-read and the merge) cannot be undone, but it is recorded and shouted instead of going unseen.
export const landedNote = (sha, tested, merged) => (tested === merged ? sha : `${sha} TREE-MISMATCH tested ${tested} merged ${merged}`);
