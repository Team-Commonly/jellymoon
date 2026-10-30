// Pure decisions behind scripts/land.mjs (docs/development/landing.md). Every function takes parsed data and
// returns { ok, reason } (or a plain value); the shell-outs live in land.mjs. Tests: scripts/check-land.mjs.
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import net from 'node:net';
import path from 'node:path';

const KNOWN = ['PASS', 'FAIL', 'LANDED', 'MERGED-UNVERIFIED', 'REGATE-OWED', 'BOOTSTRAP'];
const VERIFIER = /^(opus|sonnet|haiku|fable)-verifier$/;
const SEAT_NOTE = /^reviewer-gate-unavailable/i; // TEMPORARY: remove when the reviewers return
const ref = (v) => String(v).trim().replace(/^#/, '');

// Rows are normalised (trim, case). The first line is dropped only if it IS the header.
export function parseLedger(text) {
  const lines = text.split('\n').filter((l) => l.trim());
  const rows = /^pr\t/i.test(lines[0] || '') ? lines.slice(1) : lines;
  return rows.map((l) => {
    const [pr = '', head = '', verdict = '', by = '', when = '', note = ''] = l.split('\t');
    return { pr: ref(pr), head: head.trim().toLowerCase(), verdict: verdict.trim().toUpperCase(), by: by.trim().toLowerCase(), when: when.trim(), note: note.trim() };
  });
}

// The one protected-path rule. The preflight and the merge run code from the PR tree, so a path is unprotected only
// if it is inert: progress.md, or a documentation file (an extension below) under docs/. Go compiles docs/**/*.go, so
// the extension decides, not the directory. A rename counts both sides. files: { filename, previous_filename? }.
const INERT = /^(progress\.md|docs\/.+\.(md|html|svg|png|jpg|gif|webp))$/;
export const touchesProtected = (files) => files.some((f) => [f.filename, f.previous_filename].some((n) => n && !INERT.test(n)));

// One reviewer per line: `Name username` (the username is the pod's server-stamped one). A single token is a name with no username.
export function readReviewerEntries(text) {
  return text.split('\n').map((l) => l.replace(/#.*/, '').trim()).filter(Boolean).map((l) => {
    const parts = l.split(/\s+/);
    return parts.length > 1 ? { name: parts.slice(0, -1).join(' '), username: parts.at(-1) } : { name: l, username: null };
  });
}

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

// Fails closed. At (pr, head): an unrecognised verdict blocks; a verdict row by a non-verifier blocks; a FAIL
// blocks (whatever its note) unless the SAME label recorded a later PASS; the latest primary row must be a PASS.
export function checkLedger(rows, { pr, head }) {
  const mine = at(rows, pr, head);
  const odd = mine.find((r) => !KNOWN.includes(r.verdict));
  if (odd) return { ok: false, reason: `#${pr} has an unrecognised verdict '${odd.verdict}' at ${head.slice(0, 8)}` };
  const verdicts = mine.filter((r) => ['PASS', 'FAIL'].includes(r.verdict));
  const notV = verdicts.find((r) => !family(r.by));
  if (notV) return { ok: false, reason: `#${pr} has a ${notV.verdict} by '${notV.by}', which is not a <model>-verifier` };
  const failed = verdicts.find((r, i) => r.verdict === 'FAIL' && !verdicts.slice(i + 1).some((p) => p.verdict === 'PASS' && p.by === r.by));
  if (failed) return { ok: false, reason: `#${pr} has a FAIL by ${failed.by} at ${head.slice(0, 8)} with no later PASS from the same verifier` };
  const primary = verdicts.filter((r) => !SEAT_NOTE.test(r.note)).at(-1);
  if (primary?.verdict !== 'PASS') return { ok: false, reason: `no verifier PASS for #${pr} at ${head.slice(0, 8)}` };
  return { ok: true, primary };
}

// TEMPORARY (remove when the reviewers return): a second verifier from a different model family stands in
// for the pod verdict.
export function checkSecondVerifier(rows, { pr, head, primary }) {
  const row = at(rows, pr, head).filter((r) => ['PASS', 'FAIL'].includes(r.verdict) && SEAT_NOTE.test(r.note)).at(-1);
  if (!row) return { ok: false, reason: `#${pr} touches a protected path: no pod verdict, and no 'reviewer-gate-unavailable' second-verifier row` };
  if (row.verdict !== 'PASS') return { ok: false, reason: `#${pr} latest second-verifier row is ${row.verdict}` };
  if (family(row.by) === family(primary.by)) return { ok: false, reason: `#${pr} second verifier '${row.by}' is the same model family as the first` };
  return { ok: true, reason: `second verifier ${row.by}` };
}

const msgUser = (m) => m.userId?.username ?? m.username;
const msgTime = (m) => Date.parse(m.createdAt ?? m.created_at) || 0;

// A message counts for #N only if its FIRST non-empty line is exactly `jellymoon#<N> <head>` and its server-stamped
// username is a listed reviewer's. The latest such message per reviewer decides; text is never an identity.
export function checkPodVerdict(messages, { pr, head, entries, after = 0 }) {
  const byUser = new Map(entries.filter((e) => e.username).map((e) => [e.username, e.name]));
  const first = `jellymoon#${ref(pr)} ${head.toLowerCase()}`;
  const latestBy = new Map();
  messages.forEach((m, i) => {
    const name = byUser.get(msgUser(m));
    const line = (m.content || '').split('\n').map((l) => l.trim()).find(Boolean);
    if (!name || msgTime(m) < after || line?.toLowerCase() !== first) return;
    const prev = latestBy.get(name);
    if (!prev || msgTime(m) > msgTime(prev.m) || (msgTime(m) === msgTime(prev.m) && i > prev.i)) latestBy.set(name, { m, i });
  });
  const pass = [];
  const block = [];
  for (const [name, { m }] of latestBy) (verdictOf(m.content) === 'PASS' ? pass : block).push(name);
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

// PRs that still owe a reviewer re-gate. regated(pr, head, since) says a reviewer's pod PASS exists after that row.
export function owedList(rows, regated) {
  const seen = new Set();
  return rows.filter((r) => r.verdict === 'REGATE-OWED' && !seen.has(`${r.pr}@${r.head}`) && seen.add(`${r.pr}@${r.head}`)
    && !regated(r.pr, r.head, Date.parse(r.when) || 0));
}

// `git diff --name-status -M` output to the shape touchesProtected reads: renames and copies list both names.
export function filesFromNameStatus(text) {
  return text.split('\n').filter(Boolean).map((l) => {
    const [status, a, b] = l.split('\t');
    return /^[RC]/.test(status) ? { filename: b, previous_filename: a } : { filename: a };
  });
}

export const checkMain = (before, after) => (before === after ? { ok: true }
  : { ok: false, reason: `main moved from ${before.slice(0, 8)} to ${after.slice(0, 8)} during the run; run land again` });

export const checkApi = (url) => (/^https:\/\//.test(url) ? { ok: true } : { ok: false, reason: 'JM_COMMONLY_API must be https (the bearer token would travel in clear)' });

// The one git remote whose URL is the target repo (https or ssh, with or without .git), from `git remote -v`.
export function remoteFor(remoteV, repo) {
  const esc = repo.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const url = new RegExp(`^(https://([^@/]+@)?github\\.com/|git@github\\.com:|ssh://git@github\\.com/)${esc}(\\.git)?/?$`, 'i');
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

export const checkPlatform = (platform) => (platform === 'darwin' ? { ok: true }
  : { ok: false, reason: `preflight sandbox is macOS-only; add one for ${platform} before landing here` });


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

// macOS sandbox-exec profile for the preflight, which runs the PR's own code. The LAST matching rule wins. Order:
// everything readable; the real home unreadable; allow back what the preflight needs (read: allowRead and the
// writable dirs; write: only allowWrite and /dev); then the directories holding the token file and the ledger are hidden
// for good. Unix sockets are closed (docker.sock and every local daemon) except the DNS resolver's, and securityd is
// closed so /usr/bin/security cannot read the keychain. TCP stays open (tools.sh may download a toolchain) and trustd
// stays reachable (TLS needs it). Paths are resolved through symlinks first (macOS /tmp is a link).
const real = (p) => { try { return fs.realpathSync(p); } catch { return path.resolve(p); } };
const quote = (p) => `"${p.replace(/[\\"]/g, '\\$&')}"`;
export function sandboxProfile({ homes, allowRead = [], allowWrite, hide = [] }) {
  const rule = (verb, ps, kind = 'subpath') => { const s = [...new Set(ps.filter(Boolean).map(real))].map((p) => `(${kind} ${quote(p)})`); return s.length ? `(${verb} ${s.join(' ')})` : ''; };
  const hidden = hide.filter(Boolean).flatMap((f) => [path.dirname(f), path.dirname(real(f))]);
  // git stats every ancestor of the allowed paths: metadata (not listing) of those directories stays readable.
  const above = [...allowRead, ...allowWrite].filter(Boolean).flatMap((p) => { const up = []; for (let d = path.dirname(real(p)); d !== path.dirname(d); d = path.dirname(d)) up.push(d); return up; });
  return ['(version 1)', '(allow default)', rule('deny file-read*', homes), rule('allow file-read-metadata', above, 'literal'), rule('allow file-read*', [...allowRead, ...allowWrite]),
    '(deny file-write*)', rule('allow file-write*', [...allowWrite, '/dev']), rule('deny file-read* file-write*', hidden),
    '(deny network-outbound (remote unix-socket))', '(allow network-outbound (literal "/private/var/run/mDNSResponder"))',
    '(deny mach-lookup (global-name "com.apple.SecurityServer"))'].filter(Boolean).join('\n');
}

// Proves the profile before every preflight, under the exact profile (not nested). Each forbidden action must FAIL;
// a controls pair guards against a profile that fails everything. Exit codes only: no keychain secret is ever read.
export async function proveSandbox({ profile, dir, home }) {
  const where = real(path.dirname(dir));
  const sock = path.join(where, `jm-probe-${process.pid}.sock`);
  const outside = path.join(where, `jm-probe-${process.pid}.out`);
  const run = (boxed, cmd, ...args) => new Promise((done) => {
    const p = spawn(boxed ? 'sandbox-exec' : cmd, boxed ? ['-p', profile, cmd, ...args] : args, { stdio: 'ignore', cwd: dir });
    p.on('error', () => done(-1));
    p.on('exit', (code) => done(code ?? -1));
  });
  const sh = (boxed, script, arg) => run(boxed, '/bin/sh', '-c', script, 'sh', arg);
  const connect = '/usr/bin/nc -w 2 -U "$1" </dev/null';
  const server = net.createServer((c) => c.end());
  try {
    await new Promise((ok, no) => { server.once('error', no); server.listen(sock, ok); });
    if (await sh(true, 'echo x > "$1"', path.join(dir, 'probe-control')) !== 0) return { ok: false, reason: 'sandbox profile does not run: a control write to the throwaway dir failed' };
    if (await sh(false, connect, sock) !== 0) return { ok: false, reason: 'sandbox probe is broken: the test socket refused an unsandboxed connect' };
    const keychain = await run(false, '/usr/bin/security', 'default-keychain') === 0; // a host with no keychain cannot tell
    const probes = [...[].concat(home).map((h) => [`reading a file under ${h}`, () => run(true, '/bin/ls', h)]),
      ['connecting to a unix socket', () => sh(true, connect, sock)],
      ['security find-generic-password (keychain)', () => run(true, '/usr/bin/security', 'find-generic-password', '-s', 'jm-probe-no-such-item')],
      ['security default-keychain (keychain)', () => (keychain ? run(true, '/usr/bin/security', 'default-keychain') : 1)],
      ['writing outside the throwaway dir', () => sh(true, 'echo x > "$1"', outside)]];
    for (const [name, go] of probes) if (await go() === 0) return { ok: false, reason: `sandbox profile is not holding: ${name}` };
    return fs.existsSync(outside) ? { ok: false, reason: 'sandbox profile is not holding: writing outside the throwaway dir' } : { ok: true };
  } finally { server.close(); fs.rmSync(sock, { force: true }); fs.rmSync(outside, { force: true }); }
}
