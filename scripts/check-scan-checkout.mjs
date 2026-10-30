#!/usr/bin/env bun
// The real range scan against what a land checkout itself holds, run the way land runs it (scan-fixture.mjs): the .tools that land takes go
// from, and the gitleaks configuration a tree or the checkout could carry. Each fixture is a checkout of main whose tip carries the variant, and a PR commit off that tip that adds a token, scanned with the
// checkout's OWN .tools (never this repo's). Every variant must be refused, never clean; the honest layouts must land. Own step in `make test`.
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { cacheTools, gitEnv, ownTools, rangeIn, token } from './scan-fixture.mjs';

const dir0 = fs.mkdtempSync(path.join(os.tmpdir(), 'check-checkout-'));
// A go that main could carry: for `run ... git --log-opts=-m <range> ...` it prints what land reads as clean, with the count land derives.
const fake = path.join(dir0, 'fake-go');
fs.writeFileSync(fake, `#!/bin/sh
for a in "$@"; do case "$a" in --log-opts=-m*) range=$(printf '%s' "$a" | cut -c15-) ;; esac; done
n=$(git log -m -p -U0 --format='XSHA %H' "$range" | awk '/^XSHA /{s=$2;next} /^diff --git /{d=0;next} /^deleted file mode /{d=1;next} /^@@ /{if(!d&&s!="")seen[s]=1} END{c=0;for(k in seen)c++;print c}')
echo "1:00PM INF $n commits scanned." >&2
echo "1:00PM INF no leaks found" >&2
`, { mode: 0o755 });
const elsewhere = path.join(dir0, 'elsewhere'); // outside every fixture checkout, where an untracked link could point
fs.mkdirSync(path.join(elsewhere, 'bin'), { recursive: true });
fs.copyFileSync(fake, path.join(elsewhere, 'bin', 'go'));
fs.chmodSync(path.join(elsewhere, 'bin', 'go'), 0o755);

let n = 0;
// main's tip runs `onMain` before its commit (it may force-add what .gitignore hides); the PR runs `inPr`, then adds `text`; the checkout goes
// back to main and `inCheckout(dir)` adds what the checkout holds untracked. Returns land's scan of <main tip>..<PR head> in that checkout.
const landIn = ({ onMain = 'true', inPr = 'true', inCheckout = () => {}, text = `token = ${token}` } = {}) => {
  const dir = path.join(dir0, `c${n += 1}`);
  fs.mkdirSync(dir);
  const g = (script) => {
    const r = spawnSync('bash', ['-ec', script], { cwd: dir, encoding: 'utf8', env: gitEnv });
    if (r.status !== 0) throw new Error(`fixture setup failed: ${script}\n${r.stderr}`);
    return r.stdout.trim();
  };
  g(`git init -q -b main . && printf '.tools/\\n' > .gitignore && echo base > notes.txt && git add -A && git commit -qm base && ${onMain} && git add -A && git commit -q --allow-empty -m main`);
  const tip = g('git rev-parse HEAD');
  g(`git checkout -qb pr && ${inPr} && echo '${text}' > t.txt && git add -A && git commit -qm pr && git checkout -q main`);
  const head = g('git rev-parse pr');
  inCheckout(dir);
  return rangeIn(dir, `${tip}..${head}`);
};
const refused = (r, re) => `${r.result}/${re.test(r.reason ?? '')}`;
const FAKE_DIR = `mkdir -p tools-fake/bin && cp '${fake}' tools-fake/bin/go`;
const ALLOW = '[extend]\\nuseDefault = true\\n[allowlist]\\npaths = ["t.txt"]\\n'; // a config that would hide the PR's t.txt, for printf

