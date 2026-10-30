// The deny check: tracked files must not carry anything private (operator paths, personal emails, internal ids, cloud
// identifiers, internal note labels, citations of unpublished documents). It holds PATTERNS only: a literal private
// value here would leak what it guards, and every case below uses a synthetic sample. Names, handles, project ids,
// project numbers, titles of named private documents and other values with no safe pattern belong in the names list instead: an operator supplies it as a
// regex in JM_PRIVATE_NAMES (CI passes a repository secret; it is empty on fork pull requests, and the run then says the
// class was skipped). The check never prints matched text, only `file:line: class`, because a CI log masks a secret only
// as a whole string. What the check is for and how to run it: CONTRIBUTING.md, "The deny check".
// Run: bun scripts/check-private.mjs  (cases first, then every tracked text file).
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';

// Allow-lists, each with a reason and a clean case below.
const EMAIL_OK = /@(users\.noreply\.github\.com|example\.(com|org|net)|[a-z0-9-]+\.(example|invalid))$/i; // GitHub noreply; RFC 2606 reserved domains only
const PACKAGE_VERSION = /@\d+\.\d+\.\d+[-+.]/; // a package@x.y.z spec (bun's patchedDependencies key and patch file name), not an address
// A remote's userinfo is allowed only as the exact synthetic values the tests use: `git@` (the scp form), `tok@` and `u:p@` in a URL.
// Any other userinfo may be a credential or a personal address, so it stays a hit.
const SYNTHETIC_USERINFO = new Map([['git', ''], ['tok', '://'], ['p', '://u:']]); // user -> what must precede it
const GIT_REMOTE = (m, line, at) => { const [user, host] = m.toLowerCase().split('@'); return host === 'github.com' && line.slice(0, at).endsWith(SYNTHETIC_USERINFO.get(user) ?? '\0'); };
const HEX24_OK = /^(0{24}|deadbeef[0-9a-f]{16})$/; // all zeros, or the documented test prefix `deadbeef`
const PUBLIC_STANDARD = /(?:\bRFC ?\d+|\bOAuth(?: 2\.\d)?(?: spec)?)\s*$/; // a section mark is allowed after a public standard named in the same line
// Machine-generated or vendored files are not scanned here (a hash can look like an id, a licence header carries a
// third party's email). gitleaks still scans every one of them.
const SKIPPED = /^(?:\.agents\/|go\.sum$|bun\.lock$|package-lock\.json$|pnpm-lock\.yaml$)|\.lock$/;

