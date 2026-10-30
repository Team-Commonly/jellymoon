# Landing a PR

`make land PR=<n>` is the only merge path. Local verification is the authority; GitHub check runs are consulted only
as a veto (step 3). It runs `scripts/land.mjs`; the decisions are pure functions in `scripts/land-decide.mjs` and the
cases are in `scripts/check-land.mjs`. Run it from a checkout of main with a clean tree: the accepted reviewers come from
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
- **Trusted preflight code can persist.** It runs as the operator and can rewrite the land checkout's `.git/config`, its
  hooks or the ignored `.tools/`, and that carries into later lands. Rebuild the checkout after any doubt.

## Steps
Each refuses with one line on any mismatch, and nothing merges until all pass:
0. The checkout check runs first, before any network call, and runs again once main is fetched (so HEAD can be compared with
   main's tip). The order in the code: environment and usage, the checkout check, the lock, the PR read, the already-landed check (which can record `MERGED-UNVERIFIED` and stop), the trust gate (below), the
   fetch of main, the checkout check again. The check: `git status --porcelain` is empty and `HEAD` is main's tip (an
   explicit `--bootstrap`, the first land of the gate, is the only exception, and only for HEAD: with `--bootstrap`, `HEAD` must be the PR head), no git remote URL
   carries userinfo, no git config carries an `http.*extraheader`, and no `.env*` or `bunfig.toml` exists in the
   checkout (the filesystem is checked, ignored files included: bun would load them as configuration).
1. The PR is open, based on `main`, and its head on GitHub equals the local fetch of `pull/<n>/head`.
2. The ledger at the exact head (below).
3. CI as a veto: if check runs exist on the head and any concluded in failure (`failure`, `timed_out`, `cancelled`,
   `startup_failure`, `action_required`), land refuses. No check runs at all is fine: CI is off by decision.
