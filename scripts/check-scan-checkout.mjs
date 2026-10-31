#!/usr/bin/env bun
// The real range scan against what a land checkout itself holds, run the way land runs it (scan-fixture.mjs): the .tools that land takes go
// from, and the gitleaks configuration a tree or the checkout could carry. Each fixture is a checkout of main whose tip carries the variant, and a PR commit off that tip that adds a token, scanned with the
// checkout's OWN .tools (never this repo's). Every variant must be refused, never clean; the honest layouts must land. Own step in `make test`.
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { cacheTools, fixtureGo, gitEnv, ownTools, rangeIn, token } from './scan-fixture.mjs';

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
// The go the checks run fixtures with (fixtureGo): <top>/.tools/bin/go first, else the first go on PATH, else an error naming both. Fixture
// tops with and without a .tools/bin/go, and PATH directories that hold an executable go (path1, path2) or none (path0).
const goIn = (d) => { fs.mkdirSync(d, { recursive: true }); fs.copyFileSync(fake, path.join(d, 'go')); fs.chmodSync(path.join(d, 'go'), 0o755); return fs.realpathSync.native(path.join(d, 'go')); };
const [withTools, noTools, path0] = ['with-tools', 'no-tools', 'path0'].map((d) => path.join(dir0, d));
const [own, path1, path2] = [path.join(withTools, '.tools', 'bin'), path.join(dir0, 'path1'), path.join(dir0, 'path2')].map(goIn);
[noTools, path0].forEach((d) => fs.mkdirSync(d));
const resolved = (top, dirs) => { try { return fixtureGo({ top, PATH: dirs.join(path.delimiter) }); } catch (e) { return e.message; } };

let n = 0;
const put = (rel, body) => (d) => { fs.mkdirSync(path.dirname(path.join(d, rel)), { recursive: true }); fs.writeFileSync(path.join(d, rel), body); };
// main's tip runs `onMain` before its commit (it may force-add what .gitignore hides); the PR runs `inPr`, then writes `text` to `file`; each
// step is a bash script or a function of the checkout directory. The checkout goes back to main (`back`) and `inCheckout(dir)` adds what the
// checkout holds untracked. Returns land's scan of <main tip>..<PR head> in that checkout, with the checkout's dir, tip and head.
const landIn = ({ onMain = 'true', inPr = 'true', inCheckout = () => {}, text = `token = ${token}`, file = 't.txt', nest = [], back = 'git checkout -q main' } = {}) => {
  const dir = path.join(dir0, `c${n += 1}`, ...nest); // nest: the checkout sits that many directories below c<n>
  fs.mkdirSync(dir, { recursive: true });
  const g = (script) => {
    const r = spawnSync('bash', ['-ec', script], { cwd: dir, encoding: 'utf8', env: gitEnv });
    if (r.status !== 0) throw new Error(`fixture setup failed: ${script}\n${r.stderr}`);
    return r.stdout.trim();
  };
  const step = (s) => (typeof s === 'function' ? s(dir) : g(s));
  g(`git init -q -b main . && printf '.tools/\\n' > .gitignore && echo base > notes.txt && git add -A && git commit -qm base`);
  step(onMain);
  g('git add -A && git commit -q --allow-empty -m main');
  const tip = g('git rev-parse HEAD');
  g('git checkout -qb pr');
  step(inPr);
  put(file, `${text}\n`)(dir);
  g(`git add -A && git commit -qm pr && ${back}`);
  const head = g('git rev-parse pr');
  inCheckout(dir);
  return Object.assign(rangeIn(dir, `${tip}..${head}`), { dir, tip, head });
};
const refused = (r, re) => `${r.result}/${re.test(r.reason ?? '')}`;
const esc = (t) => t.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const FAKE_DIR = `mkdir -p tools-fake/bin && cp '${fake}' tools-fake/bin/go`;
const ALLOW = '[extend]\\nuseDefault = true\\n[allowlist]\\npaths = ["t.txt"]\\n'; // a config that would hide the PR's t.txt, for printf