const CLASSES = [
  { name: 'operator path', re: /(?:\/Users\/[A-Za-z]|(?<!\w)~\/[A-Za-z.]|\/private\/tmp\b|(?<![\w:.])\/home\/[a-z][\w.-]*\/\w)/g },
  { name: 'email', re: /[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}\b/g, ok: (m, line, at) => EMAIL_OK.test(m) || PACKAGE_VERSION.test(m) || GIT_REMOTE(m, line, at) },
  { name: '24-hex id', re: /\b[0-9a-f]{24}\b/g, ok: (m) => HEX24_OK.test(m) },
  { name: 'registry host', re: /\b[\w-]+(?:-docker\.pkg\.dev|\.gcr\.io|\.dkr\.ecr\.[\w-]+\.amazonaws\.com|\.azurecr\.io)\b/g },
  { name: 'cloud project id', re: /\bproject(?:[_ -]?id)?["'=: ]+[a-z][a-z0-9]*(?:-[a-z0-9]+)*-\d{6,}\b/gi },
  { name: 'cloud account id', re: /(?:account[_ -]?id["'=: ]+[0-9a-f]{32}\b|arn:aws:[a-z0-9-]*:[\w-]*:\d{12}\b)/gi },
  { name: 'ledger id', re: /\(D\d{3}[),;]|(?<![\w#+/])D\d{3}:/g },
  { name: 'task label', re: /\bTASK-\d+\b/g }, // no safe narrowing: a public tracker can use the same shape, so such a hit is reworded
  { name: 'memo reference', re: /\bmemo [A-HJ-Z]\b/g }, // the lettered form only; a named document goes in the names list,
  { name: 'section mark', re: /§\s*\d/g, ok: (m, line, at) => PUBLIC_STANDARD.test(line.slice(0, at)) },
];

const namesFrom = (env) => (env.JM_PRIVATE_NAMES ? { name: 'private name', re: new RegExp(env.JM_PRIVATE_NAMES, 'gi') } : null); // throws on an invalid pattern; callers never print the error, which quotes the list

function scan(text, classes) {
  const hits = [];
  text.split('\n').forEach((line, i) => {
    for (const c of classes) for (const m of line.matchAll(c.re)) if (!c.ok?.(m[0], line, m.index)) hits.push({ cls: c.name, line: i + 1, text: m[0] });
  });
  return hits;
}

const j = (...p) => p.join(''); // keeps sample strings out of this file's own text
const SYN = namesFrom({ JM_PRIVATE_NAMES: j('zorb', 'lax') }); // a synthetic name supplied by the test
// [name, text, wanted class or null for clean, optional classes]
const CASES = [
  // one hit per alternative
  ['path: /Users/', j('/Users', '/someone/x'), 'operator path'],
  ['path: ~/ dot dir', j('~/', '.cfgdir/state'), 'operator path'],
  ['path: ~/ plain dir', j('~/', 'Documents/x'), 'operator path'],
  ['path: shared temp', j('/private', '/tmp/x'), 'operator path'],
  ['path: /home/<user>/', j('/home', '/someone/work'), 'operator path'],
  ['email: personal address', j('someone@', 'gmail.com'), 'email'],
  ['24-hex id', j('pod ', 'a1b2c3d4e5f6', 'a7b8c9d0e1f2'), '24-hex id'],
  ['registry: pkg.dev', j('us-docker', '.pkg.dev/p/r'), 'registry host'],
  ['registry: gcr.io', j('eu', '.gcr.io/p/r'), 'registry host'],
  ['registry: ECR', j('123.dkr', '.ecr.us-east-1.amazonaws.com/r'), 'registry host'],
  ['registry: ACR', j('acme', '.azurecr.io/r'), 'registry host'],
  ['project id', j('project_id', ': acme-prod-', '123456'), 'cloud project id'],
  ['account id: 32 hex', j('account_id', '=', '0123456789abcdef0123456789abcdef'), 'cloud account id'],
  ['account id: AWS ARN', j('arn:aws:iam::', '123456789012', ':root'), 'cloud account id'],
  ['ledger id: parenthesised', j('shipped (D', '999).'), 'ledger id'],
  ['ledger id: with colon', j('D', '999: decided'), 'ledger id'],
  ['task label (kept: no safe narrowing)', j('Fixes TASK', '-12 upstream'), 'task label'],
  ['memo: lettered', j('memo', ' Q'), 'memo reference'],
  ['section mark after spec', j('spec ', '§', '9'), 'section mark'],
  ['section mark, bare', j('the ', '§', '9.1 envelope'), 'section mark'],
  ['names class hit', j('hello ', 'Zorblax'), 'private name', [SYN]],
  // one clean case per allow-list entry
  ['allowed: GitHub noreply address', j('1+x@users.', 'noreply.github.com'), null],
  ['allowed: userinfo in a test URL', j('https://tok@', 'github.com/o/r'), null],
  ['allowed: user and password in a remote URL', j('ssh://u:p@', 'github.com/o/r'), null],
  ['allowed: scp-form remote', j('git@', 'github.com:o/r.git'), null],
  ['email: a password in remote userinfo', j('https://', 'someone:secretpw@', 'github.com/o/r'), 'email'],
  ['email: a password in ssh remote userinfo', j('ssh://', 'someone:secretpw@', 'github.com/o/r'), 'email'],
  ['email: a percent-encoded address in remote userinfo', j('https://', 'someone%40gmail.com@', 'github.com/o/r'), 'email'],
  ['email: userinfo that is not the synthetic value', j('https://u:', 'q@', 'github.com/o/r'), 'email'],
  ['email: the synthetic user outside a URL', j('mail ', 'tok@', 'github.com'), 'email'],
  ['email: userinfo at another host is not allowed', j('https://tok@', 'gmail.com/o/r'), 'email'],
  ['email: a github.com address that is not a remote', j('someone@', 'github.com'), 'email'],
  ['allowed: reserved example domain', j('a@', 'example.org'), null],
  ['allowed: a reserved .example domain', j('http://localhost:80@', 'attacker.example'), null],
  ['allowed: a reserved .invalid domain', j('a@', 'host.invalid'), null],
  ['email: a real registrable domain is not synthetic', j('http://localhost:80@', 'evil.com'), 'email'],
  ['allowed: a package@x.y.z patch file name', j('worker/patches/miniflare', '@5.20261012.0-beta.patch'), null],
  ['email: a numeric first label is still an address', j('someone@', '163.com'), 'email'],
  ['email: two numeric labels are not a version', j('someone@', '1.2.example-mail.com'), 'email'],
  ['allowed: all-zero 24-hex id', '0'.repeat(24), null],
  ['allowed: documented test prefix', j('deadbeef', '0123456789abcdef'), null],
  ['allowed: RFC section', j('RFC 6749 ', '§', '4.1'), null],
  ['allowed: OAuth spec section', j('OAuth spec ', '§', '4.1'), null],
  // near misses stay clean
  ['clean: 12 hex', 'abcdef012345', null],
  ['clean: 24 uppercase hex', 'A1B2C3D4E5F6A7B8C9D0E1F2', null],
  ['clean: CSS colour', 'color: #D000;', null],
  ['clean: a camera model', 'Nikon D850', null],
  ['clean: a lowercase d and digits', j('d', '999: x'), null],
  ['clean: base64', 'x+D000/y', null],
  ['clean: ledger shape after #', '#D000: x', null],
  ['clean: ledger shape after +', 'x+D000: y', null],
  ['clean: ledger shape after a word', 'XD000: y', null],
  ['clean: ~/ after a word', j('a~', '/b'), null],
  ['clean: /home after a colon', j('x:/ho', 'me/u/y'), null],
  ['clean: /home after a dot', j('v1./ho', 'me/u/y'), null],
  ['clean: React.memo', 'React.memo a component', null],
  ['clean: first-person memo', 'In this memo I argue', null],
  ['clean: a web route', j('GET /ho', 'me/feed'), null],
  ['clean: a build spec', 'the Docker build spec', null],
  ['clean: a project name', 'project: my-app-2024', null],
  ['clean: a role word', 'ask your CTO', null],
  ['clean: ordinary prose', 'The pod core is pure; the ledger is the land script\'s verifier record.', null],
  ['clean: names class off', j('hello ', 'Zorblax'), null],
];

function failedCases(cases, classes) {
  const bad = [];
  for (const [name, text, want, own] of cases) {
    const got = scan(text, own ?? classes);
    const ok = want === null ? got.length === 0 : got.length > 0 && got.every((h) => h.cls === want);
    // label and classes only: the sample text and the matched text are never printed
    if (!ok) bad.push(`case failed: ${name} (wanted ${want ?? 'clean'}, got ${[...new Set(got.map((h) => h.cls))].join(', ') || 'clean'})`);
  }
  return bad;
}

// Every problem as a message, `file:line: class` for a tree hit; no cases means the check ran nothing, which is a problem too.
function run({ cases, files, classes }) {
  const names = classes.filter((c) => c.name === 'private name');
  // The self-test cases never see the operator's names list, so a list entry that matches a public sample cannot label a case in a log.
  const problems = cases.length === 0 ? ['no cases ran'] : failedCases(cases, classes.filter((c) => !names.includes(c)));
  files.forEach(([f, text], i) => {
    // THE one place a file is named in output: a path that holds a name is given by its position, never printed.
    const ref = scan(f, names).length > 0 ? `tracked file ${i + 1} (path withheld)` : f;
    if (ref !== f) problems.push(`${ref}: path: names`);
    if (!SKIPPED.test(f)) for (const h of scan(text, classes)) problems.push(`${ref}:${h.line}: ${h.cls}`);
  });
  return problems;
}

// The files named on the command line, else every tracked file. Every path is kept, so the path scan sees binary,
// NUL-containing, unreadable and dangling-symlink entries too; such a file carries empty text, which has no content to scan.
function readText(paths = execFileSync('git', ['ls-files', '-z'], { encoding: 'utf8', maxBuffer: 1 << 28 }).split('\0').filter(Boolean), read = fs.readFileSync) {
  return paths.map((f) => {
    let buf;
    try { buf = read(f); } catch { return [f, '']; }
    return [f, buf.includes(0) ? '' : buf.toString('utf8')];
  });
}

// Returns the exit code. Everything it reads or prints is injectable, so META can drive it over a fixture.
function main({ env = process.env, files, meta, cases = CASES, log = console.log, error = console.error } = {}) {
  let names;
  try { names = namesFrom(env); } catch { error('JM_PRIVATE_NAMES is not a valid pattern'); return 1; }
  const classes = names ? [...CLASSES, names] : CLASSES;
  const tree = files ?? readText(process.argv.length > 2 ? process.argv.slice(2) : undefined);
  const unread = !files && process.argv.length <= 2 && !tree.some(([f]) => f === 'scripts/check-private.mjs') ? ['the tree read did not include this script'] : [];
  const checks = meta ?? runnerChecks();
  const metaBad = checks.filter(([, ok]) => !ok).map(([n]) => `runner check failed: ${n}`);
  const problems = [...metaBad, ...(tree.length === 0 ? ['no tracked files were read'] : []), ...unread, ...run({ cases, files: tree, classes })];
  if (problems.length > 0) { error(`${problems.join('\n')}\ncheck-private: ${problems.length} problem(s); fix the text, or extend a pattern or allow-list with a reason`); return 1; }
  log(`check-private: ${cases.length + checks.length} cases ok; ${tree.length} tracked files clean${names ? '' : ' (names class skipped: JM_PRIVATE_NAMES unset)'}`);
  return 0;
}

const HIT = j('TASK', '-5');
// [path, skipped?]: pins SKIPPED; a widened pattern (a docs or source directory) flips a negative.
const SKIP_PROBES = [['bun.lock', true], ['go.sum', true], ['package-lock.json', true], ['pnpm-lock.yaml', true], ['.agents/skills/x.md', true], ['a/b.lock', true],
  ['docs/a.md', false], ['docs/adr/0004.md', false], ['scripts/a.mjs', false], ['podcore/src/a.ts', false], ['ee/README.md', false], ['daemon/go.sum', false], ['.agentsx/a.md', false], ['a.locked', false], ['sub/package-lock.json', false]];
const PLAIN = [['a.md', 'plain']];
const NAMES = { JM_PRIVATE_NAMES: j('zorb', 'lax') };
const quiet = { log() {}, error() {}, meta: [] };
const lines = (opts) => { const out = []; const code = main({ ...quiet, ...opts, log: (m) => out.push(m), error: (m) => out.push(m) }); return { code, out }; };
function runnerChecks() { // the runner and main() themselves
  const R = (cases, files) => run({ cases, files, classes: CLASSES });
  const clean = [['x', 'plain', null]];
  const secret = lines({ env: NAMES, files: [['a.md', j('hello ', 'Zorblax')]] });
  const leaks = (f, text) => { const out = lines({ env: NAMES, files: [[f, text]] }); return out.code !== 0 && out.out.length > 0 && !out.out.some((l) => /zorblax/i.test(l)); };
  const badPattern = lines({ env: { JM_PRIVATE_NAMES: j('(zorb', 'lax') }, files: PLAIN });
  const nulTree = readText([j('docs/zorb', 'lax-notes.bin'), 'ok.bin', 'gone.md'], (f) => { if (f === 'gone.md') throw new Error('unreadable'); return Buffer.from([0, 1, 2]); });
  const nulRun = lines({ env: NAMES, files: nulTree });
  const pathRun = lines({ env: NAMES, files: [[j('docs/zorb', 'lax-notes.md'), 'plain']] });
  const secretCase = R([['lbl', j('hello ', 'Zorblax'), 'email']], []).concat(R([['lbl', j('a@', 'gmail.com'), null]], []));
  return [
    ['a wrong wanted class fails', R([['x', HIT, 'email']], []).length > 0],
    ['a wanted hit that is clean fails', R([['x', 'plain', 'email']], []).length > 0],
    ['a clean case that hits fails', R([['x', HIT, null]], []).length > 0],
    ['a planted tree hit fails', R(clean, [['a.md', HIT]]).length > 0],
    ['no cases fails', R([], []).length > 0],
    ['good input passes', R(clean, PLAIN).length === 0],
    ['vendored and generated files are skipped', ['bun.lock', 'go.sum', 'package-lock.json', 'pnpm-lock.yaml', '.agents/skills/x.md', 'a/b.lock'].every((f) => R(clean, [[f, HIT]]).length === 0)],
    ['a hit in a docs file still fires', R(clean, [['docs/a.md', HIT], ['README.md', HIT], ['CONTRIBUTING.md', HIT]]).length === 3],
    ['the skip list is exactly the vendored set and nothing wider', SKIP_PROBES.every(([f, skipped]) => SKIPPED.test(f) === skipped)],
    ['a source file is not skipped', R(clean, [['scripts/a.mjs', HIT]]).length === 1],
    ['the names class is built only when the list is supplied', namesFrom({}) === null && namesFrom({ JM_PRIVATE_NAMES: 'q' }) !== null],
    ['main passes on a clean tree', lines({ files: PLAIN, cases: [['x', 'plain', null]] }).code === 0],
    ['main fails on one planted hit', lines({ files: [...PLAIN, ['b.md', HIT], ['c.md', 'plain']], cases: [['x', 'plain', null]] }).code !== 0],
    ['main fails on one hit among many clean files', lines({ files: [...Array.from({ length: 50 }, (_, i) => [`f${i}.md`, 'plain']), ['z.md', HIT]], cases: [['x', 'plain', null]] }).code !== 0],
    ['main fails when a runner check fails', lines({ files: PLAIN, cases: [['x', 'plain', null]], meta: [['x', false]] }).code !== 0],
    ['main fails on an empty file list', lines({ files: [], cases: [['x', 'plain', null]] }).code !== 0],
    ['a tree hit prints file:line: class and not the matched name', secret.code !== 0 && secret.out.some((l) => l.includes('a.md:1: private name')) && !secret.out.some((l) => /zorblax/i.test(l))],
    ['a tracked path holding a name fails and is not printed', pathRun.code !== 0 && pathRun.out.some((l) => l.includes('path: names')) && !pathRun.out.some((l) => /zorblax/i.test(l))],
    ['a binary or unreadable entry is kept for the path scan', nulTree.length === 3 && nulTree.every(([, t]) => t === '')],
    ['a name-bearing path whose content has a NUL byte fails, path withheld', nulRun.code !== 0 && nulRun.out.some((l) => l.includes('tracked file 1 (path withheld): path: names')) && !nulRun.out.some((l) => /zorblax|lax-notes/i.test(l))],
    ['a name-bearing path with a content hit never prints the name (task label)', leaks(j('notes/zorb', 'lax-notes.md'), HIT)],
    ['a name-bearing path with a content hit never prints the name (the person mentioned)', leaks(j('notes/zorb', 'lax.md'), j('met ', 'Zorblax today'))],
    ['a name-bearing directory with a content hit never prints the name (personal email)', leaks(j('zorb', 'lax/a.md'), j('a@', 'gmail.com'))],
    ['an invalid names pattern prints a fixed message and not the pattern', badPattern.code === 1 && badPattern.out.join('\n') === 'JM_PRIVATE_NAMES is not a valid pattern'],
    ['a names entry matching a public sample does not label a self-test case', lines({ env: { JM_PRIVATE_NAMES: 'plainsample' }, files: [['a.md', 'x']], cases: [['lbl', 'plainsample', null]] }).code === 0],
    ['a clean path passes the names class', lines({ env: NAMES, files: PLAIN, cases: [['x', 'plain', null]] }).code === 0],
    ['a failed case prints no sample text', secretCase.length === 2 && !secretCase.some((l) => /zorblax|gmail/i.test(l))],
  ];
}
if (import.meta.main) process.exit(main());
