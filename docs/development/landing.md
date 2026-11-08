# Landing a PR

`make land PR=<n>` is the only merge path. Local verification is the authority, except that CI may stand in for the preflight (step 5); GitHub check runs are otherwise consulted only
as a veto (step 3). It runs `scripts/land.mjs`; the decisions are pure functions in `scripts/land-decide.mjs` and the
cases are in `scripts/check-land.mjs` and `scripts/check-land-scan.mjs`; the range scan itself is `scripts/land-scan.mjs`, and the CodeRabbit path's decisions are `scripts/land-coderabbit.mjs` (cases in `scripts/check-land-coderabbit.mjs`). Run it from a checkout of main with a clean tree: the accepted reviewers come from
operator environment (`JM_VERDICT_REVIEWERS`), never from the repo. One land runs at a time **per clone**: an mkdir lock in that clone's git
common dir holds the PID, so two clones can land at once. A dead PID, or a lock dir with no pid file, is stale; it is
taken over by renaming the stale dir to a name of the taker's own (atomic, so only one taker wins) and then creating
the lock.

The stack lands as one PR retargeted to `main`. Land handles exactly one PR whose base is `main`; any other base
is refused.

## Threat model
- **The preflight is a correctness check, not a security boundary.** It runs the merge result's code on this host
  with a scrubbed environment, and that is all it claims.
- **Who is trusted, and why.** The code that runs here is written by people and agents who already had write access
  and already ran the same code on this host while writing it. So a PR runs here only if it is from this repo, not a
  fork, its author's login is in `JM_TRUSTED_LOGINS`, and every account with push access to the repo
  (`GET /repos/{repo}/collaborators?affiliation=all`, paginated, keeping `.permissions.push === true`) is in that list too. The list is an
  operator env var, comma-separated, never read from the repo, so a PR cannot widen it; unset, land refuses. Trust is
  a GitHub identity, never a commit author string, which anyone can set.
- **An outside PR is not executed on this host.** Land and `make verify-preflight` refuse it before fetching or running
  any of its code: `untrusted PR: not executed on this host. Re-author it through the loop to land it.`
- **Why the macOS sandbox was removed.** Four rounds of verifiers escaped it (the Docker socket, the keychain, LaunchServices `open`, TIOCSTI tty injection, reads outside home plus TCP egress), so `sandbox-exec` on a full Go and Bun toolchain is not a boundary we can hold.
- **A trusted PR is trusted fully.** Its preflight runs as the operator and can read any file the operator can; the pod
  token file is the sharp example. Nothing here contains code a trusted account wrote.
- **Residuals the push-access query does not list.** A GitHub App with `contents: write`, a deploy key with write,
  a ruleset or branch-protection bypass actor, an Actions token with `contents: write`, and a pending invitation that
  is later accepted can all put commits on a same-repo branch. Also credentials in `url.<x>.insteadOf` or a git
  credential helper, which the checkout check does not look for. Keep these empty on this repo and on the operator's
  git config.
- **GitHub Apps are not logins.** Any GitHub App installation or deploy key with write access can push to a branch a trusted
  login authored, and the trust gate checks logins, not apps, so `verify-preflight` would run that commit. Mitigation: limit
  write-holding app installations to the repos that need them.
- **A trusted account can carry an outsider's commits.** It can push an outsider's commits onto a same-repo branch, and
  the gate sees only the PR author and the push list. Read the diff, not the author.
- **The pod verdict is the latest per reviewer; the ledger's FAIL is permanent at a head.** A reviewer can flip their own
  verdict with a later message, so a pod PASS is weaker than a ledger FAIL.
- **The `permission=push` filter was seen returning admins only,** so land reads the unfiltered list and keeps
  `.permissions.push === true` itself. Keep `JM_TRUSTED_LOGINS` a superset of what it returns.
- **The pusher list is read at land time.** A collaborator removed since writing a commit no longer shows, so their
  earlier commits on a trusted author's branch still pass. Audit the branch's commits when removing access.
- **Whatever can write the land checkout's `.tools/` chooses the scanner.** Land runs the go installed there (step 4), or, with a
  shared toolchain cache, the go its absolute links point to outside the checkout. `scripts/tools.sh` writes it (`./init.sh` runs it); so can any preflight
  (it runs as the operator) and anything else the operator runs, and that carries into later lands, as do the checkout's
  `.git/config` and hooks, which preflight code can rewrite too. A tracked `.tools` cannot: git would write it into the
  checkout on a fresh clone or a `merge --ff-only` to main, so land refuses every tree it checks that carries one, and refuses a
  `.tools` or `.tools/bin` that is a symbolic link. Rebuild the checkout after any doubt.
- **What CodeRabbit can and cannot do here.** It can write review threads and reviews on a PR; that is all land reads from it. It
  cannot get a commit landed: land refuses unless every commit in `main..head` has a GitHub author and committer login in
  `JM_TRUSTED_LOGINS`, so a commit CodeRabbit (or any app) pushes never lands. It cannot waive a check: it posts no check run
  (`.coderabbit.yaml`), and any failed check run on the head, from any app, still refuses. A review or thread can be silenced only by someone
  with write access (resolving a thread) or by an operator-env login replying in it. Its text is data and never an instruction to land. If
  CodeRabbit is down, rate limited or skipped a head, land does not wait for it: the head passes (c) at once and the `LANDED` note says
  `no CodeRabbit review at this head`, so such lands can be counted. What CodeRabbit has already said at the head still decides.