// A tracked .tools, in each form main could ship it: refused from the trees, before any go runs. The force-tracked and the upper-case forms
// pass every filesystem check (real directories, go under .tools), so only the tree refusal, case-folded, stops them.
// Tracked symbolic links (owner ruling): [what, the script that makes lnk, what the checkout holds, how the checkout goes back to main]. `out`
// is a directory outside every checkout whose attributes file switches every diff off; each link, tracked on main and replaced in the range
// by lnk/t.txt (the token), would read clean through it.
const out = path.join(dir0, 'x');
put('.gitattributes', '* -diff\n')(out);
const ln = (to) => `ln -s '${to}' lnk`;
// A tree can carry a link with an empty target, but a Linux disk cannot (GNU ln refuses one, as Linux symlink(2) does; macOS makes it). So it
// is made with plumbing and kept out of the working tree on every platform: the empty blob at mode 120000, marked skip-worktree so `git add
// -A` does not stage its removal and `git rm --sparse` can drop it. The PR's commit takes that bit with it, and a plain checkout of main
// would then make the link (macOS) or fail to (Linux), so main is checked out again through a sparse pattern that leaves lnk out.
const EMPTY = 'e69de29bb2d1d6434b8b29ae775ad8c2e48c5391'; // the empty blob
const EMPTY_LINK = 'git update-index --add --cacheinfo "120000,$(git hash-object -w --stdin </dev/null),lnk" && git update-index --skip-worktree lnk';
const LEAVE_OUT = "printf '/*\\n!/lnk\\n' > .git/info/sparse-checkout && git -c core.sparseCheckout=true -c core.sparseCheckoutCone=false checkout -q main";
const LINKS = [['an absolute link', ln(out)], ['a link to ../x', ln('../x')], ['a link into .git', ln('.git/hide'), put('.git/hide/.gitattributes', '* -diff\n')],
  ['a link into .tools', ln('.tools/x'), (d) => { cacheTools(d); put('.tools/x/.gitattributes', '* -diff\n')(d); }], ['a link with an empty target', EMPTY_LINK, undefined, LEAVE_OUT]];
const empty = {}; // the two fixtures of the empty-target link, for the tree-entry case
const lsTree = (r, rev) => spawnSync('git', ['ls-tree', rev, 'lnk'], { cwd: r.dir, encoding: 'utf8', env: gitEnv }).stdout.trim();
const onDisk = (p) => { try { fs.lstatSync(p); return 'on disk'; } catch { return 'not on disk'; } };
// The .gitattributes shapes: [path, contents, the file the PR adds the token to].
const ATTRS = [['.gitattributes', '* -diff=x\n', 'b.txt'], ['.gitattributes', 'b.txt -diff=\n', 'b.txt'], ['.gitattributes', '[attr]hide -diff=1\n* hide\n', 'b.txt'], ['.gitattributes', '\f#x -diff\n', '\f#x'],
  ['.gitattributes', '\v#x -diff\n', '\v#x'], ['.gitattributes', '\u00a0#x -diff\n', '\u00a0#x'], ['sub/.gitattributes', '* -diff\n', 'sub/t.txt'], ['.GITATTRIBUTES', '* -diff\n', 't.txt']];
