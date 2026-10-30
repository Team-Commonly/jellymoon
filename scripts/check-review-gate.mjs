#!/usr/bin/env bun
// Tests the verdict reader (scripts/land-decide.mjs), that check-gen bites (F01 step 4), the portability check, and that the Makefile's
// secrets-range recipe is the command land runs. The real range scans are in check-scan-history.mjs and check-scan-checkout.mjs. Own step
// in `make test`.
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { verdictOf } from './land-decide.mjs';
import { GITLEAKS_MODULE, scanArgv } from './land-scan.mjs';

// Only exactly `GATE: PASS` as the last non-empty line passes; everything else blocks (nothing at all is null).
const verdicts = [
  ['a bare PASS line passes', 'notes\nGATE: PASS', 'PASS'],
  ['PASS in any case passes', 'gate: pass', 'PASS'],
  ['emphasis and a list marker are stripped', '- **GATE: PASS**', 'PASS'],
  ['a FAIL blocks', 'GATE: FAIL', 'BLOCK'],
  ['HOLD blocks', 'GATE: HOLD', 'BLOCK'],
  ['prose after the PASS blocks', 'GATE: PASS\nscratch that', 'BLOCK'],
  ['an indented PASS is code and blocks', '    GATE: PASS', 'BLOCK'],
  ['a hidden comment after the PASS does not hide it', 'GATE: PASS\n<!-- GATE: FAIL -->', 'PASS'],
  ['a fullwidth colon blocks', 'GATE\uff1a PASS', 'BLOCK'],
  ['an empty message is nothing to judge', '\n  \n', null],
];
const bad = verdicts.filter(([, text, want]) => verdictOf(text) !== want).map(([t]) => t);
if (bad.length) { console.error(`check-review-gate: ${bad.length} of ${verdicts.length} verdict cases wrong\n  ${bad.join('\n  ')}`); process.exit(1); }
console.log(`check-review-gate: ${verdicts.length} verdict cases ok`);

// F01 step 4: a schema edit without `make gen` must fail check-gen. In a throwaway git copy of what gen reads and writes, gen leaves the
// tree clean, then a schema edit followed by gen changes the generated files, which is what check-gen's `git diff --exit-code` catches.
const root = path.resolve(import.meta.dirname, '..');
const copy = fs.mkdtempSync(path.join(os.tmpdir(), 'check-gen-bites-'));
const sh = (script) => spawnSync('bash', ['-c', script], { cwd: copy, encoding: 'utf8', env: { ...process.env, GIT_CONFIG_GLOBAL: '/dev/null', GIT_AUTHOR_NAME: 't', GIT_AUTHOR_EMAIL: 't@l', GIT_COMMITTER_NAME: 't', GIT_COMMITTER_EMAIL: 't@l' } }).status;
const gen = `'${process.execPath}' scripts/gen.mjs >/dev/null || exit 2; git diff --quiet -- gen daemon/internal/protocol docs/protocol.md`;
sh(`mkdir -p daemon/internal docs && cp -R '${root}/scripts' '${root}/schema' '${root}/gen' . && cp -R '${root}/daemon/internal/protocol' daemon/internal/ && cp '${root}/docs/protocol.md' docs/ && git init -q && git add -A && git commit -qm base`);
const cleanAfterGen = sh(gen) === 0;
sh(`sed -i.bak 's/"description": "/"description": "EDITED /' schema/common.json`);
const bites = sh(gen) === 1; // 1: gen ran and the generated files differ (2 would be gen itself failing)
fs.rmSync(copy, { recursive: true, force: true });
if (!cleanAfterGen || !bites) { console.error(`check-review-gate: check-gen does not bite (gen leaves a clean tree: ${cleanAfterGen}; a schema edit changes generated files: ${bites})`); process.exit(1); }
console.log('check-review-gate: check-gen bites (a schema edit changes the generated files)');

