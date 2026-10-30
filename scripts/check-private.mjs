// The deny check: tracked files must not carry anything private (operator paths, personal emails, team roles,
// internal ids, cloud identifiers, internal note labels). It holds PATTERNS only: a literal private name, id or path
// here would leak what it guards. Person and chat-handle names have no pattern, so an operator supplies them as a
// regex in the JM_PRIVATE_NAMES environment variable; unset, that one class is skipped and the run says so.
// Run: bun scripts/check-private.mjs  (cases first, then every tracked text file). Design: docs/adr/0004-open-core-boundary.md.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';

// Allow-lists, each with a reason and a clean case below.
const EMAIL_OK = /@(users\.noreply\.github\.com|github\.com|example\.(com|org|net)|evil\.com)$/i; // GitHub noreply; userinfo in test URLs; reserved or obviously synthetic domains
const HEX24_OK = /^(0{24}|deadbeef[0-9a-f]{16})$/i; // all zeros, or the documented test prefix `deadbeef`

const CLASSES = [
  { name: 'operator path', re: /(?:\/Users\/[A-Za-z]|~\/\.(?:commonly|claude|codex)\b|\/private\/tmp\b|\/home\/[a-z])/g },
  { name: 'email', re: /[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}\b/g, ok: (m) => EMAIL_OK.test(m) },
  { name: 'team role', re: /\b(?:Co[f]ounder|CT[O])\b/g },
  { name: '24-hex id', re: /\b[0-9a-f]{24}\b/gi, ok: (m) => HEX24_OK.test(m) },
  { name: 'registry host', re: /\b[\w-]+(?:-docker\.pkg\.dev|\.gcr\.io|\.dkr\.ecr\.[\w-]+\.amazonaws\.com|\.azurecr\.io)\b/g },
  { name: 'cloud project id', re: /\bproject(?:[_ -]?id)?["'=: ]+[a-z][a-z0-9]*(?:-[a-z0-9]+)*-\d{4,}\b/gi },
  { name: 'cloud account id', re: /(?:account[_ -]?id["'=: ]+[0-9a-f]{32}\b|arn:aws:[a-z0-9-]*:[\w-]*:\d{12}\b)/gi },
  { name: 'ledger id', re: /\bD\d{3}\b/g },
  { name: 'task label', re: /\bTASK-\d+\b/g },
  { name: 'memo reference', re: /\b(?:local|internal|stack|research) memo\b|\bmemo [A-Z]\b/gi },
  { name: 'private spec citation', re: /\b(?:build[ ]spec|spec §\s*\d)/gi },
];
const NAMES = process.env.JM_PRIVATE_NAMES ? { name: 'private name', re: new RegExp(process.env.JM_PRIVATE_NAMES, 'gi') } : null;

function scan(text, classes = NAMES ? [...CLASSES, NAMES] : CLASSES) {
  const hits = [];
  text.split('\n').forEach((line, i) => {
    for (const c of classes) for (const m of line.matchAll(c.re)) if (!c.ok?.(m[0])) hits.push({ cls: c.name, line: i + 1, text: m[0] });
  });
  return hits;
}

const j = (...p) => p.join(''); // keeps sample strings out of this file's own text
const CASES = [
  // one hit per class
  ['operator path: home directory', j('/Users', '/someone/x'), 'operator path'],
  ['operator path: dotted tool dir', j('~/', '.commonly/state'), 'operator path'],
  ['operator path: shared temp', j('/private', '/tmp/x'), 'operator path'],
  ['email: personal address', j('someone@', 'gmail.com'), 'email'],
  ['team role', j('the Co', 'founder ruled'), 'team role'],
  ['24-hex id', j('pod ', 'a1b2c3d4e5f6', 'a7b8c9d0e1f2'), '24-hex id'],
  ['registry host', j('us-docker', '.pkg.dev/p/r'), 'registry host'],
  ['cloud project id', j('project_id', ': acme-prod-', '123456'), 'cloud project id'],
  ['cloud account id', j('account_id', '=', '0123456789abcdef0123456789abcdef'), 'cloud account id'],
  ['ledger id', j('see ', 'D', '123'), 'ledger id'],
  ['task label', j('TASK', '-77'), 'task label'],
  ['memo reference', j('Internal research ', 'memo', ' J'), 'memo reference'],
  ['private spec citation', j('see spec', ' section 2.4'), 'private spec citation'],
  ['private spec citation: named document', j('the build ', 'spec says'), 'private spec citation'],
  // one clean case per allow-list entry
  ['allowed: GitHub noreply address', j('1+x@users.', 'noreply.github.com'), null],
  ['allowed: userinfo in a test URL', j('https://tok@', 'github.com/o/r'), null],
  ['allowed: reserved example domain', j('a@', 'example.org'), null],
  ['allowed: obviously synthetic domain', j('http://localhost:80@', 'evil.com'), null],
  ['allowed: all-zero 24-hex id', '0'.repeat(24), null],
  ['allowed: documented test prefix', j('deadbeef', '0123456789abcdef'), null],
  // near misses stay clean
  ['clean: a 12-hex value is not an id', 'abcdef012345', null],
  ['clean: a spec file or a test spec is not a citation', 'the OpenAPI spec file; 1 Playwright spec', null],
  ['clean: ordinary prose', 'The pod core is pure; the ledger is the land script\'s verifier record.', null],
];

function runCases() {
  let bad = 0;
  for (const [name, text, want] of CASES) {
    const got = scan(text, CLASSES);
    const ok = want === null ? got.length === 0 : got.length > 0 && got.every((h) => h.cls === want);
    if (!ok) { bad += 1; console.error(`case failed: ${name} (wanted ${want ?? 'clean'}, got ${JSON.stringify(got)})`); }
  }
  return { bad, n: CASES.length };
}

function trackedText() {
  const files = execFileSync('git', ['ls-files', '-z'], { encoding: 'utf8', maxBuffer: 1 << 28 }).split('\0').filter(Boolean);
  return files.flatMap((f) => {
    let buf;
    try { buf = fs.readFileSync(f); } catch { return []; } // a deleted or unreadable path is not text
    return buf.includes(0) ? [] : [[f, buf.toString('utf8')]];
  });
}

function main() {
  const { bad, n } = runCases();
  if (bad > 0) { console.error(`check-private: ${bad} of ${n} cases failed`); process.exit(1); }
  let found = 0;
  for (const [f, text] of trackedText()) for (const h of scan(text)) { found += 1; console.error(`${f}:${h.line}: ${h.cls}: ${h.text}`); }
  if (found > 0) { console.error(`check-private: ${found} private-looking hit(s) in tracked files; fix them or extend the allow-list with a reason`); process.exit(1); }
  console.log(`check-private: ${n} cases ok; tracked files clean${NAMES ? '' : ' (names class skipped: JM_PRIVATE_NAMES unset)'}`);
}

if (import.meta.main) main();