// A tracked .tools, in each form main could ship it: refused from the trees, before any go runs. The force-tracked and the upper-case forms
// pass every filesystem check (real directories, go under .tools), so only the tree refusal, case-folded, stops them.
const cases = [
  ['checkout .tools: main ships .tools as a tracked symlink to a tracked fake go: refused, naming it', refused(landIn({ onMain: `${FAKE_DIR} && ln -s tools-fake .tools` }), /^main carries \.tools;/), 'refused/true'],
  ['checkout .tools: main ships a force-tracked .tools/bin/go: refused, naming it', refused(landIn({ onMain: `mkdir -p .tools/bin && cp '${fake}' .tools/bin/go && git add -f .tools/bin/go` }), /^main carries \.tools\/bin\/go;/), 'refused/true'],
  ['checkout .tools: main ships .TOOLS/bin/go (the filesystem folds case): refused, naming it', refused(landIn({ onMain: `mkdir -p .TOOLS/bin && cp '${fake}' .TOOLS/bin/go && git add -f .TOOLS/bin/go` }), /^main carries \.TOOLS\/bin\/go;/), 'refused/true'],
  ['checkout .tools: a range commit that adds .tools/bin/go is refused, naming the commit, so it never reaches main', refused(landIn({ inPr: `mkdir -p .tools/bin && cp '${fake}' .tools/bin/go && git add -f .tools/bin/go` }), /^commit [0-9a-f]{8} in the range carries \.tools\/bin\/go;/), 'refused/true'],
  // What the checkout holds untracked: a symlinked .tools or .tools/bin pointing out of the checkout (a stat would follow it to a real
  // directory, and the go there resolves outside the checkout as a cache's does), and a go link to a tracked file inside the checkout.
  ['checkout .tools: an untracked .tools that is a symlink to a directory outside the checkout: refused', refused(landIn({ inCheckout: (d) => fs.symlinkSync(elsewhere, path.join(d, '.tools')) }), /\/\.tools is a symbolic link;/), 'refused/true'],
  ['checkout .tools: an untracked .tools/bin that is a symlink to a directory outside the checkout: refused', refused(landIn({ inCheckout: (d) => { fs.mkdirSync(path.join(d, '.tools')); fs.symlinkSync(path.join(elsewhere, 'bin'), path.join(d, '.tools', 'bin')); } }), /\/\.tools\/bin is a symbolic link;/), 'refused/true'],
  ['checkout .tools: .tools/bin/go linked to a tracked fake go inside the checkout but outside .tools: refused', refused(landIn({ onMain: FAKE_DIR, inCheckout: (d) => { fs.mkdirSync(path.join(d, '.tools', 'bin'), { recursive: true }); fs.symlinkSync('../../tools-fake/bin/go', path.join(d, '.tools', 'bin', 'go')); } }), /inside the checkout but outside \.tools;/), 'refused/true'],
  // The honest layouts land: a token is a hit and a harmless range is clean, with the checkout's own go.
  ['checkout .tools: the shared-cache layout (go an absolute link out of the checkout) finds the token', landIn({ inCheckout: cacheTools }).result, 'hit'],
  ['checkout .tools: the shared-cache layout reads a harmless range as clean', landIn({ inCheckout: cacheTools, text: 'nothing here' }).result, 'clean'],
  ['checkout .tools: the init.sh layout (go under .tools/go-<version>) finds the token', landIn({ inCheckout: ownTools }).result, 'hit'],
  ['checkout .tools: the init.sh layout reads a harmless range as clean', landIn({ inCheckout: ownTools, text: 'nothing here' }).result, 'clean'],
  // A directory named .gitleaks.toml makes gitleaks load `.gitleaks.<ext>` beside it (json first, then yaml and the rest), in any case. In a
  // tree land checks, each is refused, naming the path; land's own --config means the same files merely present in the checkout are not read
  // (the steering cases in check-scan-history.mjs). A .gitleaksignore is read whatever the flags, so the checkout may not hold one at all.
  ['checkout config: main ships a directory .gitleaks.toml/ beside a .gitleaks.json: refused, naming a path of either', refused(landIn({ onMain: `mkdir .gitleaks.toml && echo x > .gitleaks.toml/keep && printf '${ALLOW}' > .gitleaks.json` }), /^main carries \.gitleaks\.(json|toml\/keep),/), 'refused/true'],
  ['checkout config: main ships a directory .gitleaks.toml/ beside a .gitleaks.yaml: refused', refused(landIn({ onMain: `mkdir .gitleaks.toml && echo x > .gitleaks.toml/keep && printf '${ALLOW}' > .gitleaks.yaml` }), /^main carries \.gitleaks\./), 'refused/true'],
  ['checkout config: main ships .GITLEAKS.TOML/ beside .GITLEAKS.JSON (the filesystem folds case): refused', refused(landIn({ onMain: `mkdir .GITLEAKS.TOML && echo x > .GITLEAKS.TOML/keep && printf '${ALLOW}' > .GITLEAKS.JSON` }), /^main carries \.GITLEAKS\./), 'refused/true'],
  ['checkout config: a range commit that adds .gitleaks.toml/ and .gitleaks.json is refused, naming the commit', refused(landIn({ inPr: `mkdir .gitleaks.toml && echo x > .gitleaks.toml/keep && printf '${ALLOW}' > .gitleaks.json` }), /^commit [0-9a-f]{8} in the range carries \.gitleaks\./), 'refused/true'],
  ['checkout config: a .gitleaksignore in the checkout, in no tree, that lists the token\'s finding: refused before the scan', refused(landIn({ inCheckout: (d) => fs.writeFileSync(path.join(d, '.gitleaksignore'), 't.txt:generic-api-key:1\n') }), /\/\.gitleaksignore exists;/), 'refused/true'],
];
fs.rmSync(dir0, { recursive: true, force: true });
const bad = cases.filter(([, got, want]) => got !== want).map(([name, got]) => `${name} (got ${got})`);
if (bad.length) { console.error(`check-scan-checkout: ${bad.length} of ${cases.length} wrong\n  ${bad.join('\n  ')}`); process.exit(1); }
console.log(`check-scan-checkout: ${cases.length} cases ok`);
