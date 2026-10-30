# Landing a PR

`make land PR=<n>` is the only merge path. Local verification is the authority; GitHub check runs are consulted only
as a veto (step 4). It runs `scripts/land.mjs`; the decisions are pure functions in `scripts/land-decide.mjs` and the
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
- **GitHub Apps are not logins.** Eight org GitHub App installations hold `contents:write`. Four are installed on all repos
  (claude, cursor, chatgpt-codex-connector, sentry). Four are installed on selected repos (coderabbitai, commonly-tools,
  latchkey-app, mintlify), and which repos is unknown, so assume this one. (A ninth, claude-design-import, is read-only.) An app can push to a trusted author's same-repo branch, and the trust gate checks
  logins, not apps, so `verify-preflight` would run that commit. This stays a known hole until an org owner restricts
  those installations.
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
   main's tip). The order in the code: environment and usage, the checkout check, the lock, the PR read, the trust gate (below), the
   fetch of main, the checkout check again. The check: `git status --porcelain` is empty and `HEAD` is main's tip (an
   explicit `--bootstrap`, the first land of the gate, is the only exception, and only for HEAD), no git remote URL
   carries userinfo, no git config carries an `http.*extraheader`, and no `.env*` or `bunfig.toml` exists in the
   checkout (the filesystem is checked, ignored files included: bun would load them as configuration).
1. The PR is open, based on `main`, and its head on GitHub equals the local fetch of `pull/<n>/head`.
2. The ledger at the exact head (below).
3. For a protected path (below), or any added line containing `gitleaks:allow`, the reviewer's pod verdict. A
   protected path always needs it. The changed files and added lines come from the real merge result:
   `git merge-tree --write-tree <main tip> <head>` diffed against main's tip (external diff and textconv off), so renames
   count on both sides, no file cap applies and a criss-cross history cannot hide a path. Hunk headers are skipped, so an
   added line starting `++` still counts. Git's own binary test applies to that diff (`--numstat` reports `-\t-`): a changed
   file git treats as binary (a NUL in the first 8000 bytes, or a `binary` or `-diff` attribute) makes the PR protected, and the
   content of every inert file at the merge result is scanned once more under neutral names (below).
4. CI as a veto: if check runs exist on the head and any concluded in failure (`failure`, `timed_out`, `cancelled`,
   `startup_failure`, `action_required`), land refuses. No check runs at all is fine: CI is off by decision.
5. A clean-shell preflight of what will land: a throwaway worktree at main's tip, `git -c core.hooksPath=/dev/null
   merge --no-ff --no-edit <head>` (a conflict refuses; the PR's hooks never run), then `./init.sh` (which runs
   `make preflight`) under `env -i` with HOME and `TMPDIR` inside the throwaway dir and system dirs only on PATH.
   Hygiene, not isolation: stdin is `/dev/null`, the child runs in a new session (no controlling tty), its output is
   captured and printed with control characters stripped and every line prefixed `preflight| `, and its whole
   process group is killed when it returns. Cleanup never turns a pass into a refusal: the tree is made writable
   first, and a failed cleanup only warns naming the directory.
6. The ledger, CI and the pod are read again and decided again, and the PR is re-read: the trust gate runs again, its base
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
| `JM_VERDICT_REVIEWERS` | the accepted reviewers, `<id>:<Name>,<id>:<Name>`, by immutable pod user id. Read from the environment only. Needed when a verdict is needed; unset or empty refuses |
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
cannot widen it. Unset or empty, a protected land refuses and names the variable. Ids are deployment config and are never
committed. The text, including any "Written by" line,
is never an identity. A message that merely cites another PR, a short SHA, a quoted or commented verdict, or a
message by anyone else does not count. Each reviewer id's latest message at the head decides (on equal timestamps, the newer one), and any reviewer's
latest that does not pass refuses. Messages are read newest first and deduped by id across pages. The next page
starts 1 ms after the oldest message so messages sharing that instant come back; if a full page adds nothing new,
or the page cap is reached before paging back past the PR's creation, land refuses rather than guess.

Review gate, 2026-10-30 (owner ruling): each head gets one fresh-context review, and its verdict lands the unit. There is
no separate reviewer. The operator account (its id is in `JM_VERDICT_REVIEWERS`) posts the review subagent's report as the
pod verdict: first line `jellymoon#N <head>`, last line `GATE: PASS`. That stays until the owner restores a separate review.

## Protected paths
A path is unprotected only if it is inert: the file `progress.md`, or a file under `docs/` whose extension is `.md` or `.html`
(never `docs/development/`). Images (`.svg .png .jpg .gif .webp`) are protected: gitleaks does not scan them in git mode, so a
token in one would otherwise land on the ledger PASS alone, and neither the history scan nor `make secrets-range` can see it.
Everything else is protected, `docs/**/*.go` included, because Go compiles
and runs it (`go vet ./...`, `go test ./...`). Anything protected (`GNUmakefile`, `package.json`, `scripts/`,
`.githooks/`, `go.mod`, `entropy.json`, `docs/x/x_test.go`, `docs/a.js`, ...) runs from the PR tree during the
preflight, so it needs the reviewer's verdict, as does any PR that adds a `gitleaks:allow` line (it can hide a secret). Matching is on the lowercased path (macOS is case-insensitive: `docs/Development/x.md` would overwrite
`docs/development/x.md`), and a path whose lowercase equals another path in main's tree or in the same PR, or a directory spelled
two ways, is protected. APFS also folds Unicode (U+017F to `s`, U+212A to `k`) and ignores normalization, so rather than chase each fold a path with
a non-ASCII byte, an empty, `.` or `..` segment, or a segment ending in a dot or space is protected too. A rename counts both sides: `docs/a.md` to `docs/a.go` is
protected, and so are `docs/x` to `scripts/x` and the reverse. So is any entry whose old or new mode is not a regular
file (`100644`, `100755`): a symlink (`120000`) or gitlink (`160000`) is never inert, wherever it sits. The modes come from
`git diff --raw` on the merge result. `check-portable` resolves each tracked symlink's full target physically and refuses one
that leaves the repo, and `make gen` will not write through a symlink. One function, `touchesProtected` in
`scripts/land-decide.mjs`, owns the rule; land reads the changed files through `filesFromRaw` there too.

