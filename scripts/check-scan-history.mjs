#!/usr/bin/env bun
// The real range scan over git histories, run the way land runs it (scan-fixture.mjs): markers in history, an evil merge, a main that
// carries a build file meant to steer the scan, and every range shape. Own step in `make test`.
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { scanEnv } from './land-decide.mjs';
import { GITLEAKS_MODULE } from './land-scan.mjs';
import { gitEnv, makeIn, rangeIn, root, token } from './scan-fixture.mjs';

// A gitleaks:allow marker added and removed inside the PR's history leaves the merge result clean, yet the merge keeps both commits and
// gitleaks honours the marker per commit. The range scan ignores markers and must find what the marker hid; it can only refuse.
const hist = (marker) => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'check-hist-'));
  const g = (script) => spawnSync('bash', ['-ec', script], { cwd: dir, encoding: 'utf8', env: gitEnv });
  g(`git init -q -b main && echo base > f && git add -A && git commit -qm base && git checkout -qb pr && echo 'token = ${token}${marker ? ' # gitleaks:allow' : ''}' > t.txt && git add -A && git commit -qm add && git rm -q t.txt && git commit -qm drop`);
  const run = (a) => g(`git ${a.map((x) => `'${x}'`).join(' ')}`).stdout.trim();
  const [tip, head] = [run(['rev-parse', 'main']), run(['rev-parse', 'pr'])];
  // Ignoring markers goes through land's own scan (scanRange). Honouring them is the one run land cannot make (it pins
  // --ignore-gitleaks-allow, and this run is the demonstration of the hole), so it runs gitleaks directly in the same allowlist env and
  // is read by its own exit status.
  const found = rangeIn(dir, `${tip}..${head}`).result === 'hit';
  const honoured = spawnSync(path.join(root, '.tools', 'bin', 'go'), ['run', GITLEAKS_MODULE, 'git', `--log-opts=${tip}..${head}`, '--redact', '--no-banner', dir], { encoding: 'utf8', env: scanEnv(process.env) }).status !== 0;
  fs.rmSync(dir, { recursive: true, force: true });
  return { found, foundHonouringMarker: honoured };
};
// An operator's git config that switches every diff off (core.attributesFile with `* -diff`) must reach neither the scan nor the count:
// both run in scanEnv, which pins GIT_CONFIG_GLOBAL to /dev/null. Under such a config the control must still be found, and a clean range
// must still read clean (the count is taken in the same env).
const poison = fs.mkdtempSync(path.join(os.tmpdir(), 'check-poison-'));
fs.writeFileSync(path.join(poison, 'attrs'), '* -diff\n');
fs.writeFileSync(path.join(poison, 'gitconfig'), `[core]\n\tattributesFile = ${path.join(poison, 'attrs')}\n`);
const poisoned = (f) => {
  const was = process.env.GIT_CONFIG_GLOBAL;
  process.env.GIT_CONFIG_GLOBAL = path.join(poison, 'gitconfig');
  try { return f(); } finally { if (was === undefined) delete process.env.GIT_CONFIG_GLOBAL; else process.env.GIT_CONFIG_GLOBAL = was; }
};
const [marked, control, poisonedControl] = [hist(true), hist(false), poisoned(() => hist(false))];
const history = [
  ['history: with the operator\'s git config hiding every diff, the control is still found by the scan and by the honouring run (both run in scanEnv)', poisonedControl.found && poisonedControl.foundHonouringMarker, true],
  ['history: ignoring markers, gitleaks finds the hidden token', marked.found, true],
  ['history: honouring the marker, gitleaks sees nothing (the hole)', marked.foundHonouringMarker, false],
  ['history: the control without a marker is caught by gitleaks', control.found && control.foundHonouringMarker, true],
];
const badHist = history.filter(([, got, want]) => got !== want).map(([n]) => n);
if (badHist.length) { console.error(`check-scan-history: history cases wrong\n  ${badHist.join('\n  ')}`); process.exit(1); }
console.log(`check-scan-history: ${history.length} history cases ok`);

