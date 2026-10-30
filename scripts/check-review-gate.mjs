#!/usr/bin/env bun
// Tests the verdict reader (scripts/land-decide.mjs), that check-gen bites (F01 step 4), the portability check, and the real
// secret scans land refuses on. Own step in `make test`.
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { verdictOf } from './land-decide.mjs';

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

// B2 end to end: two symlinks, docs/lnk.md -> ../.. and docs/protocol.md -> lnk.md/../victim.txt, would let `make gen` overwrite a file above the tree
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

// A gitleaks:allow marker added and removed inside the PR's history leaves the merge result clean, yet the merge keeps both commits and
// gitleaks honours the marker per commit. The range scan ignores markers and must find what the marker hid; it can only refuse.
const token = ['ghp_', 'aB3dE5gH7jK9m', 'N1pQ3sT5vW7yZ9bD2fH4jL6'].join('');
const hist = (marker) => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'check-hist-'));
  const g = (script) => spawnSync('bash', ['-ec', script], { cwd: dir, encoding: 'utf8', env: { ...process.env, GIT_CONFIG_GLOBAL: '/dev/null', GIT_AUTHOR_NAME: 't', GIT_AUTHOR_EMAIL: 't@l', GIT_COMMITTER_NAME: 't', GIT_COMMITTER_EMAIL: 't@l' } });
  g(`git init -q -b main && echo base > f && git add -A && git commit -qm base && git checkout -qb pr && echo 'token = ${token}${marker ? ' # gitleaks:allow' : ''}' > t.txt && git add -A && git commit -qm add && git rm -q t.txt && git commit -qm drop`);
  const run = (a) => g(`git ${a.map((x) => `'${x}'`).join(' ')}`).stdout.trim();
  const [tip, head] = [run(['rev-parse', 'main']), run(['rev-parse', 'pr'])];
  const leaks = (ignore) => spawnSync('go', ['run', 'github.com/zricethezav/gitleaks/v8@v8.30.1', 'git', `--log-opts=${tip}..${head}`, ...(ignore ? ['--ignore-gitleaks-allow'] : []), '--redact', '--no-banner', dir], { encoding: 'utf8' }).status !== 0;
  const out = { found: leaks(true), foundHonouringMarker: leaks(false) };
  fs.rmSync(dir, { recursive: true, force: true });
  return out;
};
const [marked, control] = [hist(true), hist(false)];
const history = [
  ['history: ignoring markers, gitleaks finds the hidden token', marked.found, true],
  ['history: honouring the marker, gitleaks sees nothing (the hole)', marked.foundHonouringMarker, false],
  ['history: the control without a marker is caught by gitleaks', control.found && control.foundHonouringMarker, true],
];
const badHist = history.filter(([, got, want]) => got !== want).map(([n]) => n);
if (badHist.length) { console.error(`check-review-gate: history cases wrong\n  ${badHist.join('\n  ')}`); process.exit(1); }
console.log(`check-review-gate: ${history.length} history cases ok`);

// NB6: a symlink path with a space resolves whole (records are NUL-separated): one that leaves the repo is refused by name, one inside passes.
const pdir = fs.mkdtempSync(path.join(os.tmpdir(), 'check-portable-'));
const pg = (script) => spawnSync('bash', ['-c', script], { cwd: pdir, encoding: 'utf8', env: { ...process.env, GIT_CONFIG_GLOBAL: '/dev/null', GIT_AUTHOR_NAME: 't', GIT_AUTHOR_EMAIL: 't@l', GIT_COMMITTER_NAME: 't', GIT_COMMITTER_EMAIL: 't@l' } });
pg(`mkdir scripts sp && cp '${root}/scripts/check-portable.sh' scripts/ && echo x > scripts/f && ln -s ../scripts "sp/in side.md" && git init -q && git add -A && git commit -qm a`);
const insideOk = pg('bash scripts/check-portable.sh').status === 0;
pg('ln -s ../.. "sp/out side.md" && git add -A && git commit -qm b');
const outside = pg('bash scripts/check-portable.sh');
fs.rmSync(pdir, { recursive: true, force: true });
const portable = [['portable: a symlink with a space that stays inside passes', insideOk, true], ['portable: a symlink with a space that leaves the repo is refused by its whole path', outside.status !== 0 && outside.stdout.includes('sp/out side.md'), true]];
const badPortable = portable.filter(([, got, want]) => got !== want).map(([n]) => n);
if (badPortable.length) { console.error(`check-review-gate: portable cases wrong\n  ${badPortable.join('\n  ')}`); process.exit(1); }
console.log(`check-review-gate: ${portable.length} portable cases ok`);