## Steps
Each refuses with one line on any mismatch, and nothing merges until all pass:
0. The checkout check runs first, before any network call, and runs again once main is fetched (so HEAD can be compared with
   main's tip). The order in the code: environment and usage, the checkout check, the lock, the PR read, the already-landed check (which can record `MERGED-UNVERIFIED` and stop), the trust gate (below), the
   fetch of main, the checkout check again. The check: `git status --porcelain` is empty and `HEAD` is main's tip (an
   explicit `--bootstrap`, the first land of the gate, is the only exception, and only for HEAD: with `--bootstrap`, `HEAD` must be the PR head), no git remote URL
   carries userinfo, no git config carries an `http.*extraheader`, and no `.env*` or `bunfig.toml` exists in the
   checkout (the filesystem is checked, ignored files included: bun would load them as configuration).
1. The PR is open, based on `main`, and its head on GitHub equals the local fetch of `pull/<n>/head`.
2. The ledger at the exact head (below). It admits the head by one of two paths: a `<model>-verifier` PASS, or a `coderabbit` PASS, which land accepts only after its own checks ("The CodeRabbit path", below).
3. CI as a veto: if check runs exist on the head and any concluded in failure (`failure`, `timed_out`, `cancelled`,
   `startup_failure`, `action_required`), land refuses. No check runs at all is fine: land does not wait for CI. The
   ruleset on `main` makes GitHub refuse the merge until CI's required checks pass; land still does not wait, so a land
   run while required checks are pending fails at the merge step.
