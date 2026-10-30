# Landing a PR

`make land PR=<n>` is the only merge path. Local verification is the authority; GitHub check runs are consulted only
as a veto (step 4). It runs `scripts/land.mjs`; the decisions are pure functions in `scripts/land-decide.mjs` and the
cases are in `scripts/check-land.mjs`. Run it from a checkout of main with a clean tree: it reads the reviewer list
from main's tip, never from the working tree. One land runs at a time **per clone**: an mkdir lock in that clone's git
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
  (`GET /repos/{repo}/collaborators?affiliation=all&permission=push`, paginated) is in that list too. The list is an
  operator env var, comma-separated, never read from the repo, so a PR cannot widen it; unset, land refuses. Trust is
  a GitHub identity, never a commit author string, which anyone can set.
- **An outside PR is never executed here.** Land and `make verify-preflight` refuse it before fetching or running any
  of its code: `untrusted PR: not executed on this host; CI runs it. Re-author it through the loop to land it.`
- **Why the macOS sandbox was removed.** Four rounds of verifiers escaped it (the Docker socket, the keychain, LaunchServices `open`, TIOCSTI tty injection, reads outside home plus TCP egress), so `sandbox-exec` on a full Go and Bun toolchain is not a boundary we can hold.
- **A trusted PR is trusted fully.** Its preflight runs as the operator and can read any file the operator can; the pod
  token file is the sharp example. Nothing here contains code a trusted account wrote.
- **Residuals the push-access query does not list.** A GitHub App with `contents: write`, a deploy key with write,
  a ruleset or branch-protection bypass actor, an Actions token with `contents: write`, and a pending invitation that
  is later accepted can all put commits on a same-repo branch. Also credentials in `url.<x>.insteadOf` or a git
  credential helper, which the checkout check does not look for. Keep these empty on this repo and on the operator's
  git config.

## Steps
Each refuses with one line on any mismatch, and nothing merges until all pass:
0. The trust gate (above), then the checkout: `git status --porcelain` is empty and `HEAD` is main's tip (an explicit
   `--bootstrap`, the first land of the gate, is the only exception, and only for HEAD), no git remote URL carries userinfo, and no git config carries an
   `http.*extraheader`.
1. The PR is open, based on `main`, and its head on GitHub equals the local fetch of `pull/<n>/head`.
2. The ledger at the exact head (below).
3. For a protected path (below), or any added line containing `gitleaks:allow`, the reviewer's pod verdict. A
   protected path always needs it. The changed files come from the local `git diff --name-status -M <merge-base>
   <head>`, so renames count on both sides and no file cap applies.
4. CI as a veto: if check runs exist on the head and any concluded in failure (`failure`, `timed_out`, `cancelled`,
   `startup_failure`, `action_required`), land refuses. No check runs at all is fine: CI is off by decision.