// B1 evil merge: gitleaks runs `git log -p` without -m, so a secret that exists only in a merge commit's own diff (a conflict resolution)
// is never scanned. Both targets pass -m. A trusted branch merges a side branch and adds a token to docs/a.md in the resolution.
{
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'check-evil-'));
  const g = (script) => spawnSync('bash', ['-ec', script], { cwd: dir, encoding: 'utf8', env: { ...process.env, GIT_CONFIG_GLOBAL: '/dev/null', GIT_AUTHOR_NAME: 't', GIT_AUTHOR_EMAIL: 't@l', GIT_COMMITTER_NAME: 't', GIT_COMMITTER_EMAIL: 't@l' } });
  const evil = (add) => `git init -q -b main . && mkdir docs && echo base > docs/a.md && git add -A && git commit -qm base && git checkout -qb side && echo side > docs/s.md && git add -A && git commit -qm side && git checkout -q main && git checkout -qb pr && echo mine > docs/m.md && git add -A && git commit -qm mine && git merge -q --no-ff --no-commit side && ${add ? `echo 'token = ${token}' >> docs/a.md && git add -A && ` : ''}git commit -qm merge`;
  const gmake = (target) => { const r = spawnSync('make', ['-s', target, 'RANGE=main..pr'], { cwd: dir, encoding: 'utf8' }); return target === 'secrets-range' ? (/^exit status 3$/m.test(r.stderr) ? 3 : r.status) : r.status; };
  g(`cp '${root}/Makefile' . && ${evil(true)}`);
  const [secretsEvil, rangeEvil] = [gmake('secrets'), gmake('secrets-range')];
  fs.rmSync(dir, { recursive: true, force: true });
  fs.mkdirSync(dir);
  g(`cp '${root}/Makefile' . && ${evil(false)}`);
  const [secretsClean, rangeClean] = [gmake('secrets'), gmake('secrets-range')];
  fs.rmSync(dir, { recursive: true, force: true });
  const evilCases = [
    ['evil merge: make secrets finds a token only in a merge commit', secretsEvil !== 0, true],
    ['evil merge: make secrets-range finds it too: gitleaks exit 3, shown as "exit status 3", which is how land tells a hit from a scan that failed', rangeEvil, 3],
    ['evil merge: the same history without the token is clean under both', secretsClean === 0 && rangeClean === 0, true],
  ];
  const badEvil = evilCases.filter(([, got, want]) => got !== want).map(([n]) => n);
  if (badEvil.length) { console.error(`check-review-gate: evil-merge cases wrong\n  ${badEvil.join('\n  ')}`); process.exit(1); }
  console.log(`check-review-gate: ${evilCases.length} evil-merge cases ok`);
}

// The range scan reads every commit of <main tip>..<head> through the real recipe, markers ignored: a plain-text token added in one commit
// and deleted in the next (in a path the scanner does not allowlist) is still in history and must be found. It must scan the range, not the
// working tree (the checkout is main on a normal land) and not only the head commit, and it must not honour a gitleaks:allow marker.
{
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'check-range-'));
  const g = (script) => spawnSync('bash', ['-ec', script], { cwd: dir, encoding: 'utf8', env: { ...process.env, GIT_CONFIG_GLOBAL: '/dev/null', GIT_AUTHOR_NAME: 't', GIT_AUTHOR_EMAIL: 't@l', GIT_COMMITTER_NAME: 't', GIT_COMMITTER_EMAIL: 't@l' } });
  const rangeStatus = (marker) => {
    fs.rmSync(dir, { recursive: true, force: true });
    fs.mkdirSync(dir);
    g(`cp '${root}/Makefile' . && git init -q -b main . && echo base > notes.txt && git add -A && git commit -qm base && git checkout -qb pr && echo 'token = ${token}${marker ? ' # gitleaks:allow' : ''}' > notes.txt && git add -A && git commit -qm add && git checkout -q main -- notes.txt && git commit -qam drop`);
    const r = spawnSync('make', ['-s', 'secrets-range', 'RANGE=main..pr'], { cwd: dir, encoding: 'utf8' });
    return /^exit status 3$/m.test(r.stderr) ? 3 : r.status;
  };
  const [plain, marked, none] = [rangeStatus(false), rangeStatus(true), (() => { fs.rmSync(dir, { recursive: true, force: true }); fs.mkdirSync(dir); g(`cp '${root}/Makefile' . && git init -q -b main . && echo base > notes.txt && git add -A && git commit -qm base && git checkout -qb pr && echo more > notes.txt && git add -A && git commit -qm add`); return spawnSync('make', ['-s', 'secrets-range', 'RANGE=main..pr'], { cwd: dir, encoding: 'utf8' }).status; })()];
  fs.rmSync(dir, { recursive: true, force: true });
  const rangeCases = [
    ['range: a plain-text token added then deleted in the range is found (exit status 3), by the recipe itself', plain, 3],
    ['range: a gitleaks:allow marker does not hide it (the recipe ignores markers)', marked, 3],
    ['range: a range without a token is clean', none, 0],
  ];
  const badRange = rangeCases.filter(([, got, want]) => got !== want).map(([n]) => n);
  if (badRange.length) { console.error(`check-review-gate: range cases wrong\n  ${badRange.join('\n  ')}`); process.exit(1); }
  console.log(`check-review-gate: ${rangeCases.length} range cases ok`);
}