// A symlink chain gen must not write through: two symlinks, docs/lnk.md -> ../.. and docs/protocol.md -> lnk.md/../victim.txt, would let `make gen` overwrite a file above the tree
// check-portable refuses the chain, and gen will not write through it. The victim must stay intact.
const base = fs.mkdtempSync(path.join(os.tmpdir(), 'check-symlink-'));
const repo = path.join(base, 'a', 'repo');
fs.mkdirSync(repo, { recursive: true });
fs.writeFileSync(path.join(base, 'victim.txt'), 'SAFE');
const at = (script) => spawnSync('bash', ['-c', script], { cwd: repo, encoding: 'utf8', env: { ...process.env, GIT_CONFIG_GLOBAL: '/dev/null', GIT_AUTHOR_NAME: 't', GIT_AUTHOR_EMAIL: 't@l', GIT_COMMITTER_NAME: 't', GIT_COMMITTER_EMAIL: 't@l' } });
at(`mkdir -p daemon/internal docs && cp -R '${root}/scripts' '${root}/schema' '${root}/gen' . && cp -R '${root}/daemon/internal/protocol' daemon/internal/ && cp '${root}/docs/protocol.md' docs/ && git init -q && git add -A && git commit -qm base`);
const cleanPortable = at('bash scripts/check-portable.sh').status === 0;
at('rm docs/protocol.md && ln -s lnk.md/../victim.txt docs/protocol.md && ln -s ../.. docs/lnk.md && git add -A && git commit -qm links');
const results = [
  ['symlink chain: the clean tree passes check-portable (so the next case is the symlinks)', cleanPortable, true],
  ['symlink chain: check-portable refuses the chain', at('bash scripts/check-portable.sh').status !== 0, true],
  ['symlink chain: gen refuses to write through it', at(`'${process.execPath}' scripts/gen.mjs`).status !== 0, true],
  ['symlink chain: the file above the tree is untouched', fs.readFileSync(path.join(base, 'victim.txt'), 'utf8'), 'SAFE'],
];
fs.rmSync(base, { recursive: true, force: true });
const badChain = results.filter(([, got, want]) => got !== want).map(([n]) => n);
if (badChain.length) { console.error(`check-review-gate: symlink cases wrong\n  ${badChain.join('\n  ')}`); process.exit(1); }
console.log(`check-review-gate: ${results.length} symlink cases ok`);

// check-portable reads whole paths: a symlink path with a space resolves whole (records are NUL-separated): one that leaves the repo is refused by name, one inside passes.
const pdir = fs.mkdtempSync(path.join(os.tmpdir(), 'check-portable-'));
const pg = (script) => spawnSync('bash', ['-c', script], { cwd: pdir, encoding: 'utf8', env: { ...process.env, GIT_CONFIG_GLOBAL: '/dev/null', GIT_AUTHOR_NAME: 't', GIT_AUTHOR_EMAIL: 't@l', GIT_COMMITTER_NAME: 't', GIT_COMMITTER_EMAIL: 't@l' } });
pg(`mkdir scripts sp && cp '${root}/scripts/check-portable.sh' scripts/ && echo x > scripts/f && ln -s ../scripts "sp/in side.md" && git init -q && git add -A && git commit -qm a`);
const insideOk = pg('bash scripts/check-portable.sh').status === 0;
pg('ln -s ../.. "sp/out side.md" && git add -A && git commit -qm b');
const outside = pg('bash scripts/check-portable.sh');
fs.rmSync(pdir, { recursive: true, force: true });
// Installed tools and dependencies: a path component named .tools or node_modules, in any case, a directory or the entry itself (a symlink).
const tracked = (files, links = []) => {
  const d = fs.mkdtempSync(path.join(os.tmpdir(), 'check-portable-'));
  fs.mkdirSync(path.join(d, 'scripts'));
  fs.copyFileSync(path.join(root, 'scripts', 'check-portable.sh'), path.join(d, 'scripts', 'check-portable.sh'));
  for (const f of files) { fs.mkdirSync(path.dirname(path.join(d, f)), { recursive: true }); fs.writeFileSync(path.join(d, f), 'x\n'); }
  for (const l of links) { fs.mkdirSync(path.dirname(path.join(d, l)), { recursive: true }); fs.symlinkSync('../scripts', path.join(d, l)); }
  const r = spawnSync('bash', ['-c', 'git init -q && git add -Af && git commit -qm a && bash scripts/check-portable.sh'], { cwd: d, encoding: 'utf8', env: { ...process.env, GIT_CONFIG_GLOBAL: '/dev/null', GIT_AUTHOR_NAME: 't', GIT_AUTHOR_EMAIL: 't@l', GIT_COMMITTER_NAME: 't', GIT_COMMITTER_EMAIL: 't@l' } });
  fs.rmSync(d, { recursive: true, force: true });
  return r.status !== 0 && /installed tools or dependencies are tracked/.test(r.stdout);
};
const many = Array.from({ length: 1500 }, (_, i) => `node_modules/a-package-with-a-long-enough-name-to-fill-a-pipe-buffer-${i}/index.js`); // about 120 KB of names
const portable = [['portable: a symlink with a space that stays inside passes', insideOk, true], ['portable: a symlink with a space that leaves the repo is refused by its whole path', outside.status !== 0 && outside.stdout.includes('sp/out side.md'), true],
  ['portable: a tracked .tools that is a symlink (an entry with no slash) is refused', tracked([], ['.tools']), true],
  ['portable: a tracked .TOOLS/bin/go is refused (any case)', tracked(['.TOOLS/bin/go']), true],
  ['portable: a tracked sub/NODE_MODULES symlink is refused', tracked([], ['sub/NODE_MODULES']), true],
  ['portable: names that only contain the words pass (my.tools/, .toolsrc, node_modules.md)', tracked(['my.tools/x', '.toolsrc', 'docs/node_modules.md']), false],
  ['portable: a list of tracked node_modules files longer than a pipe buffer is refused (a head under pipefail once passed it)', tracked(many), true],
];
const badPortable = portable.filter(([, got, want]) => got !== want).map(([n]) => n);
if (badPortable.length) { console.error(`check-review-gate: portable cases wrong\n  ${badPortable.join('\n  ')}`); process.exit(1); }
console.log(`check-review-gate: ${portable.length} portable cases ok`);