// An evil merge: gitleaks runs `git log -p` without -m, so a secret that exists only in a merge commit's own diff (a conflict resolution)
// is never scanned. Both targets pass -m. A trusted branch merges a side branch and adds a token to docs/a.md in the resolution. The
// shape is land's: make secrets runs on the detached head (the preflight's merged tree), the range scan from main with the head by SHA.
{
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'check-evil-'));
  const g = (script) => spawnSync('bash', ['-ec', script], { cwd: dir, encoding: 'utf8', env: gitEnv });
  const evil = (add) => `git init -q -b main . && mkdir docs && echo base > docs/a.md && git add -A && git commit -qm base && git checkout -qb side && echo side > docs/s.md && git add -A && git commit -qm side && git checkout -q main && git checkout -qb pr && echo mine > docs/m.md && git add -A && git commit -qm mine && git merge -q --no-ff --no-commit side && ${add ? `echo 'token = ${token}' >> docs/a.md && git add -A && ` : ''}git commit -qm merge`;
  const run = (add) => {
    fs.rmSync(dir, { recursive: true, force: true });
    fs.mkdirSync(dir);
    g(`cp '${root}/Makefile' . && ${evil(add)}`);
    const head = g('git rev-parse pr').stdout.trim();
    g(`git checkout -q --detach ${head}`);
    const tree = makeIn(dir, 'secrets');
    const tip = g('git checkout -q main && git branch -qD pr side && git rev-parse main').stdout.trim();
    return { tree, range: rangeIn(dir, `${tip}..${head}`).result };
  };
  const [evilRun, cleanRun] = [run(true), run(false)];
  fs.rmSync(dir, { recursive: true, force: true });
  const evilCases = [
    ['evil merge: make secrets finds a token only in a merge commit', evilRun.tree !== 0, true],
    ['evil merge: land\'s range scan finds it too, read as a hit (exit status 3 and a nonzero exit), from main with the head by SHA', evilRun.range, 'hit'],
    ['evil merge: the same history without the token is clean under both', cleanRun.tree === 0 && cleanRun.range === 'clean', true],
  ];
  const badEvil = evilCases.filter(([, got, want]) => got !== want).map(([n]) => n);
  if (badEvil.length) { console.error(`check-scan-history: evil-merge cases wrong\n  ${badEvil.join('\n  ')}`); process.exit(1); }
  console.log(`check-scan-history: ${evilCases.length} evil-merge cases ok`);
}