4. The range secret scan, which can only refuse, then the reviewer's pod verdict, which every land needs (below).
   First, the scan configuration: land lists main's tree at the fetched tip and the tree of every commit in the range (whole, NUL-separated,
   in the scan's env), and refuses (naming the tree and the path) when one carries a path with a component named `.tools`, at any depth,
   compared case-folded (APFS opens `.TOOLS` for `.tools`), always refused by owner ruling: land runs go from the top one, and `.gitignore`
   hides one at any depth, so git, diffing a range path under a nested one, would open an ignored `.gitattributes` there (check-portable
   refuses the same in CI, but only at the PR head, so a range that adds such a path and then deletes it passes there); a path with any
   component named `.gitattributes`, compared case-folded, at any depth, whatever the file holds (always refused, by owner ruling: git
   reads one in the checkout while it diffs, and a line can switch a file out of the diff in more ways than land could parse, among them
   `-diff=<any value>`, a macro, and a pattern after a form feed, a vertical tab or U+00A0, each measured to hide a token); a path with a
   component named `.gitleaks.toml` or
   `.gitleaksignore` at any depth (a directory counts: without a config flag, gitleaks only asks whether `<checkout>/.gitleaks.toml` exists,
   then loads `.gitleaks.json`, `.gitleaks.yaml` or another `.gitleaks.<ext>` beside it); or a top-level entry named `.gitleaks` or
   starting with `.gitleaks.`, unless that file's blob is the one this repo ships. `SHIPPED` in `land-decide.mjs` lists those blobs;
   this repo ships none, so none is allowed. The repository carries no `.gitattributes`, by owner ruling, because land refuses any.
   **Symbolic links** (owner ruling). git follows a link in the checkout while it diffs, so a link on main that a later range replaces with
   a directory makes git open `<link>/.gitattributes` wherever the link points, where the listing below never looks (measured: a link to a
   directory outside the checkout whose attributes file says `* -diff` read a token at `<link>/t.txt` as clean). So the same tree check
   refuses a tracked link, naming the tree, the link and its target, when the target is empty or absolute or, normalised lexically against
   the link's own directory, leaves the checkout or starts with a component that folds to `.git` or `.tools` (`linkFault`). A chain of
   links can pass that and still leave: `q -> .` with `p -> q/q/q/../../..` normalises to the top and resolves three levels above it. So
   land also resolves every link main's tree tracks physically, in the checkout, and refuses one that cannot be resolved or that lands
   outside the checkout's top or under its `.git` or `.tools`, naming it (the rule `check-portable` applies to the PR head in CI). A chain
   a PR adds passes the range check and is refused by the next land. The repository's two links, `CLAUDE.md -> AGENTS.md` and
   `.claude/skills -> ../.agents/skills`, pass both.
   **Land runs gitleaks itself, not make,** with the go installed in the land checkout's `.tools`, checked on the filesystem first (ignored
   files count). `<checkout>/.tools` and `<checkout>/.tools/bin` must be real directories, not symbolic links, and `.tools/bin/go`, followed
   through every link, must be an executable under `<checkout>/.tools` (what `scripts/tools.sh` installs: `.tools/bin/go -> ../go-<version>/bin/go`)
   or outside the checkout (a shared toolchain cache: `.tools/bin/go` is an absolute link into it). A go that resolves anywhere else in the
   checkout is refused, and so is a missing one. `scanRange` (`land-scan.mjs`) runs that resolved path, with `GOROOT` set to the directory
   above its `bin/`: go started through the link looks for its GOROOT two and three levels above the link, and three levels above
   `.tools/bin/go` is the checkout, so a tracked top-level `pkg/tool/` would make the checkout the GOROOT, its `go.env` read and its tools
   run (measured, a case in `check-scan-history.mjs`). It runs through `spawn` with an argv built from constants in the code: `run
   github.com/zricethezav/gitleaks/v8@v8.30.1 git --log-opts=-m <main tip>..<head> --ignore-gitleaks-allow --exit-code 3 --redact --no-banner
   --config <file> --gitleaks-ignore-path <dir> .`, cwd the checkout, env exactly `scanEnv` plus that `GOROOT`. Land writes `<file>` to a
   fresh directory under the operator's `TMPDIR`, outside the checkout, holding `[extend] useDefault = true` and nothing else: that loads
   the same embedded default rules gitleaks uses when it finds no config (v8.30.1 `config/config.go` `extendDefault`; measured: the same
   findings on the same history), and `<dir>` is an empty directory beside it; both are removed after the scan. With `--config` set,
   gitleaks runs no config discovery at all (`cmd/root.go` `initConfig`), so no `.gitleaks.*` in the checkout, tracked or ignored, is
   read, and `GITLEAKS_CONFIG` would lose to it too. gitleaks still reads `<checkout>/.gitleaksignore` whatever the flags say (`cmd/root.go`
   adds it after the ignore path), so land refuses one at the top of the checkout, in any case, on the filesystem (ignored files count). Git opens `<dir>/.gitattributes` in
   the checkout for every directory of a path it diffs, tracked or not, so land also lists the checkout on the filesystem, every entry at
   every depth (ignored ones included, names compared case-folded, symbolic links not followed: main's links are checked as above
   instead), and refuses, naming the path, when it finds a `.gitattributes`. It skips two top-level entries: `.git`, and `.tools`, under
   which no checked tree may carry a path and no checked link may lead, so git diffs nothing there (and the Go `scripts/tools.sh` installs
   ships a `.gitattributes` in its source tree). A `.tools` below the top is listed, because a link main tracks may lead into one. `node_modules` is not skipped, and
   a dependency ships one (`fast-uri`), so a land checkout where `make` installed `node_modules` (`./init.sh` does) is refused until it is
   removed; land needs none there, since its preflight installs its own in a throwaway worktree. The full listing (what a checkout
   holding none costs) took about 50 ms on a checkout with an installed `.tools` and `node_modules` (1,407 directories, 8,508 entries), and
   4 ms on a clone with a cache `.tools`. No shell and no Makefile is read, so a main Makefile cannot change what the
   scan reads, and land does not look `go` up on `PATH` (gitleaks itself runs `git` from the operator's `PATH`). (The earlier design ran `make secrets-range` under a text pin of the recipe; a pin cannot close that class, because a
   built variable name, `$(eval)`, an `include`, a tracked `GNUmakefile`, `SHELL`, a prerequisite that writes `.gitleaks.toml`, or an exported
   `GITLEAKS_CONFIG` or `GIT_CONFIG_*` all change what make runs while the pinned text still matches. Each of those, put on main, now reads as a
   hit in `check-scan-history.mjs`.) `scanEnv` keeps `PATH`, `HOME`, `TMPDIR` and the Go cache variables, pins `GIT_CONFIG_GLOBAL=/dev/null`
   and `GIT_CONFIG_NOSYSTEM=1`, and sets `GOTOOLCHAIN=local` and `GOWORK=off` (a tracked `go.work` or a `go.mod` with a `toolchain` line could
   otherwise make `go run` fetch and run another Go release (measured), and the module is named with an explicit version, so no tracked `go.mod`,
   `vendor/` or `replace` line is consulted). It also sets `GOENV=off`, so the operator's Go env file (`go env -w`) is not read, and pins
   the fetch settings to Go's own defaults: `GOPROXY=https://proxy.golang.org,direct`, `GOSUMDB=sum.golang.org`, and `GOFLAGS`,
   `GONOSUMDB`, `GOPRIVATE` and `GOINSECURE` empty (go reads an empty value as unset, and then only the toolchain's own `go.env`); without
   `GOENV=off` an empty pin would fall through to the operator's file (measured, a case in `check-scan-history.mjs`).
   The Makefile's `secrets-range` stays for people; `check-review-gate.mjs` checks that its recipe is the same command as land's argv. That check is a consistency check, not a security boundary.
   A hit (a nonzero exit plus the line "exit status 3") refuses with "a secret was found in the range; remove it from history and push a new head".
   Clean is positive evidence, read from the scan's exit status and stderr together: exit 0; no "exit status 3" line; no gitleaks
   `ERR` (or `FTL`, `PNC`) line and no "failed to scan"; the "no leaks found" line (gitleaks' "no leaks found in partial scan" warning is not
   it); and one "<n> commits scanned." line, each matched as a whole line, where n equals the number of commits gitleaks reads, which land
   derives itself with git. The counting rule, from gitleaks v8.30.1's source (`sources/git.go` `Fragments`, `detect/detect.go`
   `DetectSource`, and go-gitdiff's parser): gitleaks runs `git log -p -U0 -m <range>`, yields one fragment per hunk of every file that is not
   deleted and not a non-archive binary, and attributes each file to the last "commit" preamble its parser read; it counts a commit when one of
   its fragments arrives, even a hunk with no added line. So a commit counts when its diff has a hunk in a file git does not show as deleted
   **and the parser reached that commit's preamble**. Land runs `git log -m -p -U0 --format=%x01%H <range>` in the scan's own environment (so git reads the same
   config) and counts the commits of `git rev-list <range>` that have such a hunk (`textCommits`). An empty commit, a pure rename, a
   mode-only change, an empty file, a delete-only commit (a revert of an added file too) and a binary-only commit each yield nothing and are
   not counted; a line-removal-only modification, a symlink, a type change and a rename with an edit are. Measured on each of those shapes, on
   a merge whose `-m` diff against one parent is empty and against both, and on an octopus (empty and not), the derived count equals gitleaks'
   printed count (the shape table in `check-scan-history.mjs`), so an innocent history with commits that add no text lands.
   **A count says how many commits were read, never which.** The one shape found where the numbers agree over different commits: gitleaks'
   diff parser ends a quoted file name at the first quote not preceded by a backslash, so a path whose last character is a backslash (git
   quotes it `"b\\"`) fails to parse. The parser drops that file *and the commit preamble before it*, and the files after it are attributed to
   the previous commit's header. With commit X adding `b\` (a token) and `c.txt`, and a rename-only commit W on top, gitleaks prints "1 commits
   scanned", the derived count is also 1 (X), and the one counted is W: equal numbers over disjoint sets, no leak found. So **before the scan
   land lists every path the range touches, in the scan's env** (`git log -m --no-renames --name-status -z`: added, modified and deleted paths,
   so both sides of a rename or copy appear, and every parent of a merge), **and refuses, naming the commit and the path, when any path ends in
   a backslash.** `a.txt` plus `b\` in one commit (the file after the first is never read) and a rename onto or away from such a name are closed
   the same way. Anything else refuses as a scan that failed to run or did not read the whole range: it proves nothing. (A swallowed error, a
   missing object, an attributes path git cannot open (a long directory path, below), and a scan of an empty range in place of the PR's all exit 0 and fail one of these.)
   A clean scan waives nothing. The preflight's `make secrets` (gitleaks over the whole history) is the second scan and refuses by failing the
   preflight. (The code's order is steps 2, 3, 4 as listed: ledger, CI, scan, pod verdict.) The land checkout needs `go` at `.tools/bin/go`
   (and `git`, `node`): run `env PATH=/usr/bin:/bin:/usr/sbin:/sbin scripts/tools.sh` once in the land checkout, which installs the pinned Go and
   Bun into the ignored `.tools/` (or `JM_TOOLCHAIN_CACHE=<dir> scripts/tools.sh`, which links a shared cache into it whatever is on `PATH`).
   The system `PATH` is the point: `scripts/tools.sh` installs nothing into `.tools/` when a go of the pinned version is already on `PATH`,
   so `./init.sh`, which runs it with the operator's `PATH`, can leave the checkout without one (on a system whose `/usr/bin` holds that go,
   use the cache form). Land's refusal names this command. Without it,
   or with a `.tools` that fails the checks above, land refuses before the scan, naming the path. Without a module cache or network for gitleaks, `go run` fails, the scan reads as failed to
   run, and land refuses.
5. A clean-shell preflight of what will land, unless CI already proved the same merge result (next paragraph). The preflight is: a throwaway worktree at main's tip, `git -c core.hooksPath=/dev/null
   merge --no-ff --no-edit <head>` (a conflict refuses; the PR's hooks never run), then `./init.sh` (which runs
   `make preflight`) under `env -i` with HOME and `TMPDIR` inside the throwaway dir and system dirs only on PATH.
   Hygiene, not isolation: stdin is `/dev/null`, the child runs in a new session (no controlling tty), its output is
   captured and printed with control characters stripped and every line prefixed `preflight| `, and its whole
   process group is killed when it returns. Cleanup never turns a pass into a refusal: the tree is made writable
   first, and a failed cleanup only warns naming the directory.
   **Reading CI instead of the preflight.** The preflight is about ten minutes of heavy load per land and CI runs the same gates, so land
   skips it when ALL of these hold, and otherwise runs it exactly as above (the reason is printed to stderr as `no CI proof (...)`):
   - the tree of GitHub's merge ref for the PR (`merge_commit_sha` of `GET pulls/<n>`, its tree from `GET git/commits/<sha>`) equals the
     merge-result tree the decision computed (`git merge-tree --write-tree` of main's tip and the head). Trees are compared, not SHAs. A merge
     ref built on an older main has a different tree, so the skip is refused;
   - every check the branch rules require (`GET rules/branches/main`, then classic protection when the rules name none; never the PR) has a
     run on that ref that is `completed` with conclusion `success` (skipped, neutral, pending, failed and missing all refuse), and every run
     of a required name was made by the GitHub Actions app (slug `github-actions`), so a same-named check from a user or another app refuses;
   - every one of those reads worked: a rule list that cannot be read, or is empty, is no proof. The bootstrap land never skips.
   The LANDED row and the run's output name the path: `CI proved merge tree <8>` or `local preflight`, so the two are countable.
   `--preflight-only` (verifiers) is unchanged and always runs the preflight. The skip changes nothing else: the ledger, the head's CI veto,
   the range scan, the CodeRabbit path and the pod verdict all run first, and the merge is still `--match-head-commit`.
6. The ledger, CI, the range secret scan (run again) and the pod are read again and decided again, and the PR is re-read: the trust gate runs again, its base
   must still be `main` and its head the decided one, and main's tip is fetched once more and must equal the one tested. Then `gh pr merge --merge
   --match-head-commit <head>` (a merge commit; never squash, never `--delete-branch`) and a `LANDED` row. `gh` cannot
   pin the base, so main can still move in the seconds between the check and the merge; that window is accepted. The
   run says which branch is now safe to delete.

Re-running is safe: a merged PR with a `LANDED` row at its head exits 0 saying so; merged without one is recorded
with a warning as `MERGED-UNVERIFIED` (see the ledger format). `--dry-run` runs every check and the preflight, merges nothing and writes no ledger row (git's own
fetch and worktree bookkeeping and the land lock live in the git dir). An unreadable ledger (a row that is not 6 tab columns with a 40-hex head) refuses; no row is skipped silently. A malformed token file is refused without
echoing it, and no output, the quoted preflight output included, ever contains the token. The preflight also gets
`JM_ENTROPY_BASE` (main's tip as fetched) so `make entropy` ratchets against it and fails if it cannot read it. Only the bootstrap land of a main with no `entropy.json`
passes `none` (no base yet); every other unreadable base fails.

The Makefile calls `node`, which an ignored `.tools/bin/node` could shadow under make 3.82 or later. The documented land
command is `node scripts/land.mjs <n>` from the operator's shell.

## Environment (never committed)
| Variable | Meaning |
|---|---|
| `JM_REPO` | the target repo as `owner/name`. Required: a worktree can carry remotes for more than one repo (here `origin` is a different repository), so a bare `gh` call would read or merge in the wrong one. Every `gh` call passes `--repo`, the git remote is the single one whose URL matches, and each PR's URL is checked to be `github.com/<JM_REPO>/pull/<n>` before any action |
| `JM_LEDGER` | path to the ledger TSV |
| `JM_COMMONLY_API` | base URL of the API holding the verdict pod; must be exactly `https://api.commonly.me`, unless `JM_ALLOW_API_ORIGIN` equals it (https only), so the bearer goes only to a host the operator chose |
| `JM_COMMONLY_TOKEN_FILE` | path to a token file; read, never printed |
| `JM_VERDICT_POD` | id of the pod whose messages carry reviewer verdicts |
| `JM_VERDICT_REVIEWERS` | the accepted reviewers, `<id>:<Name>,<id>:<Name>`, by immutable pod user id. Read from the environment only. Every land needs it; unset refuses, and a value with no valid `<id>:<Name>` entry refuses as malformed, before the token is read or the pod is fetched |
| `JM_TRUSTED_LOGINS` | comma-separated GitHub logins that may author code this host runs; read from the environment only. Unset refuses |