4. The range secret scan, which can only refuse, then the reviewer's pod verdict, which every land needs (below).
   First, main's own scan configuration: land lists main's tree at the fetched tip, and refuses (naming the file) when it carries a
   `.gitleaks.toml` or `.gitleaksignore` at any path, or a `.gitattributes` line that switches files out of the diff (`-diff`, `!diff`,
   `binary`, or a `!` pattern), unless that file's blob is the one this repo ships. `SHIPPED` in `land-decide.mjs` lists those blobs; this repo
   ships none, so none is allowed.
   `make secrets-range` runs gitleaks over `<main tip>..<head>` with `-m` and `--ignore-gitleaks-allow`: a hit (a nonzero exit
   plus the line "exit status 3") refuses with "a secret was found in the range; remove it from history and push a new head".
   Clean is positive evidence, read from the scan's exit status and stderr together: exit 0; no "exit status 3" line; no gitleaks
   `ERR` (or `FTL`, `PNC`) line and no "failed to scan"; the "no leaks found" line (gitleaks' "no leaks found in partial scan" warning is not
   it); and one "<n> commits scanned." line, each matched as a whole line, where n equals the number of commits gitleaks reads, which land
   derives itself with git, outside make. The rule, from gitleaks v8.30.1's source (`sources/git.go` `Fragments`, `detect/detect.go`
   `DetectSource`): gitleaks runs `git log -p -U0 -m <range>`, yields one fragment per hunk of every file that is not deleted and not a
   non-archive binary, and counts a commit when one of its fragments arrives, even a hunk with no added line. So a commit counts exactly when
   its diff has a hunk in a file git does not show as deleted. Land runs `git log -m -p -U0 --format=%x01%H <range>` in the scan's own
   environment (the same allowlist env as make, so git reads the same config) and counts the commits of `git rev-list <range>` that have such a
   hunk (`textCommits`). An empty commit, a pure rename, a mode-only change, an empty file, a delete-only commit (a revert of an added file
   too) and a binary-only commit each yield nothing and are not counted; a line-removal-only modification, a symlink, a type change and a
   rename with an edit are. Measured on each of those shapes, on a merge whose `-m` diff against one parent is empty and against both, and on an
   octopus (empty and not), the derived count equals gitleaks' printed count (the shape table in `check-review-gate.mjs`), so an innocent
   history with commits that add no text lands. Limit: the rule follows one gitleaks version; a bump of the pin must re-run the shape table,
   and a change in what gitleaks counts makes land refuse (fail closed) until `textCommits` follows. No shape is known where the two differ;
   archives named like archives were measured too and gitleaks reads none in git mode, so they count as no text. Anything else refuses as a scan
   that failed to run or did not read the whole range: it proves nothing. (A recipe that swallowed an error, a missing object, a
   `.gitattributes` the scan stumbles on, and a scan of an empty range in place of the PR's all exit 0 and fail one of these.) A clean scan waives nothing. The preflight's
   `make secrets` (gitleaks over the whole history) is the second scan and refuses by failing the preflight. (The code's order is
   steps 2, 3, 4 as listed: ledger, CI, scan, pod verdict.) The scan runs the make recipe, which runs `go run`: the land checkout needs
   `go` (and `make`, `git`, `node`) reachable. Run `./init.sh` once in the land checkout, which installs the pinned Go and Bun into the
   ignored `.tools/` (the Makefile finds `.tools/bin/go` by path), and put `.tools/bin` on PATH. Without `go`, or without a module cache or network for
   gitleaks, the recipe fails, the scan reads as failed to run, and land refuses.
5. A clean-shell preflight of what will land: a throwaway worktree at main's tip, `git -c core.hooksPath=/dev/null
   merge --no-ff --no-edit <head>` (a conflict refuses; the PR's hooks never run), then `./init.sh` (which runs
   `make preflight`) under `env -i` with HOME and `TMPDIR` inside the throwaway dir and system dirs only on PATH.
   Hygiene, not isolation: stdin is `/dev/null`, the child runs in a new session (no controlling tty), its output is
   captured and printed with control characters stripped and every line prefixed `preflight| `, and its whole
   process group is killed when it returns. Cleanup never turns a pass into a refusal: the tree is made writable
   first, and a failed cleanup only warns naming the directory.
6. The ledger, CI, the range secret scan (`make secrets-range` runs again) and the pod are read again and decided again, and the PR is re-read: the trust gate runs again, its base
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
Tab-separated with a header row: `pr head verdict by when note`. The coordinator transcribes each PASS or FAIL row from the verifier's report: the report's `GATE:` line becomes the `verdict` column, the verifier's model becomes `by` (`<model>-verifier`), and the `FINDINGS` line goes in `note`. The coordinator writes no verdict the report does not carry. The tool writes `LANDED`, `MERGED-UNVERIFIED` (a PR merged out of band, recorded with no check; it is not an endorsement, never counts as a PASS, and ends the PR's idempotent re-run) and `BOOTSTRAP`. `by` is `opus-verifier`,
`sonnet-verifier`, `haiku-verifier` or `fable-verifier` (exactly), for verdicts; `land` for the tool's rows. `head`
is the full 40-character SHA. Values are trimmed and compared case-insensitively. At (pr, head):
- an unrecognised verdict, or a `PASS` or `FAIL` by anything but a verifier label, refuses;
- any `FAIL` at the head refuses, whoever wrote it and whatever its note; a later `PASS` does not clear it (no verdict
  shopping). The fix is a new head;
- the latest verdict row must be a `PASS`.

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

## Every land needs both
Every land needs the ledger PASS by a `<model>-verifier` at the head and the pod verdict from a listed reviewer at the head. No
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
- **A PR can switch off its own scans on the bootstrap land and in the preflight.** On the bootstrap land the checkout is the PR head, so the range scan runs the PR's own Makefile `secrets-range` recipe (a recipe of `true` reads as clean) and its `.gitleaksignore`. On every land, the preflight's `make secrets` runs the PR's Makefile, `.gitleaks.toml`, `.gitleaksignore` and `gitleaks:allow` markers. On a normal land the range scan uses main's Makefile and config and ignores markers. These are defence in depth only; the verdict is always required.
- **The throwaway's own `make secrets` reads a PR-added `.gitleaks.toml`.** Land's range scan runs in the operator's checkout, which is main with main's config, except on the bootstrap land: there the checkout is the PR head, so a PR-added `.gitleaks.toml` or `.gitattributes` configures the scan (a `paths = [".*"]` allowlist makes it clean). The scans only refuse, so a quiet scan is never evidence; the review is.
- **What the scans do not read, so the review must cover it.** Both scans are defence in depth that can refuse and never waive. Read by neither:
  - **Fragment cuts.** `gitleaks dir` and git mode read a file in 100,000-byte chunks (extended at most 25,000 bytes to a blank line), and no rule matches across chunks, so a token straddling a cut is not matched.
  - **File-type sniffing.** `gitleaks dir` types a file by its leading bytes (offsets up to 32,773) and skips what it calls `application/*`.
  - **Binaries and images.** Git mode skips binary files and images.
  - **Allowlisted paths.** The pinned gitleaks v8.30.1 ships a built-in allowlist (its default config, `config/gitleaks.toml`, under `[allowlist]`: path patterns for dependency directories, lockfiles and manifests such as `go.mod`, minified and vendored JS, fonts, PDFs and more). Cited by reference, not listed here, because a partial list understates it. A secret added then deleted in such a path sits in main's history unscanned. Added-then-deleted text elsewhere IS read.
  - **Commit headers.** Extra commit headers and the author and committer fields.
  - **File names.**
  - **A commit message after a NUL.** `git log %B` stops at it, and git mode never reads commit messages at all.
- **The expected commit count follows one gitleaks version.** `textCommits` encodes how v8.30.1 counts (a hunk in a file git does not show as
  deleted). It is measured against the real recipe on every shape listed in step 4. A gitleaks bump that counts differently makes land
  refuse every PR with the "did not read the whole range" line until `textCommits` and its shape table follow; it never lands one wrongly.
- **Main's scan configuration is checked in the committed tree only.** `.git/info/attributes`, the land checkout's own `.git/config`
  (a `diff` driver, `core.attributesFile`) and ignored files are not read; the checkout check and the operator's clean clone cover them. A
  `.gitattributes` that hides files by a means other than `-diff`, `!diff`, `binary` or a `!` pattern is not caught either.
- **Both secret scans pass `-m`** (`make secrets`, `make secrets-range`): gitleaks runs `git log -p` without it, so a secret that
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
- **The CI veto reads completed check runs only.** In-progress runs and legacy commit statuses are ignored, which matches
  "CI is off; a failing run vetoes".
- **The checkout check** also refuses `assume-unchanged` or `skip-worktree` files, a diff from HEAD, and untracked files
  (all shown).
- **A message's author, or the pod creator, can change its `created_at`** (Commonly's `PATCH /api/messages/:id/created-at`). A reviewer could reorder their own verdicts, and the creator anyone's; a backdated FAIL could also slip past the paging bound. Same trust class as the deletion below.
- **A pod creator can delete a reviewer's message.** Land reads only what exists at land time, so deleting a
  reviewer's latest FAIL brings back their earlier PASS at the same head. The pod's creator is trusted.
- **The deny check's names class does not run on the merge path.** Land builds the preflight environment without
  `JM_PRIVATE_NAMES`, so `make private` skips that class during a land. It runs in CI once CI is enabled, and locally when the
  operator exports `JM_PRIVATE_NAMES`. Passing it through land's environment is a follow-up.
- **A trusted author is trusted fully.** Nothing here contains code a trusted account wrote; see the threat model.

## Bootstrap: the first land of the gate
While main has no `scripts/land.mjs`, land and `verify-preflight` refuse unless run with `--bootstrap` (and with
`--bootstrap` they refuse once main has the file). With `--bootstrap` the checkout must be at the PR head itself, not main's tip. It appends a
`BOOTSTRAP` row. The first land of the gate is judged by people and recorded as such.