// A main that carries a build file meant to steer the scan: land does not run make, so each of these reads exactly like the plain repo. The PR's
// one commit adds a.txt (harmless) and b.txt (a token); main's tip (committed after the range, so the range is unchanged) carries the variant, and
// the scan runs the way land runs it. Every one must be a hit: with make on the path, each of the first eight read CLEAN with the token unread.
// No tree land checks carries the variant, so these show what the checkout's files alone can do; the last four read clean without --config.
{
  const MK = fs.readFileSync(path.join(root, 'Makefile'), 'utf8');
  const EXCL = ' -- . :!b.txt';
  const wrap = '#!/bin/sh\nshift\nexec /bin/sh -c "$(printf \'%s\' "$1" | sed \'s/-m \\([^"]*\\)"/-m \\1 -- . :!b.txt"/\')"\n';
  const cfg = '[extend]\nuseDefault = true\n[allowlist]\npaths = ["b.txt"]\n';
  const variants = [
    ['the real Makefile', { Makefile: MK }],
    ['an override of a built variable name', { Makefile: `R := R\n${MK}\noverride $(R)ANGE := $(RANGE)${EXCL}\n` }],
    ['an $(eval) of a built string', { Makefile: `${MK}\n$(eval override $(subst X,,RXANGE) := $$(RANGE)${EXCL})\n` }],
    ['an include of a tracked file that overrides RANGE', { Makefile: `${MK}\ninclude scan.mk\n`, 'scan.mk': `override RANGE := $(RANGE)${EXCL}\n` }],
    ['a tracked GNUmakefile that includes the Makefile and overrides RANGE', { Makefile: MK, GNUmakefile: `include Makefile\noverride RANGE := $(RANGE)${EXCL}\n` }],
    ['SHELL set to a tracked wrapper that rewrites the recipe', { Makefile: `${MK}\nSHELL := ./scan-shell\n`, 'scan-shell': wrap }],
    ['a built target name whose prerequisite writes .gitleaks.toml', { Makefile: `S := secrets\n${MK}\n$(S)-range: cfg\ncfg:\n\t@printf '${cfg.replace(/\n/g, '\\n')}' > .gitleaks.toml\n` }],
    ['an exported GITLEAKS_CONFIG naming a tracked file', { Makefile: `${MK}\nexport GITLEAKS_CONFIG := scan.toml\n`, 'scan.toml': cfg }],
    ['exported GIT_CONFIG_* pointing core.attributesFile at a tracked -diff file', { Makefile: `${MK}\nexport GIT_CONFIG_COUNT := 1\nexport GIT_CONFIG_KEY_0 := core.attributesFile\nexport GIT_CONFIG_VALUE_0 := scan.attrs\n`, 'scan.attrs': 'b.txt -diff\n' }],
    ['a tracked go.work with a newer go line and a toolchain line', { 'go.work': 'go 1.99\ntoolchain go1.99.0\nuse ./nope\n' }],
    ['a tracked go.mod with a toolchain line and a replace of the scanner', { 'go.mod': 'module x\ngo 1.99\ntoolchain go1.99.0\nreplace github.com/zricethezav/gitleaks/v8 => ./nope\n' }],
    // go started through the .tools/bin/go link looks three levels up for its GOROOT, which is the checkout; land runs the resolved go with GOROOT set
    ['a tracked pkg/tool/ and go.env at the top, which would make the checkout the GOROOT of a go started through the .tools/bin link', { 'pkg/tool/README': 'x\n', 'go.env': 'GOPROXY=off\nGOFLAGS=-mod=mod\n' }],
    // gitleaks looks for a config only without --config: a .gitleaks.toml, or a directory of that name, which makes it load `.gitleaks.<ext>`
    ['a .gitleaks.toml that allowlists the token\'s file', { '.gitleaks.toml': cfg }],
    ['a directory .gitleaks.toml/ beside a .gitleaks.json that allowlists it', { '.gitleaks.toml/keep': 'x\n', '.gitleaks.json': cfg }],
    ['a directory .gitleaks.toml/ beside a .gitleaks.yaml that allowlists it', { '.gitleaks.toml/keep': 'x\n', '.gitleaks.yaml': cfg }],
    ['.GITLEAKS.TOML/ beside .GITLEAKS.JSON (the filesystem folds case)', { '.GITLEAKS.TOML/keep': 'x\n', '.GITLEAKS.JSON': cfg }],
  ];
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'check-steer-'));
  const g = (script) => spawnSync('bash', ['-ec', script], { cwd: dir, encoding: 'utf8', env: gitEnv });
  const steerCases = variants.map(([name, files]) => {
    fs.rmSync(dir, { recursive: true, force: true });
    fs.mkdirSync(dir);
    g('git init -q -b main . && echo base > notes.txt && git add -A && git commit -qm base');
    fs.writeFileSync(path.join(dir, 'a.txt'), 'harmless\n');
    fs.writeFileSync(path.join(dir, 'b.txt'), `token = ${token}\n`);
    g('git add -A && git commit -qm pr');
    const range = `${g('git rev-parse HEAD~1').stdout.trim()}..${g('git rev-parse HEAD').stdout.trim()}`;
    Object.entries(files).forEach(([f, text]) => { fs.mkdirSync(path.dirname(path.join(dir, f)), { recursive: true }); fs.writeFileSync(path.join(dir, f), text, { mode: f === 'scan-shell' ? 0o755 : 0o644 }); });
    g('git add -A && git commit -qm main');
    return [`land's scan: ${name} on main changes nothing, the token is found`, rangeIn(dir, range).result, 'hit'];
  });
  fs.rmSync(dir, { recursive: true, force: true });
  const badSteer = steerCases.filter(([, got, want]) => got !== want).map(([n]) => n);
  if (badSteer.length) { console.error(`check-scan-history: steering cases wrong\n  ${badSteer.join('\n  ')}`); process.exit(1); }
  console.log(`check-scan-history: ${steerCases.length} steering cases ok`);
}

