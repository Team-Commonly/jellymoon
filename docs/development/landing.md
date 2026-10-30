# Landing a PR

`make land PR=<n>` is the only merge path. Local verification is the authority: GitHub checks and GitHub review
text are never consulted. It runs `scripts/land.mjs`; the decisions are pure functions in `scripts/land-decide.mjs`
and the cases are in `scripts/check-land.mjs`. Run it from main's checkout: it reads the reviewer list from
main's tip, never from the working tree. One land runs at a time (an mkdir lock in the git common dir holds the
PID; a dead PID is taken over).

The stack lands as one PR retargeted to `main`. Land handles exactly one PR whose base is `main`; any other base
is refused.

## Steps
Each refuses with one line on any mismatch, and nothing merges until all pass:
1. The PR is open, based on `main`, and its head on GitHub equals the local fetch of `pull/<n>/head`.
2. The ledger at the exact head (below).
3. For a protected path (below), the reviewer's pod verdict. The changed files come from the local
   `git diff --name-status -M <merge-base> <head>`, so renames count on both sides and no file cap applies.
4. A clean-shell preflight of what will land: a throwaway worktree at main's tip, `git merge --no-ff --no-edit
   <head>` (a conflict refuses), then `./init.sh` (which runs `make preflight`) under `env -i` with an empty HOME and
   system dirs only on PATH, inside `sandbox-exec`. The sandbox cannot read the operator's home-directory
   `.commonly`, `.claude`, `.codex`, `.ssh`, `.aws` or `.config` dirs or the directory holding `JM_LEDGER`, and
   cannot write outside the throwaway dir, `JM_TOOLCHAIN_CACHE`, `/dev` and the system temp dirs. The network
   stays open (`tools.sh` may download a toolchain). On any platform but macOS land refuses: `preflight sandbox
   is macOS-only; add one for <platform> before landing here`. `scripts/check-land.mjs` proves the profile with a
   forbidden read and write.
   Cleanup never turns a pass into a refusal: the tree is made writable first, and a
   failed cleanup only warns naming the directory.
5. Main's tip is fetched again and must equal the one tested; the ledger and the pod are read again and decided
   again. Then `gh pr merge --merge --match-head-commit <head>` (a merge commit; never squash, never
   `--delete-branch`) and a `LANDED` row. `gh` cannot pin the base, so main can still move in the seconds
   between the check and the merge; that window is accepted. The run says which branch is now safe to delete.

Re-running is safe: a merged PR with a `LANDED` row at its head exits 0 saying so; merged without one is recorded
with a warning. `--dry-run` runs every check and the preflight, merges nothing and writes no ledger row (git's own
fetch and worktree bookkeeping and the land lock live in the git dir). A malformed token file is refused without
echoing it, and no output ever contains the token.

## Environment (never committed)
| Variable | Meaning |
|---|---|
| `JM_REPO` | the target repo as `owner/name`. Required: a worktree can carry remotes for more than one repo (here `origin` is a different repository), so a bare `gh` call would read or merge in the wrong one. Every `gh` call passes `--repo`, the git remote is the single one whose URL matches, and each PR's URL is checked to be `github.com/<JM_REPO>/pull/<n>` before any action |
| `JM_LEDGER` | path to the ledger TSV |
| `JM_COMMONLY_API` | base URL of the API holding the verdict pod; must be https |
| `JM_COMMONLY_TOKEN_FILE` | path to a token file; read, never printed |
| `JM_VERDICT_POD` | id of the pod whose messages carry reviewer verdicts |
| `JM_TOOLCHAIN_CACHE` | optional; passed into the clean shell, and takes effect only once `scripts/tools.sh` supports it (a separate PR) |

A missing variable refuses and names it. The tree holds no pod id, host or path.

## Ledger format
Tab-separated with a header row: `pr head verdict by when note`. Each PASS or FAIL row is the verifier's own verdict line, copied verbatim into the ledger by the coordinator. The coordinator never writes a verdict of its own. The tool writes `LANDED`, `REGATE-OWED` and `BOOTSTRAP`. `by` is `opus-verifier`,
`sonnet-verifier`, `haiku-verifier` or `fable-verifier` (exactly), for verdicts; `land` for the tool's rows. `head`
is the full 40-character SHA. Values are trimmed and compared case-insensitively. At (pr, head):
- an unrecognised verdict, or a `PASS` or `FAIL` by anything but a verifier label, refuses;
- any `FAIL` refuses, whatever its note, unless a LATER `PASS` from the same label supersedes it;
- the latest primary row must be a `PASS`.

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
message by anyone else does not count. Each reviewer's latest message at the head decides, and any reviewer's
latest that does not pass refuses. Messages are read newest first and deduped by id across pages. The next page
starts 1 ms after the oldest message so messages sharing that instant come back; if a full page adds nothing new,
or the page cap is reached before paging back past the PR's creation, land refuses rather than guess.

## Protected paths
Every changed path is protected except paths under `docs/` and the file `progress.md`. Anything else
(`GNUmakefile`, `package.json`, `scripts/`, `.githooks/`, `go.mod`, `entropy.json`, ...) runs from the PR tree
during the preflight, so it needs the reviewer's verdict. A rename counts both sides: `docs/x` to
`scripts/x` is protected, and so is the reverse. One function, `touchesProtected` in `scripts/review-gate.mjs`,
owns the rule; land reads the changed files through `filesFromNameStatus` in `scripts/land-decide.mjs`.

## Bootstrap: the first land of the gate
While main has no `scripts/land.mjs`, land refuses unless run with `--bootstrap` (and with `--bootstrap` it
refuses once main has the file). It then reads the reviewer list from the PR head and appends a
`BOOTSTRAP` row. The first land of the gate is judged by people and recorded as such.

## Temporary: remove when the reviewers return
`make land PR=<n> ARGS=--reviewer-gate-unavailable` satisfies step 3 without a pod verdict. It needs a second ledger
row at the same PR and head: `PASS`, by a verifier of a different model family from the first (a different
family word), note starting `reviewer-gate-unavailable`; the latest such row decides. The second verifier is
fresh-context: it has never seen the builder's or the first verifier's reasoning. Each land that used the flag
appends `REGATE-OWED`; `make land ARGS=--owed` (no PR needed) lists PRs with that row and no later pod PASS from a
listed reviewer at that head. It blocks nothing. Whatever a later reviewer re-gate fails is fixed forward.
