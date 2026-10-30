#!/usr/bin/env bun
// Tests the protected-path rule, the verdict reader and the allow-marker rule (scripts/land-decide.mjs), the reviewer list,
// and that check-gen bites (F01 step 4). Own step in `make test`.
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { addsAllowMarker, checkPodVerdict, filesFromRaw, mergeResult, readReviewerEntries, touchesProtected, verdictOf } from './land-decide.mjs';

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
const paths = [
  ['a schema edit is protected', [{ filename: 'schema/x.json' }], true],
  ['a workflow edit is protected', [{ filename: '.github/workflows/ci.yml' }], true],
  ['the land scripts and reviewer list are protected', [{ filename: 'scripts/land-decide.mjs' }, { filename: '.github/opus-reviewers.txt' }], true],
  ['a rename OUT of schema/ is protected (previous_filename)', [{ filename: 'elsewhere/x.json', previous_filename: 'schema/x.json' }], true],
  ['a rename INTO schema/ is protected', [{ filename: 'schema/x.json', previous_filename: 'elsewhere/x.json' }], true],
  ['the Makefile, init.sh and tools.sh are protected', [{ filename: 'Makefile' }], true],
  ['a docs-only PR is not protected', [{ filename: 'docs/INDEX.md' }, { filename: 'docs/design/x.md' }], false],
  ['progress.md alone is not protected', [{ filename: 'progress.md' }], false],
  ['a rename between docs paths is not protected', [{ filename: 'docs/b.md', previous_filename: 'docs/a.md' }], false],
  ['package.json is protected', [{ filename: 'package.json' }], true],
  ['a check script is protected', [{ filename: 'scripts/check-x.mjs' }], true],
  ['gen.mjs and entropy.json are protected', [{ filename: 'scripts/gen.mjs' }, { filename: 'entropy.json' }], true],
  ['a file named progress.md.sh or sub/progress.md is protected', [{ filename: 'sub/progress.md' }], true],
  ['docs-lookalike docsx/ is protected', [{ filename: 'docsx/a.md' }], true],
  ['a rename docs/a -> scripts/a is protected', [{ filename: 'scripts/a', previous_filename: 'docs/a' }], true],
  ['a rename scripts/a -> docs/a is protected', [{ filename: 'docs/a', previous_filename: 'scripts/a' }], true],
  // Go compiles docs/**/*.go, and tooling may run any script: only inert documentation types are unprotected.
  ['docs/x/x_test.go is protected (go test ./... runs it)', [{ filename: 'docs/x/x_test.go' }], true],
  ['docs/a.md is not protected', [{ filename: 'docs/a.md' }], false],
  ['docs/img/a.png is not protected', [{ filename: 'docs/img/a.png' }], false],
  ['every inert extension is not protected', ['html', 'svg', 'jpg', 'gif', 'webp'].map((e) => ({ filename: `docs/a.${e}` })), false],
  ['a rename docs/a.md -> docs/a.go is protected', [{ filename: 'docs/a.go', previous_filename: 'docs/a.md' }], true],
  ['a file with no extension under docs/ is protected', [{ filename: 'docs/Makefile' }], true],
  ['a .md that is a directory of code is protected', [{ filename: 'docs/a.md/run.sh' }], true],
  ['a symlink under docs/ is protected (mode 120000)', [{ filename: 'docs/lnk.md', modes: ['000000', '120000'] }], true],
  ['a symlink turned into a regular .md is protected (old mode 120000)', [{ filename: 'docs/a.md', modes: ['120000', '100644'] }], true],
  ['a gitlink under docs/ is protected (mode 160000)', [{ filename: 'docs/sub.md', modes: ['000000', '160000'] }], true],
  ['a regular docs file with modes is not protected', [{ filename: 'docs/a.md', modes: ['100644', '100644'] }, { filename: 'docs/b.md', modes: ['000000', '100755'] }], false],
  ['the land rules under docs/development/ are protected', [{ filename: 'docs/development/landing.md' }], true],
  ['the verifier protocol under docs/development/ is protected', [{ filename: 'docs/development/verifier.md' }], true],
  ['a rename out of docs/development/ is protected', [{ filename: 'docs/x.md', previous_filename: 'docs/development/workflow.md' }], true],
  ['a docs change beside one protected file is protected', [{ filename: 'docs/a.md' }, { filename: 'go.mod' }], true],
];
const hunk = (body, name = 'docs/a.md') => `diff --git a/${name} b/${name}\n--- a/${name}\n+++ b/${name}\n@@ -0,0 +1 @@\n${body}`;
const markers = [
  ['an added gitleaks:allow line makes the PR protected', hunk('+key = "x" # gitleaks:allow'), true],
  ['an added line starting "++" cannot hide the marker', hunk('++ secret gitleaks:allow'), true],
  ['a removed gitleaks:allow line does not', hunk('-key = "x" # gitleaks:allow'), false],
  ['a file header naming the marker does not', hunk('+hello', 'gitleaks:allow'), false],
  ['an unrelated added line does not', hunk('+hello'), false],
];
const badMarkers = markers.filter(([, d, want]) => addsAllowMarker(d) !== want).map(([t]) => t);
if (badMarkers.length) { console.error(`check-review-gate: allow-marker cases wrong\n  ${badMarkers.join('\n  ')}`); process.exit(1); }
const badPaths = paths.filter(([, files, want]) => touchesProtected(files) !== want).map(([t]) => t);
if (badPaths.length) { console.error(`check-review-gate: protected-path cases wrong\n  ${badPaths.join('\n  ')}`); process.exit(1); }
const text = fs.readFileSync(new URL('../.github/opus-reviewers.txt', import.meta.url), 'utf8');
const entries = readReviewerEntries(text);
const fileOk = entries.length > 0 && entries.every((e) => e.name && /^[a-z0-9][a-z0-9-]*$/.test(e.username || ''));
if (!fileOk) { console.error(`check-review-gate: .github/opus-reviewers.txt does not parse as 'Name username' lines: ${JSON.stringify(entries)}`); process.exit(1); }
const bad = verdicts.filter(([, text, want]) => verdictOf(text) !== want).map(([t]) => t);
if (bad.length) { console.error(`check-review-gate: ${bad.length} of ${verdicts.length} verdict cases wrong\n  ${bad.join('\n  ')}`); process.exit(1); }
console.log(`check-review-gate: ${verdicts.length} verdict cases and ${paths.length} path cases ok`);

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
// and look docs-only. The diff protects the PR, check-portable refuses the chain, and gen will not write through it. The victim must stay intact.
const base = fs.mkdtempSync(path.join(os.tmpdir(), 'check-symlink-'));
const repo = path.join(base, 'a', 'repo');
fs.mkdirSync(repo, { recursive: true });
fs.writeFileSync(path.join(base, 'victim.txt'), 'SAFE');
const at = (script) => spawnSync('bash', ['-c', script], { cwd: repo, encoding: 'utf8', env: { ...process.env, GIT_CONFIG_GLOBAL: '/dev/null', GIT_AUTHOR_NAME: 't', GIT_AUTHOR_EMAIL: 't@l', GIT_COMMITTER_NAME: 't', GIT_COMMITTER_EMAIL: 't@l' } });
at(`mkdir -p daemon/internal docs && cp -R '${root}/scripts' '${root}/schema' '${root}/gen' . && cp -R '${root}/daemon/internal/protocol' daemon/internal/ && cp '${root}/docs/protocol.md' docs/ && git init -q && git add -A && git commit -qm base`);
const cleanPortable = at('bash scripts/check-portable.sh').status === 0;
at('rm docs/protocol.md && ln -s lnk.md/../victim.txt docs/protocol.md && ln -s ../.. docs/lnk.md && git add -A && git commit -qm links');
const chain = filesFromRaw(at('git diff --raw -M HEAD~1 HEAD').stdout);
const results = [
  ['symlink chain: the PR is protected, not docs-only', touchesProtected(chain), true],
  ['symlink chain: the clean tree passes check-portable (so the next case is the symlinks)', cleanPortable, true],
  ['symlink chain: check-portable refuses the chain', at('bash scripts/check-portable.sh').status !== 0, true],
  ['symlink chain: gen refuses to write through it', at(`'${process.execPath}' scripts/gen.mjs`).status !== 0, true],
  ['symlink chain: the file above the tree is untouched', fs.readFileSync(path.join(base, 'victim.txt'), 'utf8'), 'SAFE'],
];
fs.rmSync(base, { recursive: true, force: true });
const badChain = results.filter(([, got, want]) => got !== want).map(([n]) => n);
if (badChain.length) { console.error(`check-review-gate: symlink cases wrong\n  ${badChain.join('\n  ')}`); process.exit(1); }
console.log(`check-review-gate: ${results.length} symlink cases ok`);

