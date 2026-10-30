// The deny check: tracked files must not carry anything private (operator paths, personal emails, internal ids, cloud
// identifiers, internal note labels, citations of unpublished documents). It holds PATTERNS only: a literal private
// value here would leak what it guards, and every case below uses a synthetic sample. Names, handles, project ids,
// project numbers and other values with no safe pattern belong in the names list instead: an operator supplies it as a
// regex in JM_PRIVATE_NAMES (CI passes a repository secret; it is empty on fork pull requests, and the run then says the
// class was skipped). What the check is for and how to run it: CONTRIBUTING.md, "The deny check".
// Run: bun scripts/check-private.mjs  (cases first, then every tracked text file).
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';

// Allow-lists, each with a reason and a clean case below.
const EMAIL_OK = /@(users\.noreply\.github\.com|github\.com|example\.(com|org|net)|evil\.com)$/i; // GitHub noreply; userinfo in test URLs; reserved or obviously synthetic domains
const HEX24_OK = /^(0{24}|deadbeef[0-9a-f]{16})$/; // all zeros, or the documented test prefix `deadbeef`
const PUBLIC_STANDARD = /(?:\bRFC ?\d+|\bOAuth(?: 2\.\d)?(?: spec)?)\s*$/; // a section mark is allowed after a public standard named in the same line
// Machine-generated or vendored files are not scanned here (a hash can look like an id, a licence header carries a
// third party's email). gitleaks still scans every one of them.
const SKIPPED = /^(?:\.agents\/|go\.sum$|bun\.lock$)|\.lock$/;