// The Makefile's secrets-range is for people; land does not run it (land runs gitleaks itself with scanArgv, below). The pin is only a
// consistency check that the recipe people run is the command land runs, and it is NOT a security boundary: a main Makefile can change what
// make reads while passing any text pin (a built variable name, $(eval), include, a tracked GNUmakefile, SHELL, an exported variable), which is why
// make is off land's path. Comparisons are exact, and no other line may assign RANGE, GO or GITLEAKS in a plain form. Backslash continuations
// are joined first; full-line comments are skipped.
const PIN = 'secrets-range:\n\t$(GO) run $(GITLEAKS) git --log-opts="-m $(RANGE)" --ignore-gitleaks-allow --exit-code 3 --redact --no-banner .';
const VARS = 'GO := $(or $(wildcard $(CURDIR)/.tools/bin/go),go)\nGITLEAKS := github.com/zricethezav/gitleaks/v8@v8.30.1';
const OURS = /^(GO|GITLEAKS) :=/;
const pinProblems = (mk) => {
  const lines = mk.replace(/\\\n/g, ' ').split('\n');
  const at = lines.indexOf('secrets-range:');
  const stray = lines.filter((l) => !/^\s*#/.test(l) && !OURS.test(l) && (/(^|[\s:(])(RANGE|GO|GITLEAKS)\s*[:?+!]*=/.test(l) || /^\s*(override\s+)?define\s+(RANGE|GO|GITLEAKS)\b/.test(l)));
  return [
    at < 0 || lines.slice(at, lines.indexOf('', at)).join('\n') !== PIN ? 'the secrets-range recipe is not exactly the pinned line' : '',
    lines.filter((l) => /^secrets-range\b/.test(l)).length !== 1 ? 'secrets-range is defined more than once' : '',
    lines.filter((l) => OURS.test(l)).join('\n') !== VARS ? 'the go and gitleaks definitions are not exactly the pinned ones' : '',
    stray.length ? `another line assigns RANGE, GO or GITLEAKS: ${stray[0].trim()}` : '',
  ].filter(Boolean);
};
{
  const mk = fs.readFileSync(path.join(root, 'Makefile'), 'utf8');
  const recipe = mk.split('\n').find((l) => l.startsWith('\t$(GO) run $(GITLEAKS) git --log-opts="-m $(RANGE)"'));
  const mutants = [
    ['an override RANGE line', `${mk}\noverride RANGE := HEAD\n`],
    ['a target-specific GO', `${mk}\nsecrets-range: GO = true\n`],
    ['an exported GITLEAKS with ?=', `${mk}\nexport GITLEAKS ?= true\n`],
    ['RANGE += with no spaces', `${mk}\nRANGE+=x\n`],
    ['GO= with no spaces', `${mk}\nGO=true\n`],
    ['a define GO block', `${mk}\ndefine GO\ntrue\nendef\n`],
    ['an assignment hidden by a continuation', `${mk}\noverride \\\nRANGE := HEAD\n`],
    ['a second secrets-range target', `${mk}\nsecrets-range:\n\ttrue\n`],
    ['|| true appended to the recipe line', mk.replace(recipe, `${recipe} || true`)],
    ['a prefix of the recipe line (a flag dropped from the end)', mk.replace(recipe, recipe.replace(' --no-banner .', ''))],
    ['the GITLEAKS version changed', mk.replace('gitleaks/v8@v8.30.1', 'gitleaks/v8@v8.30.2')],
    ['a second GO := line in the pinned form (only the exact compare of the two definitions sees it)', `${mk}\nGO := /usr/bin/true\n`],
  ];
  const pinCases = [
    ['recipe pin: the real Makefile passes every check', pinProblems(mk).join('; '), ''],
    ['recipe pin: the pinned recipe, expanded, is the argv land runs (the Makefile and land cannot drift apart)', PIN.split('\n')[1].trim().replace('$(GO) run ', '').replace('$(GITLEAKS)', GITLEAKS_MODULE).replace('"-m $(RANGE)"', '-m R'), scanArgv('R').slice(1).join(' ')],
    ...mutants.map(([name, text]) => [`recipe pin: ${name} is refused`, pinProblems(text).length > 0, true]),
  ];
  const badPin = pinCases.filter(([, got, want]) => got !== want).map(([n]) => n);
  if (badPin.length) { console.error(`check-review-gate: recipe pin wrong\n  ${badPin.join('\n  ')}`); process.exit(1); }
  console.log(`check-review-gate: ${pinCases.length} recipe pin cases ok`);
}
