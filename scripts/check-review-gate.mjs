#!/usr/bin/env bun
// Tests the protected-path rule and the verdict reader (scripts/land-decide.mjs) and the reviewer list. Own step in `make test`.
import fs from 'node:fs';
import { readReviewerEntries, touchesProtected, verdictOf } from './land-decide.mjs';

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
  ['a hidden PASS inside a comment alone is nothing to judge', '<!-- GATE: PASS -->', null],
  ['a fullwidth colon blocks', 'GATE\uff1a PASS', 'BLOCK'],
  ['an empty message is nothing to judge', '\n  \n', null],
];
const paths = [
  ['a schema edit is protected', [{ filename: 'schema/x.json' }], true],
  ['a workflow edit is protected', [{ filename: '.github/workflows/ci.yml' }], true],
  ['the land scripts and reviewer list are protected', [{ filename: 'scripts/land-decide.mjs' }, { filename: '.github/opus-reviewers.txt' }], true],
  ['a rename OUT of schema/ is protected (previous_filename)', [{ filename: 'elsewhere/x.json', previous_filename: 'schema/x.json' }], true],
  ['a rename INTO schema/ is protected', [{ filename: 'schema/x.json', previous_filename: 'elsewhere/x.json' }], true],
  ['land and its decisions are protected', [{ filename: 'scripts/land.mjs' }, { filename: 'scripts/land-decide.mjs' }], true],
  ['the Makefile, init.sh and tools.sh are protected', [{ filename: 'Makefile' }], true],
  ['init.sh is protected', [{ filename: 'init.sh' }], true],
  ['tools.sh is protected', [{ filename: 'scripts/tools.sh' }], true],
  ['a rename out of land.mjs is protected', [{ filename: 'scripts/x.mjs', previous_filename: 'scripts/land.mjs' }], true],
  ['a docs-only PR is not protected', [{ filename: 'docs/INDEX.md' }, { filename: 'docs/design/x.md' }], false],
  ['progress.md alone is not protected', [{ filename: 'progress.md' }], false],
  ['a rename between docs paths is not protected', [{ filename: 'docs/b.md', previous_filename: 'docs/a.md' }], false],
  ['a root GNUmakefile is protected', [{ filename: 'GNUmakefile' }], true],
  ['package.json is protected', [{ filename: 'package.json' }], true],
  ['a git hook is protected', [{ filename: '.githooks/pre-commit' }], true],
  ['a check script is protected', [{ filename: 'scripts/check-x.mjs' }], true],
  ['gen.mjs and entropy.json are protected', [{ filename: 'scripts/gen.mjs' }, { filename: 'entropy.json' }], true],
  ['a file named progress.md.sh or sub/progress.md is protected', [{ filename: 'sub/progress.md' }], true],
  ['docs-lookalike docsx/ is protected', [{ filename: 'docsx/a.md' }], true],
  ['a rename docs/a -> scripts/a is protected', [{ filename: 'scripts/a', previous_filename: 'docs/a' }], true],
  ['a rename scripts/a -> docs/a is protected', [{ filename: 'docs/a', previous_filename: 'scripts/a' }], true],
  // Go compiles docs/**/*.go, and tooling may run any script: only inert documentation types are unprotected.
  ['docs/x/x_test.go is protected (go test ./... runs it)', [{ filename: 'docs/x/x_test.go' }], true],
  ['docs/a.go is protected', [{ filename: 'docs/a.go' }], true],
  ['docs/a.js is protected', [{ filename: 'docs/a.js' }], true],
  ['docs/a.mjs is protected', [{ filename: 'docs/a.mjs' }], true],
  ['docs/a.md is not protected', [{ filename: 'docs/a.md' }], false],
  ['docs/img/a.png is not protected', [{ filename: 'docs/img/a.png' }], false],
  ['every inert extension is not protected', ['html', 'svg', 'jpg', 'gif', 'webp'].map((e) => ({ filename: `docs/a.${e}` })), false],
  ['a rename docs/a.md -> docs/a.go is protected', [{ filename: 'docs/a.go', previous_filename: 'docs/a.md' }], true],
  ['a file with no extension under docs/ is protected', [{ filename: 'docs/Makefile' }], true],
  ['a .md that is a directory of code is protected', [{ filename: 'docs/a.md/run.sh' }], true],
  ['a docs change beside one protected file is protected', [{ filename: 'docs/a.md' }, { filename: 'go.mod' }], true],
];
const badPaths = paths.filter(([, files, want]) => touchesProtected(files) !== want).map(([t]) => t);
if (badPaths.length) { console.error(`check-review-gate: protected-path cases wrong\n  ${badPaths.join('\n  ')}`); process.exit(1); }
const text = fs.readFileSync(new URL('../.github/opus-reviewers.txt', import.meta.url), 'utf8');
const entries = readReviewerEntries(text);
const fileOk = entries.length > 0 && entries.every((e) => e.name && /^[a-z0-9][a-z0-9-]*$/.test(e.username || ''));
if (!fileOk) { console.error(`check-review-gate: .github/opus-reviewers.txt does not parse as 'Name username' lines: ${JSON.stringify(entries)}`); process.exit(1); }
const bad = verdicts.filter(([, text, want]) => verdictOf(text) !== want).map(([t]) => t);
if (bad.length) { console.error(`check-review-gate: ${bad.length} of ${verdicts.length} verdict cases wrong\n  ${bad.join('\n  ')}`); process.exit(1); }
console.log(`check-review-gate: ${verdicts.length} verdict cases and ${paths.length} path cases ok`);