A missing variable refuses and names it. The tree holds no pod id, host or path. `--preflight-only` needs only `JM_REPO` and `JM_TRUSTED_LOGINS`.

## Ledger format
Tab-separated with a header row: `pr head verdict by when note`. The operator transcribes each PASS or FAIL row from the verifier's report: the report's `GATE:` line becomes the `verdict` column, the verifier's model becomes `by` (`<model>-verifier`), and the `FINDINGS` line goes in `note`. The coordinator writes no verdict the report does not carry. The tool writes `LANDED`, `MERGED-UNVERIFIED` (a PR merged out of band, recorded with no check; it is not an endorsement, never counts as a PASS, and ends the PR's idempotent re-run) and `BOOTSTRAP`. `by` is `opus-verifier`,
`sonnet-verifier`, `haiku-verifier` or `fable-verifier` (exactly), for verdicts; `land` for the tool's rows. `head`
is the full 40-character SHA. Values are trimmed and compared case-insensitively. At (pr, head):
- an unrecognised verdict, or a `PASS` or `FAIL` by anything but a verifier label, refuses;
- any `FAIL` at the head refuses, whoever wrote it and whatever its note; a later `PASS` does not clear it (no verdict
  shopping). The fix is a new head;
- the latest verdict row by a verifier must be a `PASS` (the verifier path). With no verifier row at the head, a `PASS` by
  `coderabbit` (exactly) names the CodeRabbit path; a `coderabbit` row of any other verdict, or a `PASS` by a lookalike name, refuses.