// B2: a gitleaks:allow marker added and removed inside the PR's history leaves the merge result clean, yet the merge keeps both commits and
// gitleaks honours the marker per commit. The history scan must protect it (and the pod verdict must then be missing: refused), and a
// secret scan that ignores markers must find what the marker hid. The control carries no marker and is caught as is.
const token = ['ghp_', 'aB3dE5gH7jK9m', 'N1pQ3sT5vW7yZ9bD2fH4jL6'].join('');
const hist = (marker) => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'check-hist-'));
  const g = (script) => spawnSync('bash', ['-ec', script], { cwd: dir, encoding: 'utf8', env: { ...process.env, GIT_CONFIG_GLOBAL: '/dev/null', GIT_AUTHOR_NAME: 't', GIT_AUTHOR_EMAIL: 't@l', GIT_COMMITTER_NAME: 't', GIT_COMMITTER_EMAIL: 't@l' } });
  g(`git init -q -b main && echo base > f && git add -A && git commit -qm base && git checkout -qb pr && echo 'token = ${token}${marker ? ' # gitleaks:allow' : ''}' > t.txt && git add -A && git commit -qm add && git rm -q t.txt && git commit -qm drop`);
  const run = (a) => g(`git ${a.map((x) => `'${x}'`).join(' ')}`).stdout.trim();
  const [tip, head] = [run(['rev-parse', 'main']), run(['rev-parse', 'pr'])];
  const merged = mergeResult(run, tip, head);
  const leaks = (ignore) => spawnSync('go', ['run', 'github.com/zricethezav/gitleaks/v8@v8.30.1', 'git', `--log-opts=${tip}..${head}`, ...(ignore ? ['--ignore-gitleaks-allow'] : []), '--redact', '--no-banner', dir], { encoding: 'utf8' }).status !== 0;
  const out = { merged, head, found: leaks(true), foundHonouringMarker: leaks(false) };
  fs.rmSync(dir, { recursive: true, force: true });
  return out;
};
const [marked, control] = [hist(true), hist(false)];
const noVerdict = (h) => checkPodVerdict([], { pr: '5', head: h, entries: [{ name: 'Ada', username: 'ada' }] }).ok;
const history = [
  ['history: the merge result alone shows nothing (the verifier\'s fixture)', marked.merged.files.length === 0 && !addsAllowMarker(marked.merged.treePatch), true],
  ['history: the marker added then removed still protects the PR', addsAllowMarker(marked.merged.patch), true],
  ['history: such a PR is refused without a pod verdict', noVerdict(marked.head), false],
  ['history: ignoring markers, gitleaks finds the hidden token', marked.found, true],
  ['history: honouring the marker, gitleaks sees nothing (the hole)', marked.foundHonouringMarker, false],
  ['history: the control without a marker is caught by gitleaks', control.found && control.foundHonouringMarker, true],
  ['history: the control without a marker does not trip the marker scan', addsAllowMarker(control.merged.patch), false],
];
const badHist = history.filter(([, got, want]) => got !== want).map(([n]) => n);
if (badHist.length) { console.error(`check-review-gate: history cases wrong\n  ${badHist.join('\n  ')}`); process.exit(1); }
console.log(`check-review-gate: ${history.length} history cases ok`);