The scanner has blind spots: gitleaks does not read binaries, and its built-in allowlist skips any path with a `node_modules` or
`bower_components` segment and any path containing `gitleaks.toml`. So inertness does not depend on a list of those: land writes each
inert file's blob at the merge result to `<throwaway>/inert/<n>.txt` and runs `gitleaks dir --ignore-gitleaks-allow` over that
directory (`make secrets-dir`), with no config from the PR. A hit makes the PR protected. Before the preflight runs, the worktree's
merge tree must equal the `merge-tree --write-tree` result the decision judged, or land refuses.

## Verifiers use the same trust gate
`make verify-preflight PR=<n>` (`node scripts/land.mjs <n> --preflight-only`) runs the trust gate, the head check and
the merge-result preflight: no ledger, no pod, no merge. Run it from a clean checkout of main, never from the PR's tree,
with the same checkout rule as land (`--bootstrap` is the one exception, for the bootstrap PR only). It needs `JM_REPO` and `JM_TRUSTED_LOGINS`. For an untrusted
PR it refuses before fetching anything, and the verifier reads the code (`git show`, `git diff`, GitHub) and runs nothing.

## Known limits
- **A grandchild that calls `setsid` again escapes the group kill.** The preflight child is killed as a process group; code that
  starts its own session survives it. That is trusted code (see the threat model).
- **Windows 8.3 short names and backslash aliases are not folded** by the protected-path rule. The host is macOS.
- **The throwaway's own `make secrets` reads a PR-added `.gitleaks.toml`.** The decision is unaffected: that file is not inert, so
  the PR is protected. Land's range scan runs in the operator's checkout, which is main with main's config, except on the bootstrap
  land: there the checkout is the PR head, so a PR-added `.gitleaks.toml` or `.gitattributes` configures the scan (a `paths = [".*"]` allowlist makes it clean). Both files are protected paths, so that PR needs the verdict, and the neutral-name scan below reads no config from the PR.
- **Every blob the range adds is scanned, not only the merge result.** `gh pr merge --merge` keeps each commit, so a secret added in one commit and deleted in the next sits in main's history while the merge result is clean, and gitleaks git mode skips binaries, images, `node_modules`, lockfiles and `go.sum`. Land lists the blobs with `git rev-list --objects <main tip>..<head>`. **Land decides what is text, never the scanner.** `gitleaks dir` types each file by its leading bytes (the `filetype` matchers, offsets up to 32,773) and silently skips what it calls `application/*`, whatever the file is named: measured on the pinned gitleaks, a token after `%PDF-`, `\x7fELF`, `PK\x03\x04`, `%!PS-Adobe` or `{\rtf1` was "no leaks found", and so was one in a text file with `ustar` at offset 257 (tar), `DICM` at 128 (DICOM) or `CD001` at 32769 (ISO). So:
  - A range blob or commit message is text only if it is valid UTF-8 with no NUL, no C0 control other than tab, LF, FF and CR, and no DEL. Anything else (ELF, zip, gzip, PNG, Mach-O, wasm, SQLite, a PDF with binary streams) makes the PR protected, so it needs the verdict.
  - Every text is written to its neutral file behind 33,600 bytes of fixed plain lines (`jellymoon-range-scan`), so the bytes the scanner sniffs are always that padding and no magic at any offset can match. Re-measured with the padding: `%PDF-`, `%!PS-Adobe`, `{\rtf1`, `GIF89a`, `ustar` at 257, `DICM` at 128 and `CD001` at 32769 are all found. A hit needs the verdict.
  - Tests run the real pinned gitleaks over one case per magic, for a deleted range blob and for a commit message, plus an end-to-end case through `main()`.
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
- **A `gitleaks:allow` marker anywhere in the PR's history protects it.** Land scans the added lines of every commit in
  `<main tip>..<head>` (the merge keeps every commit, and gitleaks honours the marker per commit), and `make
  secrets-range` runs gitleaks over that range with `--ignore-gitleaks-allow`. A hit needs the pod verdict.
- **`docs/development/` is protected**, because it holds these rules and the verifier protocol.
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
- **A trusted author is trusted fully.** Nothing here contains code a trusted account wrote; see the threat model.

## Bootstrap: the first land of the gate
While main has no `scripts/land.mjs`, land and `verify-preflight` refuse unless run with `--bootstrap` (and with
`--bootstrap` they refuse once main has the file). It appends a
`BOOTSTRAP` row. The first land of the gate is judged by people and recorded as such.