## The verdict message
In the verdict pod, from the reviewer's account:
```
jellymoon#<pr> <40-char head sha>
...findings...
GATE: PASS
```
The FIRST non-empty line must be exactly `jellymoon#<pr> <full head>` and the last non-empty line exactly
`GATE: PASS` or `GATE: FAIL`. Land matches the author's immutable user id (`userId._id`; a message whose `userId` is not a populated user is ignored)
against the accepted reviewers in operator env `JM_VERDICT_REVIEWERS`, formatted `<id>:<Name>,<id>:<Name>`. A username is a
label any pod member can set on an agent they install, so it is never an identity, and the list is never read from the repo, so a PR
cannot widen it. Unset or malformed, land refuses and names the variable. Ids are deployment config and are never
committed. The text, including any "Written by" line,
is never an identity. A message that merely cites another PR, a short SHA, a quoted or commented verdict, or a
message by anyone else does not count. Each reviewer id's latest message at the head decides (on equal timestamps, the newer one), and any reviewer's
latest that does not pass refuses. Messages are read newest first and deduped by id across pages. The next page
starts 1 ms after the oldest message so messages sharing that instant come back; if a full page adds nothing new,
or the page cap is reached before paging back past the PR's creation, land refuses rather than guess.

Review gate, 2026-10-30 (owner ruling): each head gets one fresh-context review, and its verdict lands the unit. There is
no separate reviewer. The operator account (its id is in `JM_VERDICT_REVIEWERS`) posts the review subagent's report as the
pod verdict: first line `jellymoon#N <head>`, last line `GATE: PASS`. That stays until the owner restores a separate review.