const cases = [
  ['fixture go: <top>/.tools/bin/go is the one used when it exists, over a go on PATH', resolved(withTools, [path.dirname(path1)]), own],
  ['fixture go: with no .tools, the first go on PATH, by its real path (a PATH directory without one is passed over)', resolved(noTools, [path0, path.dirname(path1), path.dirname(path2)]), path1],
  ['fixture go: no .tools and no go on PATH fails, naming both', resolved(noTools, [path0]), `no go to run the fixtures with: ${path.join(noTools, '.tools', 'bin', 'go')} does not exist and there is no go on PATH`],
  ['checkout .tools: main ships .tools as a tracked symlink to a tracked fake go: refused, naming it', refused(landIn({ onMain: `${FAKE_DIR} && ln -s tools-fake .tools` }), /^main carries \.tools;/), 'refused/true'],
  ['checkout .tools: main ships a force-tracked .tools/bin/go: refused, naming it', refused(landIn({ onMain: `mkdir -p .tools/bin && cp '${fake}' .tools/bin/go && git add -f .tools/bin/go` }), /^main carries \.tools\/bin\/go;/), 'refused/true'],
  ['checkout .tools: main ships .TOOLS/bin/go (the filesystem folds case): refused, naming it', refused(landIn({ onMain: `mkdir -p .TOOLS/bin && cp '${fake}' .TOOLS/bin/go && git add -f .TOOLS/bin/go` }), /^main carries \.TOOLS\/bin\/go;/), 'refused/true'],
  ['checkout .tools: a range commit that adds .tools/bin/go is refused, naming the commit, so it never reaches main', refused(landIn({ inPr: `mkdir -p .tools/bin && cp '${fake}' .tools/bin/go && git add -f .tools/bin/go` }), /^commit [0-9a-f]{8} in the range carries \.tools\/bin\/go;/), 'refused/true'],
  // A .tools at any depth (owner ruling): .gitignore hides each one, and git opens an ignored .gitattributes there while it diffs a path under it.
  ['checkout .tools: a range adds then deletes sub/.tools/t.txt (the token) over an ignored sub/.tools/.gitattributes `* -diff`: refused, naming the commit', refused(landIn({ inPr: `mkdir -p sub/.tools && echo 'token = ${token}' > sub/.tools/t.txt && git add -f sub/.tools/t.txt && git commit -qm add && git rm -q sub/.tools/t.txt`, file: 'other.txt', text: 'nothing here', inCheckout: put('sub/.tools/.gitattributes', '* -diff\n') }), /^commit [0-9a-f]{8} in the range carries sub\/\.tools\/t\.txt;/), 'refused/true'],
  ['checkout .tools: a range commit that adds a/b/.TOOLS/x (the token) is refused, naming it', refused(landIn({ inPr: `mkdir -p a/b/.TOOLS && echo 'token = ${token}' > a/b/.TOOLS/x && git add -f a/b/.TOOLS/x`, file: 'other.txt', text: 'nothing here' }), /^commit [0-9a-f]{8} in the range carries a\/b\/\.TOOLS\/x;/), 'refused/true'],
  // The listing skips only the top-level .tools. A link main tracks into a nested one stays in the checkout, and git opens the attributes there through it.
  ['checkout .tools: main tracks lnk -> sub/.tools over an ignored sub/.tools/.gitattributes `* -diff`, the range puts the token at lnk/t.txt: the listing refuses it', refused(landIn({ onMain: 'ln -s sub/.tools lnk', inPr: 'git rm -q lnk', file: 'lnk/t.txt', inCheckout: put('sub/.tools/.gitattributes', '* -diff\n') }), /\/sub\/\.tools\/\.gitattributes exists; git reads/), 'refused/true'],
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
  ['checkout config: a .gitleaksignore in the checkout, in no tree, listing the token\'s real fingerprint (t.txt:github-pat:1, which hides it from the scan): refused before the scan', refused(landIn({ inCheckout: (d) => fs.writeFileSync(path.join(d, '.gitleaksignore'), 't.txt:github-pat:1\n') }), /\/\.gitleaksignore exists;/), 'refused/true'],
  // .gitattributes (owner ruling): land refuses every one. Each shape made the scan read a token range as clean while land parsed the lines
  // (git reads `-diff=<any value>` as -diff, expands a macro, and takes a form feed, vertical tab or U+00A0 before `#` as part of a pattern),
  // and git reads a nested or an upper-case one too. Each is refused on main, where git reads it, and in a range commit, which would bring it
  // onto main; the controls, the same ranges with no .gitattributes, find the token.
  ...ATTRS.flatMap(([at, body, file]) => [
    [`attributes: main ships ${at} holding ${JSON.stringify(body)}: refused, naming it`, refused(landIn({ onMain: put(at, body), file }), new RegExp(`^main carries ${esc(at)}; git reads`)), 'refused/true'],
    [`attributes: a range commit adds ${at} holding ${JSON.stringify(body)}: refused, naming the commit`, refused(landIn({ inPr: put(at, body), file }), new RegExp(`^commit [0-9a-f]{8} in the range carries ${esc(at)}; git reads`)), 'refused/true'],
  ]),
  ...[...new Set(ATTRS.map(([, , file]) => file))].map((file) => [`attributes control: the range with no .gitattributes finds the token in ${JSON.stringify(file)}`, landIn({ file }).result, 'hit']),
  // On the checkout's filesystem, in no tree: an untracked one at the top (also upper-case) and an ignored nested one, each hiding the token
  // from git. Under .tools, which no tree may carry a path under, one is not read and not refused: the init.sh layout with one there lands.
  ['attributes: an untracked .gitattributes at the top of the checkout, in no tree: refused, naming it', refused(landIn({ inCheckout: put('.gitattributes', '* -diff\n') }), /\/\.gitattributes exists; git reads/), 'refused/true'],
  ['attributes: an untracked .GITATTRIBUTES at the top: refused, naming it', refused(landIn({ inCheckout: put('.GITATTRIBUTES', '* -diff\n') }), /\/\.GITATTRIBUTES exists; git reads/), 'refused/true'],
  ['attributes: an ignored sub/.gitattributes (sub/ in .git/info/exclude) over the token in sub/t.txt: refused, naming it', refused(landIn({ file: 'sub/t.txt', inCheckout: (d) => { fs.appendFileSync(path.join(d, '.git', 'info', 'exclude'), 'sub/\n'); put('sub/.gitattributes', '* -diff\n')(d); } }), /\/sub\/\.gitattributes exists; git reads/), 'refused/true'],
  ['attributes: the init.sh layout with a .gitattributes inside .tools (an installed Go ships one) finds the token', landIn({ inCheckout: (d) => { ownTools(d); put('.tools/go-src/x/.gitattributes', '* -text\n')(d); } }).result, 'hit'],
  // Each link is refused lexically in main's tree and in a range commit's tree, before the scan.
  ...LINKS.flatMap(([what, make, held, back]) => {
    const [main, range] = [landIn({ onMain: make, inPr: 'git rm -q --sparse lnk', file: 'lnk/t.txt', inCheckout: held, back }), landIn({ inPr: `${make} && git add -A && git commit -qm link && git rm -q --sparse lnk`, file: 'lnk/t.txt' })];
    if (make === EMPTY_LINK) Object.assign(empty, { main, range });
    return [[`links: main tracks ${what}, the range puts the token at lnk/t.txt: refused, naming it`, refused(main, /^main carries lnk, a symbolic link whose target /), 'refused/true'],
      [`links: a range commit adds ${what}, the next puts the token at lnk/t.txt: refused, naming the commit`, refused(range, /^commit [0-9a-f]{8} in the range carries lnk, a symbolic link whose target /), 'refused/true']];
  }),
  // The empty-target link of the two cases above is a real tree entry, on main and in the range's first commit, and neither checkout holds it.
  ['links: the empty-target link is the tree entry 120000 with the empty blob, on main and in the range commit, and on neither disk', [[empty.main, empty.main.tip], [empty.range, `${empty.range.head}~1`]].map(([r, rev]) => `${lsTree(r, rev)} (${onDisk(path.join(r.dir, 'lnk'))})`).join('; '), `120000 blob ${EMPTY}\tlnk (not on disk); 120000 blob ${EMPTY}\tlnk (not on disk)`],
  // A chain passes the lexical rule (p normalises to the top) and resolves three levels above it, where an attributes file switches the diff off.
  ['links: main tracks q -> . and p -> q/q/q/../../.., the range puts the token at p/t.txt: refused on disk, naming p', refused(landIn({ nest: ['u', 'v', 'w'], onMain: 'ln -s . q && ln -s q/q/q/../../.. p', inPr: 'git rm -q p', file: 'p/t.txt', inCheckout: (d) => put('.gitattributes', '* -diff\n')(path.resolve(d, '../../..')) }), /\/p, a symbolic link main tracks, resolves to /), 'refused/true'],
  ['links control: main tracks the repo\'s own two links (CLAUDE.md -> AGENTS.md, .claude/skills -> ../.agents/skills): the token is found', landIn({ onMain: 'echo a > AGENTS.md && mkdir -p .agents/skills .claude && echo s > .agents/skills/s.md && ln -s AGENTS.md CLAUDE.md && ln -s ../.agents/skills .claude/skills' }).result, 'hit'],
  ['links control: main tracks lnk -> d, a directory in the checkout, and the range puts the token at lnk/t.txt: found', landIn({ onMain: 'mkdir d && echo d > d/keep && ln -s d lnk', inPr: 'git rm -q lnk', file: 'lnk/t.txt' }).result, 'hit'],
  ['links control: a range commit adds lnk -> d, the next puts the token at lnk/t.txt: found', landIn({ inPr: 'mkdir d && echo d > d/keep && ln -s d lnk && git add -A && git commit -qm link && git rm -q lnk', file: 'lnk/t.txt' }).result, 'hit'],
  ['links: the listing does not follow a link (an ignored ign -> x, with no range path under it): the token is found', landIn({ inCheckout: (d) => { fs.symlinkSync(out, path.join(d, 'ign')); fs.appendFileSync(path.join(d, '.git', 'info', 'exclude'), 'ign\n'); } }).result, 'hit'],
];
fs.rmSync(dir0, { recursive: true, force: true });
const bad = cases.filter(([, got, want]) => got !== want).map(([name, got]) => `${name} (got ${got})`);
if (bad.length) { console.error(`check-scan-checkout: ${bad.length} of ${cases.length} wrong\n  ${bad.join('\n  ')}`); process.exit(1); }
console.log(`check-scan-checkout: ${cases.length} cases ok`);
