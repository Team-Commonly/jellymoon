#!/usr/bin/env bun
// Tests the range secret scan's pure decisions: how its result is read (scanResult), the environment it runs in (scanEnv), main's own
// scan configuration (checkScanConfig) and land-scan.mjs (the argv, the paths gitleaks cannot parse). No network. Own step in `make test`.
import { checkScanConfig, foldName, scanEnv, scanResult, textCommits } from './land-decide.mjs';
import { backslashPaths, GITLEAKS_MODULE, SCAN_CONFIG, scanArgv, touchedPaths } from './land-scan.mjs';

const CLEAN = '10:11AM INF 3 commits scanned.\n10:11AM INF no leaks found\n'; // what a clean gitleaks run prints to stderr

const cases = [
  // the dry run
  ['scan result: exit 0, no leaks found and the expected count is clean', scanResult({ status: 0, stderr: CLEAN, expected: 3 }), 'clean'],
  ['scan result: gitleaks colour codes are stripped before reading', scanResult({ status: 0, stderr: '\x1b[90m10:11AM\x1b[0m \x1b[32mINF\x1b[0m \x1b[1m3 commits scanned.\x1b[0m\n10:11AM INF \x1b[1mno leaks found\x1b[0m\n', expected: 3 }), 'clean'],
  ['scan result: an empty range scanned as 0 commits is clean when 0 are expected', scanResult({ status: 0, stderr: '10:11AM INF 0 commits scanned.\n10:11AM INF no leaks found\n', expected: 0 }), 'clean'],
  ['scan result: a nonzero exit with the hit line is a hit', scanResult({ status: 2, stderr: 'WRN leaks found: 1\nexit status 3\nmake: *** Error 1\n', expected: 3 }), 'hit'],
  ['scan result: exit 0 that printed the hit line (a swallowed failure) is a failure, not clean', scanResult({ status: 0, stderr: CLEAN + 'exit status 3\n', expected: 3 }), 'failure'],
  ['scan result: a nonzero exit without the hit line is a failure', scanResult({ status: 2, stderr: 'go: no network\nexit status 1\n', expected: 3 }), 'failure'],
  ['scan result: no exit status at all is a failure, whatever the stderr says', scanResult({ status: null, stderr: 'exit status 3\n', expected: 3 }), 'failure'],
  ['scan result: a null status with a clean-looking stderr is a failure (status === 0, not !status)', scanResult({ status: null, stderr: CLEAN, expected: 3 }), 'failure'],
  ['scan result: a null status with an empty stderr is a failure', scanResult({ status: null, stderr: '', expected: 0 }), 'failure'],
  ['scan result: an undefined status is a failure', scanResult({ stderr: CLEAN, expected: 3 }), 'failure'],
  ['scan result: a zero exit with an empty stderr is a failure (nothing proves a scan ran)', scanResult({ status: 0, stderr: '', expected: 3 }), 'failure'],
  ['scan result: the hit line must be a whole line', scanResult({ status: 2, stderr: 'not exit status 30\n', expected: 3 }), 'failure'],
  ['scan result: a zero exit with an ERR log line is a failure', scanResult({ status: 0, stderr: '10:11AM ERR failed to read an object\n' + CLEAN, expected: 3 }), 'failure'],
  ['scan result: a zero exit with a FTL log line is a failure', scanResult({ status: 0, stderr: '10:11AM FTL boom\n' + CLEAN, expected: 3 }), 'failure'],
  ['scan result: a zero exit with "failed to scan" is a failure', scanResult({ status: 0, stderr: 'could not read: failed to scan the repo\n' + CLEAN, expected: 3 }), 'failure'],
  ['scan result: a zero exit without "no leaks found" is a failure', scanResult({ status: 0, stderr: '10:11AM INF 3 commits scanned.\n', expected: 3 }), 'failure'],
  ['scan result: a zero exit without the commit count line is a failure', scanResult({ status: 0, stderr: '10:11AM INF no leaks found\n', expected: 3 }), 'failure'],
  ['scan result: a scanned count below the expected one is a failure', scanResult({ status: 0, stderr: CLEAN, expected: 4 }), 'failure'],
  ['scan result: a scanned count above the expected one is a failure too (exact, not at least)', scanResult({ status: 0, stderr: CLEAN, expected: 2 }), 'failure'],
  ['scan result: no expected count is a failure', scanResult({ status: 0, stderr: CLEAN }), 'failure'],
  ['scan result: a non-numeric expected count is a failure', scanResult({ status: 0, stderr: CLEAN, expected: NaN }), 'failure'],
  ['scan result: two count lines are a failure', scanResult({ status: 0, stderr: CLEAN + '10:11AM INF 3 commits scanned.\n', expected: 3 }), 'failure'],
  ['scan result: a count that is a string is a failure (the count must be a number)', scanResult({ status: 0, stderr: CLEAN, expected: '3' }), 'failure'],
  ['scan result: a negative expected count is a failure', scanResult({ status: 0, stderr: CLEAN, expected: -1 }), 'failure'],
  ['scan result: gitleaks\' own "no leaks found in partial scan" warning is not the clean line', scanResult({ status: 0, stderr: '10:11AM INF 3 commits scanned.\n10:11AM WRN no leaks found in partial scan\n', expected: 3 }), 'failure'],
  ['scan result: the count text inside a longer line does not count', scanResult({ status: 0, stderr: '10:11AM INF no leaks found\nnote: 10:11AM INF 3 commits scanned.\n', expected: 3 }), 'failure'],
  ['scan result: the count text with a trailing word does not count', scanResult({ status: 0, stderr: '10:11AM INF no leaks found\n10:11AM INF 3 commits scanned. (partial)\n', expected: 3 }), 'failure'],
  ['scan result: a count that merely ends in the expected digits does not count', scanResult({ status: 0, stderr: '10:11AM INF no leaks found\n10:11AM INF 13 commits scanned.\n', expected: 3 }), 'failure'],
  ['scan result: the "no leaks found" text inside a longer line is not the clean line', scanResult({ status: 0, stderr: '10:11AM INF 3 commits scanned.\nprefix 10:11AM INF no leaks found\n', expected: 3 }), 'failure'],
  ['scan env: exactly the allowlist passes, Go is pinned to the installed toolchain with no workspace, and git config is pinned off', JSON.stringify(Object.entries(scanEnv({ MAKEFLAGS: 'i', GITLEAKS_CONFIG: 'x', GIT_DIR: 'g', GOFLAGS: '-x', PATH: '/p', HOME: '/h', TMPDIR: '/t', GOPATH: 'a', GOCACHE: 'b', GOMODCACHE: 'c', GOTOOLCHAIN: 'go9.9', GOWORK: '/w' })).sort()), JSON.stringify(Object.entries({ PATH: '/p', HOME: '/h', TMPDIR: '/t', GOPATH: 'a', GOCACHE: 'b', GOMODCACHE: 'c', GOTOOLCHAIN: 'local', GOWORK: 'off', GIT_CONFIG_GLOBAL: '/dev/null', GIT_CONFIG_NOSYSTEM: '1' }).sort())],
  ['scan env: only the names that are set pass', JSON.stringify(Object.entries(scanEnv({ PATH: '/p' })).sort()), JSON.stringify(Object.entries({ PATH: '/p', GOTOOLCHAIN: 'local', GOWORK: 'off', GIT_CONFIG_GLOBAL: '/dev/null', GIT_CONFIG_NOSYSTEM: '1' }).sort())],
  ['scan env: GIT_CONFIG_GLOBAL and GIT_CONFIG_NOSYSTEM set by the operator are overridden, not taken', JSON.stringify([scanEnv({ GIT_CONFIG_GLOBAL: '/etc/operator-gitconfig', GIT_CONFIG_NOSYSTEM: '0' }).GIT_CONFIG_GLOBAL, scanEnv({ GIT_CONFIG_GLOBAL: '/etc/operator-gitconfig', GIT_CONFIG_NOSYSTEM: '0' }).GIT_CONFIG_NOSYSTEM]), JSON.stringify(['/dev/null', '1'])],
  // land runs gitleaks itself: the argv is built from constants, never read from the Makefile
  ['scan argv: exactly the pinned command, one element per argument (an array, no shell string), with land\'s own config and an empty ignore directory', JSON.stringify(scanArgv('A..B', '/x/c.toml', '/x/empty')), JSON.stringify(['run', 'github.com/zricethezav/gitleaks/v8@v8.30.1', 'git', '--log-opts=-m A..B', '--ignore-gitleaks-allow', '--exit-code', '3', '--redact', '--no-banner', '--config', '/x/c.toml', '--gitleaks-ignore-path', '/x/empty', '.'])],
  ['scan config text: gitleaks\' built-in rules and nothing else', SCAN_CONFIG, '[extend]\nuseDefault = true\n'],
  ['scan argv: the module is the pinned version', GITLEAKS_MODULE, 'github.com/zricethezav/gitleaks/v8@v8.30.1'],
  ['text commits: only the count is returned', Object.keys(textCommits('\u0001' + 'a'.repeat(40) + '\n@@ -0,0 +1 @@\n+x\n', ['a'.repeat(40)])).join(), 'expected'],
  // the paths a range touches: `git log -m --no-renames --format=%x01%H --name-status -z`, read by position
  ...(() => {
    const [X, W] = ['a'.repeat(40), 'b'.repeat(40)];
    const log = (...recs) => recs.map(([sha, ...rest]) => `\u0001${sha}\0${rest.length ? '\n' : ''}${rest.map((r) => `${r}\0`).join('')}`).join('');
    const bs = (text) => backslashPaths(text).map((p) => `${p.sha.slice(0, 1)}:${p.path}`).join();
    return [
      ['touched paths: each status is followed by its path, across commits', touchedPaths(log([X, 'A', 'b\\', 'M', 'c.txt'], [W, 'D', 'old.txt'])).map((p) => `${p.sha.slice(0, 1)}:${p.path}`).join(), 'a:b\\,a:c.txt,b:old.txt'],
      ['touched paths: a commit with no files lists nothing and does not shift the next', touchedPaths(log([X], [W, 'A', 'f.txt'])).map((p) => p.path).join(), 'f.txt'],
      ['touched paths: both sides of a rename or copy are listed (a status R or C carries two paths)', touchedPaths(log([X, 'R100', 'old\\', 'new.txt', 'C075', 'src.txt', 'dup\\'])).map((p) => p.path).join(), 'old\\,new.txt,src.txt,dup\\'],
      ['touched paths: a path that looks like a commit header is a path, not a header', touchedPaths(log([X, 'A', `\u0001${W}`])).map((p) => `${p.sha.slice(0, 1)}:${p.path.slice(1, 2)}`).join(), 'a:b'],
      ['backslash paths: a path ending in a backslash is found, naming its commit', bs(log([X, 'A', 'b\\'])), 'a:b\\'],
      ['backslash paths: a backslash in the middle of a name is fine', bs(log([X, 'A', 'b\\x.txt'])), ''],
      ['backslash paths: the deleted side of a change counts', bs(log([X, 'D', 'b\\'])), 'a:b\\'],
      ['backslash paths: either side of a rename counts', bs(log([X, 'R100', 'b\\', 'new.txt'])) + '|' + bs(log([X, 'R100', 'old.txt', 'b\\'])), 'a:b\\|a:b\\'],
      ['backslash paths: the second commit is named when the first is fine', bs(log([X, 'A', 'ok.txt'], [W, 'M', 'b\\'])), 'b:b\\'],
      ['backslash paths: a log with no files is fine', bs(log([X])), ''],
    ];
  })(),
  // main's own scan configuration: refused unless the blob is one this repo ships (it ships none)
  ...(() => {
    const A = 'a'.repeat(40);
    const tree = (...paths) => paths.map((p) => `100644 blob ${A}\t${p}\0`).join('');
    const attrs = (text, shipped) => checkScanConfig(tree('docs/x.md', '.gitattributes'), () => text, shipped);
    const named = (path, shipped) => checkScanConfig(tree('README.md', path), () => { throw new Error('a blob read for a file that is not .gitattributes'); }, shipped);
    return [
      ['scan config: a tree with none of the files is accepted', named('src/a.mjs').ok, true],
      ['scan config: a .gitleaks.toml at the root is refused, naming it', named('.gitleaks.toml').reason?.includes('main carries .gitleaks.toml'), true],
      ['scan config: a .gitleaksignore at the root is refused, naming it', named('.gitleaksignore').reason?.includes('main carries .gitleaksignore'), true],
      ['scan config: a nested .gitleaksignore is refused, naming its path', named('sub/dir/.gitleaksignore').reason?.includes('sub/dir/.gitleaksignore'), true],
      ['scan config: the name compares case-insensitively', named('.GitLeaks.TOML').ok, false],
      ['scan config: a .gitattributes with `-diff` is refused', attrs('*.md text\n* -diff\n').ok, false],
      ['scan config: a .gitattributes with `binary` is refused', attrs('*.lock binary\n').ok, false],
      ['scan config: a .gitattributes with `!diff` is refused', attrs('docs/** !diff\n').ok, false],
      ['scan config: a .gitattributes with a negative pattern is refused', attrs('!*.md text\n').ok, false],
      ['scan config: a macro that switches diff off is refused', attrs('[attr]quiet -diff\n*.x quiet\n').ok, false],
      ['scan config: a tab-separated `-diff` is refused', attrs('*.md\t-diff\n').ok, false],
      ['scan config: a .gitattributes that only sets text and eol is accepted', attrs('# -diff is not here\n* text=auto eol=lf\n*.sh text\n\n').ok, true],
      ['scan config: a comment that mentions -diff is not a line that switches it', attrs('# * -diff\n').ok, true],
      ['scan config: the refusal names the .gitattributes path', checkScanConfig(tree('web/.gitattributes'), () => '* binary').reason.includes('main carries web/.gitattributes'), true],
      ['scan config: a file whose blob is the one this repo ships is accepted', named('.gitleaks.toml', { '.gitleaks.toml': A }).ok, true],
      ['scan config: a file at a shipped path with another blob is refused', checkScanConfig(`100644 blob ${'b'.repeat(40)}\t.gitleaks.toml\0`, () => '', { '.gitleaks.toml': A }).ok, false],
      ['scan config: a shipped blob at another path is refused', checkScanConfig(tree('other/.gitleaks.toml'), () => '', { '.gitleaks.toml': A }).ok, false],
      ['scan config: this repo ships none, so none is allowed', checkScanConfig(tree('.gitleaks.toml'), () => '').ok, false],
      // a tracked .tools: land runs go from <checkout>/.tools, so no tree land checks may carry one, in any form, whatever the blob
      ['scan config: a tracked .tools entry (a symlink, no slash) is refused, naming it', checkScanConfig(`120000 blob ${A}\t.tools\0`, () => '').reason?.startsWith('main carries .tools;'), true],
      ['scan config: a force-tracked .tools/bin/go is refused, naming it', named('.tools/bin/go').reason?.startsWith('main carries .tools/bin/go;'), true],
      ['scan config: .TOOLS and .Tools are refused (the filesystem folds case)', [named('.TOOLS/bin/go').ok, named('.Tools').ok].join(), 'false,false'],
      ['scan config: .tool\u017f (a long s, which APFS folds to s) is refused', named('.tool\u017f/bin/go').ok, false],
      ['scan config: a .tools listed as shipped is still refused (no blob is allowed)', checkScanConfig(tree('.tools/bin/go'), () => '', { '.tools/bin/go': A }).ok, false],
      ['scan config: only the first component counts (.toolsrc, my.tools/x and sub/.tools/x pass here; check-portable refuses the last)', [named('.toolsrc').ok, named('my.tools/x').ok, named('sub/.tools/x').ok].join(), 'true,true,true'],
      ['scan config: the refusal names the tree it was found in', checkScanConfig(tree('.tools/bin/go'), () => '', undefined, 'commit abcd1234 in the range').reason?.startsWith('commit abcd1234 in the range carries .tools/bin/go;'), true],
      // a directory named .gitleaks.toml makes gitleaks load `.gitleaks.<ext>` beside it: any component with either name, and any top-level
      // entry gitleaks could take for a config, is refused
      ['scan config: a directory named .gitleaks.toml (a path through it) is refused, naming the path', named('.gitleaks.toml/keep').reason?.startsWith('main carries .gitleaks.toml/keep,'), true],
      ['scan config: .GITLEAKS.TOML/keep is refused (folded)', named('.GITLEAKS.TOML/keep').ok, false],
      ['scan config: a .gitleaks.toml or .gitleaksignore component at any depth is refused', [named('docs/.gitleaks.toml/x').ok, named('a/.GitleaksIgnore/b').ok].join(), 'false,false'],
      ['scan config: a top-level .gitleaks.json, .gitleaks.yaml, .gitleaks and .Gitleaks.YML are refused', ['.gitleaks.json', '.gitleaks.yaml', '.gitleaks', '.Gitleaks.YML'].map((p) => named(p).ok).join(), 'false,false,false,false'],
      ['scan config: a top-level directory named .gitleaks.json is refused', named('.gitleaks.json/x').ok, false],
      ['scan config: a nested .gitleaks.json and a top-level .gitleaksrc pass (gitleaks discovers neither)', [named('docs/.gitleaks.json').ok, named('.gitleaksrc').ok].join(), 'true,true'],
      ['fold: case, the long s, the Kelvin sign and sharp s fold the way APFS compares them', ['.ToOLS', '.tool\u017f', '\u212Aeep', 'stra\u00dfe', 'STRA\u1E9EE'].map(foldName).join(), '.tools,.tools,keep,strasse,strasse'],
    ];
  })(),
];

const bad = cases.filter(([, got, want]) => got !== want).map(([n]) => n);
if (bad.length) { console.error(`check-land-scan: ${bad.length} of ${cases.length} wrong\n  ${bad.join('\n  ')}`); process.exit(1); }
console.log(`check-land-scan: ${cases.length} cases ok`);