## The CodeRabbit path
The rule: a change needs one review from any one source (the local verifier, another agent, or CodeRabbit) plus green CI. Each PR gets one
local review, at its first head, and its blocking findings get one fix pass. The fix head, and a head that only merges main, is then admitted
by green CI and what CodeRabbit has already said, instead of a second local review. CodeRabbit is asked when there is allowance, and never
awaited. A Major or Critical finding it raises after a merge becomes a fix PR, first in the queue. The operator writes a `PASS` row by `coderabbit` at the
head after reading CodeRabbit's review there, but land never relies on that row: conditions (a) and (c) to (e) are read and checked by land
itself, from git and GitHub, at the moment of the decision (and again right before the merge). Any API error, missing field or unexpected
shape refuses.
- **(a) The one local review happened on this PR's history.** The ledger holds a `PASS` or `FAIL` by a `<model>-verifier` for the PR at a
  different head that is a commit of `git rev-list <main tip>..<head>`. A head outside that range (a commit already on main, or one a
  force-push dropped) is not this PR's history. The latest such row is cited in the output.
- **(b) No `FAIL` at this exact head, by anyone.** The existing rule, unchanged and checked first.
- **(c) Land never waits for CodeRabbit, and never lands over what it already said.** The PR's one review is the local one, (a); CodeRabbit is
  asked when its allowance has room and is not awaited. Land reads `GET /repos/{repo}/pulls/{n}/reviews` (paginated). The GitHub App account
  is REST `user.type` `Bot` with `user.login` exactly `coderabbitai[bot]`; a display name is never read, and a `User` with that login does
  not count. A `CHANGES_REQUESTED` review of CodeRabbit's at the head refuses, whatever else is there; `DISMISSED` and `PENDING` reviews are
  ignored. A `COMMENTED` or `APPROVED` review at the head passes. With none at the head, land reads `GET /repos/{repo}/issues/{n}/comments`
  (paginated; a failed or malformed read refuses) and looks at CodeRabbit's summary comment, whose edit is all a clean incremental review
  leaves. Only comments by that account count; a comment by anyone else is ignored, even if it quotes the marker. The summary is the one whose
  body starts with `<!-- coderabbit-ai-generated:v1 -->` and whose leading HTML-comment lines include
  `<!-- This is an auto-generated comment: summarize by coderabbit.ai -->`.
  - **A gap passes at once, with no ask and no wait:** no CodeRabbit review and no summary comment; a summary whose header says
    `rate limited by coderabbit.ai` or `skip review by coderabbit.ai`; or a well-formed summary whose coverage marker names another commit.
    The output and the `LANDED` note say `no CodeRabbit review at this head`.
  - **A well-formed summary at the head passes, clean or not.** The one review already happened, and what it found is read by (d). Well-formed
    means its status, walkthrough and risk blocks (`recent_review_status`, `walkthrough`, `final_review_risk` start and end delimiters, each
    once, in that order, fenced code ignored) are all there, and the one hidden `final_review_risk_coverage` marker sits inside the risk
    block, parses as JSON with no repeated key, and has `sourceCommitId` and `coveredCommitId` both equal to the head (40 hex, compared
    lower-case) and `kind` exactly `"reviewed"`.
  - **Still refuses, because the shape could be forged or is unreadable:** two or more summary comments; a coverage marker in any other
    CodeRabbit comment (a reply or notice can repeat text other people chose); a missing block, a marker outside the risk block, two
    markers, unparseable JSON, a repeated key, or a marker of another kind; a reviews or comments reply that is not a list.
  The land output says which way (c) was met.
- **(d) No unanswered Major or Critical thread.** Land reads every review thread with the GraphQL `reviewThreads` query (100 per page, up
  to 20 pages, comments first 100 per thread; more than that refuses). A thread counts as CodeRabbit's when its first comment's author is
  `__typename` `Bot` with login `coderabbitai` (GraphQL drops the `[bot]` suffix). It blocks when it is unresolved, not outdated at this
  head, and marked Major or Critical, unless a later comment in the same thread is by a `User` whose login is in `JM_TRUSTED_LOGINS`
  (a reply from CodeRabbit itself, an app or anyone else does not answer it). Severity is read from the first non-empty line of the first
  comment (HTML comments removed), where CodeRabbit writes it, as in `_⚠️ Potential issue_ | _🟠 Major_`. The thread is Minor or lower
  only when that line holds nothing but the known label words (potential, issue, refactor, suggestion, nitpick, trivial, minor, major,
  critical) and every severity word in it is Minor, Trivial or Nitpick. Anything else, an unknown word, a mix with Major, no such line, counts as Major (fail
  closed), and the way out is a trusted reply in the thread (`fixed in <sha>`, `false positive: <why>` or `not fixing: <why>`) or resolving it.
  Limit: (d) reads review threads only. A finding CodeRabbit posts only in its review body (one outside the diff, for example) is not
  read by land, so the operator reads that body at the head before writing the `coderabbit` row.
- **(e) Only trusted people's commits.** For every commit of `main..head` land reads `GET /repos/{repo}/commits/{sha}` and needs both
  `author.login` and `committer.login` in `JM_TRUSTED_LOGINS` (case-insensitive). A null login (an email GitHub cannot match), a missing
  commit, a count above 250, or any app account refuses. Land's own merge commit is made by `gh pr merge` after these checks and is not in the range.

The pod verdict from a listed reviewer at the head (step 4) is still required on this path, as is every other step. The output says
which path admitted the head, `verifier PASS by <model>-verifier` or `coderabbit at <head8>, after the review at <head8>` (with ` (summary comment)` or ` (no CodeRabbit review at this head)` after the first head when (c) was met that way), and the
`LANDED` ledger note records it (`admitted by ...`).

