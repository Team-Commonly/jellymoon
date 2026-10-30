#!/usr/bin/env bun
// Tests the protected-path rule, the verdict reader and the allow-marker rule (scripts/land-decide.mjs), the reviewer list,
// and that check-gen bites (F01 step 4). Own step in `make test`.
import { execFileSync, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { addsAllowMarker, readReviewerEntries, touchesProtected, verdictOf } from './land-decide.mjs';

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
  ['a docs change beside one protected file is protected', [{ filename: 'docs/a.md' }, { filename: 'go.mod' }], true],
];
const markers = [
  ['an added gitleaks:allow line makes the PR protected', '+++ b/docs/a.md\n+key = "x" # gitleaks:allow', true],
  ['a removed gitleaks:allow line does not', '--- a/x\n-key = "x" # gitleaks:allow', false],
  ['a context mention in the +++ header does not', '+++ b/gitleaks:allow', false],
  ['an unrelated added line does not', '+hello', false],
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

// F01 step 4: a schema edit without `make gen` must fail check-gen. In a throwaway copy of what gen reads and writes: gen leaves it
// clean, then a schema edit followed by gen changes the generated files, which is what check-gen's `git diff --exit-code` catches.
const root = path.resolve(import.meta.dirname, '..');
const copy = fs.mkdtempSync(path.join(os.tmpdir(), 'check-gen-bites-'));
const sh = (cmd, args) => spawnSync(cmd, args, { cwd: copy, encoding: 'utf8' });
const GEN = ['gen', 'daemon/internal/protocol/gen.go', 'docs/protocol.md'];
const dirty = () => sh('git', ['diff', '--quiet', '--', ...GEN]).status !== 0;
for (const f of execFileSync('git', ['ls-files', '-z'], { cwd: root, encoding: 'utf8' }).split('\0').filter(Boolean)) {
  if (!/^(scripts\/|schema\/|gen\/|daemon\/internal\/protocol\/|docs\/protocol\.md$)/.test(f) || !fs.lstatSync(path.join(root, f)).isFile()) continue;
  fs.mkdirSync(path.dirname(path.join(copy, f)), { recursive: true });
  fs.copyFileSync(path.join(root, f), path.join(copy, f));
}
sh('git', ['init', '-q']);
sh('git', ['add', '-A']);
sh('git', ['-c', 'user.name=t', '-c', 'user.email=t@localhost', 'commit', '-qm', 'base']);
const gen = () => sh(process.execPath, ['scripts/gen.mjs']).status;
const first = gen();
const cleanAfterGen = first === 0 && !dirty();
const schema = path.join(copy, 'schema', 'common.json');
fs.writeFileSync(schema, fs.readFileSync(schema, 'utf8').replace('"description": "', '"description": "EDITED '));
const bites = gen() === 0 && dirty();
fs.rmSync(copy, { recursive: true, force: true });
if (!cleanAfterGen || !bites) { console.error(`check-review-gate: check-gen does not bite (gen leaves a clean tree: ${cleanAfterGen}; a schema edit changes generated files: ${bites})`); process.exit(1); }
console.log('check-review-gate: check-gen bites (a schema edit changes the generated files)');