const CLASSES = [
  { name: 'operator path', re: /(?:\/Users\/[A-Za-z]|(?<!\w)~\/[A-Za-z.]|\/private\/tmp\b|(?<![\w:.])\/home\/[a-z][\w.-]*\/\w)/g },
  { name: 'email', re: /[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}\b/g, ok: (m) => EMAIL_OK.test(m) },
  { name: '24-hex id', re: /\b[0-9a-f]{24}\b/g, ok: (m) => HEX24_OK.test(m) },
  { name: 'registry host', re: /\b[\w-]+(?:-docker\.pkg\.dev|\.gcr\.io|\.dkr\.ecr\.[\w-]+\.amazonaws\.com|\.azurecr\.io)\b/g },
  { name: 'cloud project id', re: /\bproject(?:[_ -]?id)?["'=: ]+[a-z][a-z0-9]*(?:-[a-z0-9]+)*-\d{6,}\b/gi },
  { name: 'cloud account id', re: /(?:account[_ -]?id["'=: ]+[0-9a-f]{32}\b|arn:aws:[a-z0-9-]*:[\w-]*:\d{12}\b)/gi },
  { name: 'ledger id', re: /\(D\d{3}[),;]|(?<![\w#+/])D\d{3}:/g },
  { name: 'task label', re: /\bTASK-\d+\b/g }, // no safe narrowing: a public tracker can use the same shape, so such a hit is reworded
  { name: 'memo reference', re: /\b(?:[Ll]ocal|[Ii]nternal|[Ss]tack|[Rr]esearch) memo\b|\bmemo [A-HJ-Z]\b/g },
  { name: 'section mark', re: /§\s*\d/g, ok: (m, line, at) => PUBLIC_STANDARD.test(line.slice(0, at)) },
];

const namesFrom = (env) => (env.JM_PRIVATE_NAMES ? { name: 'private name', re: new RegExp(env.JM_PRIVATE_NAMES, 'gi') } : null);

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
  ['memo: kind word', j('stack ', 'memo'), 'memo reference'],
  ['memo: lettered', j('memo', ' Q'), 'memo reference'],
  ['section mark after spec', j('spec ', '§', '9'), 'section mark'],
  ['section mark, bare', j('the ', '§', '9.1 envelope'), 'section mark'],
  ['names class hit', j('hello ', 'Zorblax'), 'private name', [SYN]],
  // one clean case per allow-list entry
  ['allowed: GitHub noreply address', j('1+x@users.', 'noreply.github.com'), null],
  ['allowed: userinfo in a test URL', j('https://tok@', 'github.com/o/r'), null],
  ['allowed: reserved example domain', j('a@', 'example.org'), null],
  ['allowed: obviously synthetic domain', j('http://localhost:80@', 'evil.com'), null],
  ['allowed: all-zero 24-hex id', '0'.repeat(24), null],
  ['allowed: documented test prefix', j('deadbeef', '0123456789abcdef'), null],
  ['allowed: RFC section', j('RFC 6749 ', '§', '4.1'), null],
  ['allowed: OAuth spec section', j('OAuth spec ', '§', '4.1'), null],
  // near misses stay clean
  ['clean: 12 hex', 'abcdef012345', null],
  ['clean: 24 uppercase hex', 'A1B2C3D4E5F6A7B8C9D0E1F2', null],
  ['clean: CSS colour', 'color: #D923;', null],
  ['clean: a camera model', 'Nikon D850', null],
  ['clean: a lowercase d and digits', j('d', '999: x'), null],
  ['clean: base64', 'x+D923/y', null],
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
    if (!ok) bad.push(`case failed: ${name} (wanted ${want ?? 'clean'}, got ${JSON.stringify(got)})`);
  }
  return bad;
}

// Every problem as a message; no cases means the check ran nothing, which is a problem too.
function run({ cases, files, classes }) {
  const problems = cases.length === 0 ? ['no cases ran'] : failedCases(cases, classes);
  for (const [f, text] of files) {
    if (!SKIPPED.test(f)) for (const h of scan(text, classes)) problems.push(`${f}:${h.line}: ${h.cls}: ${h.text}`);
  }
  return problems;
}

const HIT = j('TASK', '-5');
const META = [ // the runner itself
  ['a wrong wanted class fails', run({ cases: [['x', HIT, 'email']], files: [], classes: CLASSES }).length > 0],
  ['a wanted hit that is clean fails', run({ cases: [['x', 'plain', 'email']], files: [], classes: CLASSES }).length > 0],
  ['a clean case that hits fails', run({ cases: [['x', HIT, null]], files: [], classes: CLASSES }).length > 0],
  ['a planted tree hit fails', run({ cases: [['x', 'plain', null]], files: [['a.md', HIT]], classes: CLASSES }).length > 0],
  ['no cases fails', run({ cases: [], files: [], classes: CLASSES }).length > 0],
  ['good input passes', run({ cases: [['x', 'plain', null]], files: [['a.md', 'plain']], classes: CLASSES }).length === 0],
  ['vendored and generated files are skipped', ['bun.lock', 'go.sum', '.agents/skills/x.md', 'a/b.lock'].every((f) => run({ cases: [['x', 'plain', null]], files: [[f, HIT]], classes: CLASSES }).length === 0)],
  ['a source file is not skipped', run({ cases: [['x', 'plain', null]], files: [['scripts/a.mjs', HIT]], classes: CLASSES }).length === 1],
  ['the names class is built only when the list is supplied', namesFrom({}) === null && namesFrom({ JM_PRIVATE_NAMES: 'q' }) !== null],
];

function trackedText() {
  const files = execFileSync('git', ['ls-files', '-z'], { encoding: 'utf8', maxBuffer: 1 << 28 }).split('\0').filter(Boolean);
  return files.flatMap((f) => {
    let buf;
    try { buf = fs.readFileSync(f); } catch { return []; } // a deleted or unreadable path is not text
    return buf.includes(0) ? [] : [[f, buf.toString('utf8')]];
  });
}

function main() {
  const names = namesFrom(process.env);
  const classes = names ? [...CLASSES, names] : CLASSES;
  const metaBad = META.filter(([, ok]) => !ok).map(([n]) => `runner check failed: ${n}`);
  const problems = [...metaBad, ...run({ cases: CASES, files: trackedText(), classes })];
  if (problems.length > 0) { console.error(`${problems.join('\n')}\ncheck-private: ${problems.length} problem(s); fix the text, or extend a pattern or allow-list with a reason`); process.exit(1); }
  console.log(`check-private: ${CASES.length + META.length} cases ok; tracked files clean${names ? '' : ' (names class skipped: JM_PRIVATE_NAMES unset)'}`);
}

if (import.meta.main) main();