## Every land needs both
Every land needs the ledger PASS at the head (by a `<model>-verifier`, or the CodeRabbit path above) and the pod verdict from a listed reviewer at the head. No
path classification and no scan result waives either: a docs-only PR needs the verdict too, as does a PR with a clean scan. (An earlier
design waived the verdict for "inert" paths and a clean neutral-name re-scan; four scanner blind spots in a row each became a way to
land without the review, and a scanner cannot prove that a secret is absent, so that path was removed.) Before the preflight runs,
the worktree's merge tree must equal the `merge-tree --write-tree` result the decision judged, or land refuses.

The preflight and the merge run code from the PR tree, and the trust gate (below) is what makes that acceptable, not any path rule.
`check-portable` resolves each tracked symlink's full target physically and refuses one that leaves the repo, and `make gen` will
not write through a symlink.

## Verifiers use the same trust gate
`make verify-preflight PR=<n>` (`node scripts/land.mjs <n> --preflight-only`) runs the trust gate, the head check and
the merge-result preflight: no ledger, no pod, no merge. Run it from a clean checkout of main, never from the PR's tree,
with the same checkout rule as land (`--bootstrap` is the one exception, for the bootstrap PR only, and then `HEAD` must be the PR head). It needs `JM_REPO` and `JM_TRUSTED_LOGINS`. For an untrusted
PR it refuses before fetching anything, and the verifier reads the code (`git show`, `git diff`, GitHub) and runs nothing.

## Known limits
- **A grandchild that calls `setsid` again escapes the group kill.** The preflight child is killed as a process group; code that
  starts its own session survives it. That is trusted code (see the threat model).
- **A PR can switch off its own scans on the bootstrap land and in the preflight.** On the bootstrap land the checkout is the PR head. Its range's trees are checked as on any land and land passes its own config, so a PR's `.gitleaks*`, `.gitleaksignore`, `.gitattributes` and tracked `.tools` are refused, not read; but the `.tools/bin/go` it runs is the PR checkout's own install. On every land, the preflight's `make secrets` runs the PR's Makefile, `.gitleaks.toml`, `.gitleaksignore` and `gitleaks:allow` markers. On a normal land the range scan uses main's config, never its Makefile, and ignores markers. These are defence in depth only; the verdict is always required.
- **The throwaway's own `make secrets` reads a PR-added `.gitleaks.toml`.** The preflight runs `make secrets` in the merged tree, so a PR-added `.gitleaks.toml`, `.gitleaksignore` or `.gitattributes` configures that scan (a `paths = [".*"]` allowlist makes it clean). Land's range scan is not configured by one: it checks every range commit's tree (step 4), where each of them is refused, and passes its own config. The scans only refuse, so a quiet scan is never evidence; the review is.
- **What the scans do not read, so the review must cover it.** Both scans are defence in depth that can refuse and never waive. Read by neither:
  - **Fragment cuts.** `gitleaks dir` and git mode read a file in 100,000-byte chunks (extended at most 25,000 bytes to a blank line), and no rule matches across chunks, so a token straddling a cut is not matched.
  - **File-type sniffing.** `gitleaks dir` types a file by its leading bytes (offsets up to 32,773) and skips what it calls `application/*`.
  - **Binaries and images.** Git mode skips binary files and images.
  - **Allowlisted paths.** The pinned gitleaks v8.30.1 ships a built-in allowlist (its default config, `config/gitleaks.toml`, under `[allowlist]`: path patterns for dependency directories, lockfiles and manifests such as `go.mod`, minified and vendored JS, fonts, PDFs and more). Cited by reference, not listed here, because a partial list understates it. A secret added then deleted in such a path sits in main's history unscanned. Added-then-deleted text elsewhere IS read.
  - **Commit headers.** Extra commit headers and the author and committer fields.
  - **File names.**
  - **A commit message after a NUL.** `git log %B` stops at it, and git mode never reads commit messages at all.
- **The expected commit count follows one gitleaks version.** `textCommits` encodes how v8.30.1 counts (a hunk in a file git does not show as
  deleted). It is measured against land's own scan on every shape listed in step 4. A gitleaks bump that counts differently makes land
  refuse every PR with the "did not read the whole range" line until `textCommits` and its shape table follow (fail closed). A count is a
  count and not a set: the trailing-backslash refusal closes the one shape found where the count and the commits read disagree, and no other
  is known, but this check cannot prove there is none; the review is the other check.
- **A path ending in a backslash anywhere in the range is refused,** also when it only arrives through a merge of main into the PR (the
  `-m` diff against the PR's parent lists it), and also when it is harmless. The cost is an innocent PR refused (fail closed); the remedy is
  a new head without that name (rename the file on main first, or rebase the history).
- **A directory path of 1,009 bytes or more in the range is refused.** For each directory a diff touches, git opens that directory's
  `.gitattributes` in the checkout, by its path relative to the top. From 1,009 bytes `<dir>/.gitattributes` reaches the macOS `PATH_MAX`
  of 1,024, so git prints "warning: unable to access …: File name too long", gitleaks logs it as an `ERR` line, and land reads the scan as a
  failure ("the range secret scan failed to run, or did not read the whole range"). It fails closed: an innocent PR with such a path is
  refused too (measured: a directory path of 1,008 bytes reads clean and 1,009 fails; the absolute path, 1,180 bytes at 1,008, does not count).
  Shorten the path.
- **A commit message with a line over 64 KB is refused.** gitleaks' log reader cannot take it, so it counts none where land derives one
  (measured: rev-list 1, gitleaks 0, derived 1); land fails closed with the "did not read the whole range" line on an innocent PR.
- **What `go run` still takes from the operator, and from whatever writes the operator's files.** The installed toolchain (`.tools/`, or
  the cache its links point into: see the threat model), the module and build caches (`GOMODCACHE`, `GOPATH`, `GOCACHE`) and the
  network (what is not in the module cache comes through the public proxy and is checked against the checksum database, both pinned in
  `scanEnv`; what is in it is used as it stands). The operator's Go env file is not read (`GOENV=off`). From the repo, a tracked `.tools` is refused in every tree land checks, and
  a tracked `go.work`, `go.mod` (`toolchain`, `replace`), `vendor/` or top-level `pkg/tool/` has no effect (see step 4).