5. A clean-shell preflight of what will land: a throwaway worktree at main's tip, `git -c core.hooksPath=/dev/null
   merge --no-ff --no-edit <head>` (a conflict refuses; the PR's hooks never run), then `./init.sh` (which runs
   `make preflight`) under `env -i` with HOME and `TMPDIR` inside the throwaway dir and system dirs only on PATH.
   Hygiene, not isolation: stdin is `/dev/null`, the child runs in a new session (no controlling tty), its output is
   captured and printed with control characters stripped and every line prefixed `preflight| `, and its whole
   process group is killed when it returns. Cleanup never turns a pass into a refusal: the tree is made writable
   first, and a failed cleanup only warns naming the directory.
6. Main's tip is fetched again and must equal the one tested; the ledger, CI and the pod are read again and decided
   again, and the PR is re-read: its base must still be `main` and its head the decided one. Then `gh pr merge --merge
   --match-head-commit <head>` (a merge commit; never squash, never `--delete-branch`) and a `LANDED` row. `gh` cannot
   pin the base, so main can still move in the seconds between the check and the merge; that window is accepted. The
   run says which branch is now safe to delete.

Re-running is safe: a merged PR with a `LANDED` row at its head exits 0 saying so; merged without one is recorded
with a warning as `MERGED-UNVERIFIED` (see the ledger format). `--dry-run` runs every check and the preflight, merges nothing and writes no ledger row (git's own
fetch and worktree bookkeeping and the land lock live in the git dir). An unreadable ledger (a row that is not 6 tab columns with a 40-hex head) refuses; no row is skipped silently. A malformed token file is refused without
echoing it, and no output ever contains the token.

## Environment (never committed)
| Variable | Meaning |
|---|---|
| `JM_REPO` | the target repo as `owner/name`. Required: a worktree can carry remotes for more than one repo (here `origin` is a different repository), so a bare `gh` call would read or merge in the wrong one. Every `gh` call passes `--repo`, the git remote is the single one whose URL matches, and each PR's URL is checked to be `github.com/<JM_REPO>/pull/<n>` before any action |
| `JM_LEDGER` | path to the ledger TSV |
| `JM_COMMONLY_API` | base URL of the API holding the verdict pod; must be https |
| `JM_COMMONLY_TOKEN_FILE` | path to a token file; read, never printed |
| `JM_VERDICT_POD` | id of the pod whose messages carry reviewer verdicts |
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

## The verdict message a reviewer posts
In the verdict pod, from the reviewer's own account:
```
jellymoon#<pr> <40-char head sha>
...findings...
GATE: PASS
```
The FIRST non-empty line must be exactly `jellymoon#<pr> <full head>` and the last non-empty line exactly
`GATE: PASS` or `GATE: FAIL`. Land matches the message's server-stamped `username` against
`.github/opus-reviewers.txt` (`Name username`) as it stands on main; the text, including any "Written by" line,
is never an identity. A message that merely cites another PR, a short SHA, a quoted or commented verdict, or a
message by anyone else does not count. Each reviewer's latest message at the head decides (on equal timestamps, the newer one), and any reviewer's
latest that does not pass refuses. Messages are read newest first and deduped by id across pages. The next page
starts 1 ms after the oldest message so messages sharing that instant come back; if a full page adds nothing new,
or the page cap is reached before paging back past the PR's creation, land refuses rather than guess.

## Protected paths
A path is unprotected only if it is inert: the file `progress.md`, or a file under `docs/` whose extension is one of
`.md .html .svg .png .jpg .gif .webp`. Everything else is protected, `docs/**/*.go` included, because Go compiles
and runs it (`go vet ./...`, `go test ./...`). Anything protected (`GNUmakefile`, `package.json`, `scripts/`,
`.githooks/`, `go.mod`, `entropy.json`, `docs/x/x_test.go`, `docs/a.js`, ...) runs from the PR tree during the
preflight, so it needs the reviewer's verdict, as does any PR that adds a `gitleaks:allow` line (it can hide a secret). A rename counts both sides: `docs/a.md` to `docs/a.go` is
protected, and so are `docs/x` to `scripts/x` and the reverse. One function, `touchesProtected` in
`scripts/land-decide.mjs`, owns the rule; land reads the changed files through `filesFromNameStatus` there too.

## Verifiers use the same trust gate
`make verify-preflight PR=<n>` (`node scripts/land.mjs <n> --preflight-only`) runs the trust gate, the head check and
the merge-result preflight: no ledger, no pod, no merge. Run it from a clean checkout of main, never from the PR's tree,
with the same checkout rule as land (`--bootstrap` is the one exception, for the bootstrap PR only). It needs `JM_REPO` and `JM_TRUSTED_LOGINS`. For an untrusted
PR it refuses before fetching anything, and the verifier reads the CI logs and the code and runs nothing.

## Known limits
- **A pod creator can delete a reviewer's message.** Land reads only what exists at land time, so deleting a
  reviewer's latest FAIL brings back their earlier PASS at the same head. The pod's creator is trusted.
- **A trusted author is trusted fully.** Nothing here contains code a trusted account wrote; see the threat model.

## Bootstrap: the first land of the gate
While main has no `scripts/land.mjs`, land refuses unless run with `--bootstrap` (and with `--bootstrap` it
refuses once main has the file). It then reads the reviewer list from the PR head and appends a
`BOOTSTRAP` row. The first land of the gate is judged by people and recorded as such.