// The range scan reads every commit of <main tip>..<head> through land's own scan (scanRange), markers ignored, in the shape of a normal land: the
// checkout is main, and the PR head exists as a SHA (land fetches it to FETCH_HEAD). Each shape runs twice: with no ref at the head, and
// with one (a local branch and a refs/remotes ref, which any later git fetch leaves behind). A token added then deleted in the range is
// still in history and must be found whatever shape the history has, and a ref holding the head must not hide the range from the scan.
{
  // Shapes whose commits gitleaks does not count (it counts a commit only when its diff has a hunk in a file git does not show as deleted):
  // name -> [setup on a branch off main, 'rev-list/gitleaks/expected']. The three numbers are measured; land's expected must equal gitleaks'.
  const C = 'git add -A && git commit -qm x';
  const bare = {
    empty: ['git commit -q --allow-empty -m e', '1/0/0'],
    rename: [`git mv notes.txt moved.txt && ${C}`, '1/0/0'],
    modeonly: [`chmod +x notes.txt && ${C}`, '1/0/0'],
    deleteonly: [`echo a > f.txt && ${C} && git rm -q f.txt && ${C}`, '2/1/1'],
    revertadd: [`echo a > f.txt && ${C} && git revert --no-edit HEAD`, '2/1/1'],
    emptyfile: [`: > e.txt && ${C}`, '1/0/0'],
    binary: [`printf '\\000\\001\\002' > b.bin && ${C}`, '1/0/0'],
    archive: [`echo x > in.txt && tar czf a.tar.gz in.txt && rm in.txt && ${C}`, '1/0/0'],
    lineremoval: [`printf 'a\\nb\\n' > notes.txt && ${C} && printf 'a\\n' > notes.txt && ${C}`, '2/2/2'],
    renameedit: [`git mv notes.txt moved.txt && echo more >> moved.txt && ${C}`, '1/1/1'],
    typechange: [`rm notes.txt && ln -s elsewhere notes.txt && ${C}`, '1/1/1'],
    mixed: [`git commit -q --allow-empty -m e && echo more >> notes.txt && ${C} && git mv notes.txt n2.txt && ${C}`, '3/1/1'],
    mergeEmptyOne: [`echo mine > mine.txt && ${C} && git checkout -qb side main && git commit -q --allow-empty -m s && git checkout -q pr && git merge -q --no-ff --no-edit side`, '3/2/2'],
    mergeEmptyBoth: [`git commit -q --allow-empty -m a && git checkout -qb side main && git commit -q --allow-empty -m b && git checkout -q pr && git merge -q --no-ff --no-edit side`, '3/0/0'],
    octopusEmpty: [`git commit -q --allow-empty -m a && git checkout -qb s1 main && git commit -q --allow-empty -m b && git checkout -qb s2 main && git commit -q --allow-empty -m c && git checkout -q pr && git merge -q --no-ff --no-edit s1 s2`, '4/0/0'],
  };
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'check-range-'));
  const g = (script) => spawnSync('bash', ['-ec', script], { cwd: dir, encoding: 'utf8', env: gitEnv });
  let plain = false; // the same shapes with the token replaced by harmless text, to read the scanned count on clean merges
  const tok = (marker) => `echo '${plain ? 'nothing here' : `token = ${token}${marker ? ' # gitleaks:allow' : ''}`}'`;
  const shapes = {
    linear: (m) => `git checkout -qb pr && ${tok(m)} > notes.txt && git commit -qam add && echo base > notes.txt && git commit -qam drop`,
    // the token lives only on a side branch (added then deleted there) that the PR merges
    sidebranch: (m) => `git checkout -qb pr && echo mine > mine.txt && git add -A && git commit -qm mine && git checkout -qb side main && ${tok(m)} > side.txt && git add -A && git commit -qm add && git rm -q side.txt && echo s > s.txt && git add -A && git commit -qm drop && git checkout -q pr && git merge -q --no-ff --no-edit side`,
    // main gains a commit after the PR branched
    oldbase: (m) => `git checkout -qb pr && ${tok(m)} > notes.txt && git commit -qam add && echo base > notes.txt && git commit -qam drop && git checkout -q main && echo next > next.txt && git add -A && git commit -qm next`,
    // an octopus merge of two side branches, the token added then deleted on one of them
    octopus: (m) => `git checkout -qb pr && echo mine > mine.txt && git add -A && git commit -qm mine && git checkout -qb s1 main && ${tok(m)} > s1.txt && git add -A && git commit -qm add && git rm -q s1.txt && echo s > s.txt && git add -A && git commit -qm drop && git checkout -qb s2 main && echo t > t.txt && git add -A && git commit -qm t && git checkout -q pr && git merge -q --no-ff --no-edit s1 s2`,
    // main moved after the PR branched, and the PR merged that tip in
    mainmerged: (m) => `git checkout -qb pr && ${tok(m)} > notes.txt && git commit -qam add && echo base > notes.txt && git commit -qam drop && git checkout -q main && echo next > next.txt && git add -A && git commit -qm next && git checkout -q pr && git merge -q --no-ff --no-edit main`,
    clean: () => 'git checkout -qb pr && echo more > notes.txt && git commit -qam add',
    tokenAmongEmpty: (m) => `git checkout -qb pr && git commit -q --allow-empty -m e && ${tok(m)} > notes.txt && git commit -qam add && git commit -q --allow-empty -m f && echo base > notes.txt && git commit -qam drop`,
    ...Object.fromEntries(Object.entries(bare).map(([name, [script]]) => [name, () => `git checkout -qb pr && ${script}`])),
    // Paths that end in a backslash (below): gitleaks fails to parse the file name and loses the "commit" line before it. Land refuses each.
    backslashend: () => `git checkout -qb pr && ${tok(false)} > 'b\\' && ${C}`,
    plusbackslashend: () => `git checkout -qb pr && echo harmless > a.txt && ${tok(false)} > 'b\\' && ${C}`,
    // the token in b\ (older commit) with c.txt, then a commit that adds no text: gitleaks counts the newer one and attributes c.txt to it
    staleSwap: () => `git checkout -qb pr && ${tok(false)} > 'b\\' && echo harmless > c.txt && ${C} && git mv notes.txt moved.txt && ${C}`,
    staleSwapDelete: () => `git checkout -qb pr && ${tok(false)} > 'b\\' && echo harmless > c.txt && ${C} && git rm -q notes.txt && ${C}`,
    staleNewest: () => `git checkout -qb pr && ${tok(false)} > 'b\\' && echo harmless > c.txt && ${C}`,
    dropPlusText: () => `git checkout -qb pr && ${tok(false)} > 'b\\' && ${C} && echo more > other.txt && ${C}`,
    renameToBackslash: () => `git checkout -qb pr && echo aaa > old.txt && ${C} && git mv old.txt 'b\\' && ${C}`,
    renameFromBackslash: () => `${tok(false)} > 'b\\' && ${C} && git checkout -qb pr && git mv 'b\\' moved.txt && ${C}`, // main holds b\ ; the PR only renames it away
    mergeParent: () => `git checkout -qb pr && echo mine > mine.txt && ${C} && git checkout -q main && ${tok(false)} > 'b\\' && ${C} && git checkout -q pr && git merge -q --no-ff --no-edit main`, // b\ appears only in the merge's diff against the PR parent
    longMessageLine: () => `git checkout -qb pr && echo text > m.txt && git add -A && git commit -q -F <(awk 'BEGIN{printf "subject\\n\\n"; for(i=0;i<70000;i++) printf "x"; printf "\\n"}')`, // a message line over 64 KB: gitleaks reads none, land expects one, so it fails closed
    backslashmid: () => `git checkout -qb pr && ${tok(false)} > 'b\\x.txt' && ${C}`, // a backslash inside the name parses fine: the token is found
  };
  const BLOCKED = ['backslashend', 'plusbackslashend', 'staleSwap', 'staleSwapDelete', 'staleNewest', 'dropPlusText', 'renameToBackslash', 'renameFromBackslash', 'mergeParent'];
  const extras = { sidebranch: 'side', octopus: 's1 s2', mergeEmptyOne: 'side', mergeEmptyBoth: 'side', octopusEmpty: 's1 s2' };
  let last; // the makeIn result of the latest status call
  const status = (shape, marker, refs, harmless = false) => {
    plain = harmless;
    fs.rmSync(dir, { recursive: true, force: true });
    fs.mkdirSync(dir);
    g(`cp '${root}/Makefile' . && git init -q -b main . && echo base > notes.txt && git add -A && git commit -qm base && ${shapes[shape](marker)}`);
    const head = g('git rev-parse pr').stdout.trim();
    const keep = refs ? `git branch -q kept ${head} && git update-ref refs/remotes/origin/pr ${head}` : 'true';
    const tip = g(`${keep} && git checkout -q main && git branch -qD pr ${extras[shape] ?? ''} && git rev-parse main`).stdout.trim();
    last = rangeIn(dir, `${tip}..${head}`);
    return last.result;
  };
  const scanned = () => /(\d+) commits? scanned/.exec(last.stderr.replace(/\u001b\[[0-9;]*m/g, ''))?.[1];
  const rangeCases = [];
  for (const refs of [false, true]) {
    const where = refs ? 'refs at the head' : 'no ref at the head';
    for (const shape of ['linear', 'sidebranch', 'oldbase', 'octopus', 'mainmerged']) {
      rangeCases.push([`range (${shape}, ${where}): a token added then deleted in the range is found by the recipe itself`, status(shape, false, refs), 'hit']);
      rangeCases.push([`range (${shape}, ${where}): a gitleaks:allow marker does not hide it`, status(shape, true, refs), 'hit']);
    }
    rangeCases.push([`range (${where}): a range without a token is clean`, status('clean', false, refs), 'clean']);
    for (const shape of ['linear', 'sidebranch', 'oldbase', 'octopus', 'mainmerged']) rangeCases.push([`range (${shape}, ${where}): without the token it reads clean, so gitleaks' scanned count equals the expected one`, status(shape, false, refs, true), 'clean']);
    for (const shape of refs ? [] : Object.keys(bare)) { // the ref variants above already show a ref at the head cannot hide the range
      rangeCases.push([`range (${shape}, ${where}): reads clean, so land's expected count equals what gitleaks scanned`, status(shape, false, refs, true), 'clean']);
      rangeCases.push([`range (${shape}, ${where}): rev-list/gitleaks/expected counts`, `${last.revlist}/${scanned()}/${last.expected}`, bare[shape][1]]);
    }
    for (const shape of refs ? [] : BLOCKED) rangeCases.push([`range (${shape}, ${where}): a path ending in a backslash is refused before the scan, never read as clean`, `${status(shape, false, refs)}/${/ends in a backslash/.test(last.reason)}`, 'refused/true']);
    if (!refs) rangeCases.push(['range (longMessageLine): a commit message line over 64 KB makes the counts differ, and that fails closed, never clean', status('longMessageLine', false, refs), 'failure']);
    rangeCases.push([`range (${where}): a backslash inside a name is not refused, and the token in it is found`, status('backslashmid', false, refs), 'hit']);
    rangeCases.push([`range (${where}): a token among commits that add no text is still found`, status('tokenAmongEmpty', false, refs), 'hit']);
  }
  process.env.MAKEFLAGS = 'i'; // the scan env drops it (make is not on land's path, and `make secrets-range` for people still must not be switched off)
  rangeCases.push(['range: an operator MAKEFLAGS=i does not switch the scan off', status('linear', false, false), 'hit']);
  delete process.env.MAKEFLAGS;
  rangeCases.push(['range: with the operator\'s git config hiding every diff, a clean range still reads clean (the count is taken in scanEnv)', poisoned(() => status('linear', false, false, true)), 'clean']);
  rangeCases.push(['range: with the operator\'s git config hiding every diff, a token is still found', poisoned(() => status('linear', false, false)), 'hit']);
  fs.rmSync(dir, { recursive: true, force: true });
  const badRange = rangeCases.filter(([, got, want]) => got !== want).map(([n]) => n);
  if (badRange.length) { console.error(`check-scan-history: range cases wrong\n  ${badRange.join('\n  ')}`); process.exit(1); }
  console.log(`check-scan-history: ${rangeCases.length} range cases ok`);
}