- **The scan configuration is checked in the committed trees, and on the filesystem only for `.tools`, a top-level `.gitleaksignore`,
  a `.gitattributes` outside the top-level `.git` and `.tools`, and where the links main tracks resolve.** The scan also reads attributes and config that are in no tree, and land
  checks none of them: `.git/info/attributes`; the land checkout's own `.git/config` and `.git/config.worktree` (a `diff` driver,
  `core.attributesFile`, `attr.tree`); the operator's global attributes file, `.config/git/attributes` in their home directory (git's default `core.attributesFile`, read although
  `scanEnv` points `GIT_CONFIG_GLOBAL` at `/dev/null`); the system attributes file (`git var GIT_ATTR_SYSTEM` names `/etc/gitattributes` for
  the macOS git; by git's documentation `GIT_CONFIG_NOSYSTEM` does not turn it off, not measured here); a `.gitattributes` under an ignored symbolic link to a directory (git follows the link, the
  listing does not, and the on-disk link check resolves only the links main's tree tracks, so it does not close this); and an ignored file at a path gitleaks or go looks for. The checkout check refuses untracked files and a diff from HEAD,
  but an ignored file passes it, so keep the land checkout a clean clone and rebuild it after any doubt.
- **Both secret scans pass `-m`** (`make secrets`, and land's range scan): gitleaks runs `git log -p` without it, so a secret that
  exists only in a merge commit's own diff (a conflict resolution) would never be scanned.
- **Every fetch of main names `refs/heads/main`**, never bare `main`: a tag named `main` wins git's DWIM.
- **The merge commit's tree is compared with the tested one.** After `gh pr merge`, land fetches the merge commit and checks its tree
  against the preflight's merge result. A mismatch (the base or main moved in the last window) is written on the LANDED row as
  `TREE-MISMATCH` and warned loudly. It is detected, not undone.
- **A hand `make preflight` with `JM_ENTROPY_BASE=none` switches the entropy ratchet off.** Land sets that only for the bootstrap
  land of a main with no `entropy.json`; nothing stops a person setting it by hand.
- **A signal during `gh pr merge` can leave a merged PR with no LANDED row.** Recovery is record-only: the next run sees the PR
  merged and records `MERGED-UNVERIFIED`; it checks nothing.
- **Node 20 or newer.** Land refuses to start on an older Node (`process.versions.node`).
- **A `gitleaks:allow` marker anywhere in the PR's history is ignored by the range scan** (`--ignore-gitleaks-allow`), which refuses on what the marker hid. The preflight's `make secrets` honours markers, so the review reads for them.
- **The `.env*` and `bunfig.toml` check reads the repo root only**, ignored files included. A nested `sub/.env` is not
  checked, because bun runs from the root.
- **The remote check covers http(s) userinfo and `http.*.extraheader`**, including a URL rewritten by `insteadOf` (it reads
  `git remote -v`, which shows the rewrite). It does not look at `credential.helper`, and it lets a non-http URL carry a
  plain user (`ssh://git@host`); only a user:password pair is refused there.
- **The base can be retargeted between the last re-read and the merge.** `--match-head-commit` pins the head only.
- **The CI veto reads completed check runs only.** In-progress runs and legacy commit statuses are ignored, so land never
  waits for CI; a failed completed run vetoes.
- **The checkout check** also refuses `assume-unchanged` or `skip-worktree` files, a diff from HEAD, and untracked files
  (all shown).
- **A message's author, or the pod creator, can change its `created_at`** (Commonly's `PATCH /api/messages/:id/created-at`). A reviewer could reorder their own verdicts, and the creator anyone's; a backdated FAIL could also slip past the paging bound. Same trust class as the deletion below.
- **A pod creator can delete a reviewer's message.** Land reads only what exists at land time, so deleting a
  reviewer's latest FAIL brings back their earlier PASS at the same head. The pod's creator is trusted.
- **The deny check's names class does not run on the merge path.** Land builds the preflight environment without
  `JM_PRIVATE_NAMES`, so `make private` skips that class during a land. It runs in CI only once a
  `JM_PRIVATE_NAMES` repository secret is set, which it is not yet, and locally when the operator exports
  `JM_PRIVATE_NAMES`. Passing it through land's environment is a follow-up.
- **A trusted author is trusted fully.** Nothing here contains code a trusted account wrote; see the threat model.

## Bootstrap: the first land of the gate
While main has no `scripts/land.mjs`, land and `verify-preflight` refuse unless run with `--bootstrap` (and with
`--bootstrap` they refuse once main has the file). With `--bootstrap` the checkout must be at the PR head itself, not main's tip. It appends a
`BOOTSTRAP` row. The first land of the gate is judged by people and recorded as such.
