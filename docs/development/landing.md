# Landing a PR

`make land PR=<n>` is the only merge path. Local verification is the authority; GitHub checks and GitHub review text
are not consulted. It runs `scripts/land.mjs` (decisions in `scripts/land-decide.mjs`, cases in
`scripts/check-land.mjs`). GitHub's own review text is not read.

## Steps
For each PR, each step refusing with one line on any mismatch:
1. The PR is open, and its head SHA on GitHub equals the local fetch of `pull/<n>/head`.
2. A clean-shell preflight at that head: a throwaway worktree, `./init.sh` and `make preflight` under `env -i`
   with an empty HOME and system dirs only on PATH. The worktree is removed afterwards.
3. The ledger: the last verdict row for this PR and head is `PASS`, by a `<model>-verifier`. A later `FAIL`, a
   row at another head, or `coordinator` as author refuses.
4. For a protected path (list in `AGENTS.md`), the reviewer's pod verdict (below).
5. `gh pr merge --merge --match-head-commit <head>` (a merge commit; never squash, never `--delete-branch`),
   then a `LANDED` ledger row.

Re-running is safe: a merged PR with a `LANDED` row exits 0 saying so; merged without one is recorded with a warning.

`--stack` walks the base chain down to `main` and lands bottom-up, retargeting each base to `main` once the one
below is merged. Every check runs for every PR before the first merge. A PR whose ledger says `FIXED-FWD` with
note `#<m>` passes step 3 only if `#<m>` is above it in the same chain and has a verifier `PASS` at its head.
Branches are never deleted; the run prints which are now safe to delete. `--dry-run` runs every check, prints the
plan and merges nothing.

## Environment (never committed)
| Variable | Meaning |
|---|---|
| `JM_REPO` | the target repo as `owner/name`. Required: a worktree can carry remotes for more than one repo (here `origin` is a different repository), so a bare `gh` call would read or merge in the wrong one. Every `gh` call passes `--repo`, the git remote is the single one whose URL matches (https or ssh, with or without `.git`), and each PR's URL is checked to be `github.com/<JM_REPO>/pull/<n>` before any action |
| `JM_LEDGER` | path to the ledger TSV |
| `JM_COMMONLY_API` | base URL of the API holding the verdict pod |
| `JM_COMMONLY_TOKEN_FILE` | path to a token file; read, never printed |
| `JM_VERDICT_POD` | id of the pod whose messages carry reviewer verdicts |
| `JM_TOOLCHAIN_CACHE` | optional; passed into the clean shell |

A missing variable refuses and names it. The tree holds no pod id, host or path.

## Ledger format
Tab-separated with a header row: `pr head verdict by when note`. `verdict` is `PASS`, `FAIL` or `FIXED-FWD` (note
`#<pr>`), plus the tool's own `LANDED` (note: merge sha) and `REGATE-OWED`. `by` is `<model>-verifier` (for
example `opus-verifier`), `coordinator` or `land`. `head` is the full 40-character SHA.

## The verdict message a reviewer posts
In the verdict pod, from the reviewer's own account:
```
jellymoon#<pr> <40-char head sha>
...findings...
GATE: PASS
```
The last non-empty line must be exactly `GATE: PASS` or `GATE: FAIL`. Land matches the message's server-stamped
`username` against `.github/opus-reviewers.txt` (`Name username`); the text, including any "Written by" line, is
never an identity. A short SHA, another PR number, a quoted or commented verdict, or a message by anyone else
does not count. Each reviewer's latest message at the head decides, and any reviewer's latest that does not pass
refuses. Messages are read newest first, paged back past the PR's creation.

## Temporary: remove when the reviewers return
`make land PR=<n> ARGS=--reviewer-gate-unavailable` satisfies step 4 without a pod verdict. It needs a second ledger
row at the same PR and head: `PASS`, by a verifier on a different model from the first, note starting
`reviewer-gate-unavailable`; the latest such row decides. The second verifier is fresh-context: it has never seen the
builder's or the first verifier's reasoning. Each land that used the flag appends `REGATE-OWED`;
`make land ARGS=--owed` lists PRs with that row and no later pod PASS from a listed reviewer at that head. It
blocks nothing. Whatever a later reviewer re-gate fails is fixed forward.
