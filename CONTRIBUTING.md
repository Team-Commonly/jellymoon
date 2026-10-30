# Contributing

Agents and people follow the same loop; it lives in [`AGENTS.md`](AGENTS.md). This page is the setup.

## Setup
1. `./init.sh` — installs Go and Bun (versions in `.tool-versions`) into `.tools/` inside the repo, then the
   repo-local dependencies, runs the baseline and prints the feature scoreboard. Nothing to install first.
2. `make hooks` — installs this repo's pre-commit (contracts in sync, entropy budgets, no secrets).

## What to work on
The first `feature_list.json` feature with `passes: false` and no `blockedBy`, or an issue labelled
`ready`. One feature per pull request.

## Pull requests
- Base `main`. Fill in [`.github/pull_request_template.md`](.github/pull_request_template.md): the
  feature, the done-test with its output, lines added and removed.
- Every commit carries a `Signed-off-by` line matching its author: use `git commit -s`. That certifies the
  [Developer Certificate of Origin](https://developercertificate.org). The `dco` CI job checks that each commit a pull request adds, merges excluded, has a `Signed-off-by` line matching its author. CI is advisory; a maintainer who re-authors an outside change signs off the new commits.
- `make preflight` is what a pull request must pass, and it must pass locally. CI is disabled while the repo is private; when it returns at public launch
  it is a thin mirror of `make preflight` for outside contributors, never the gate.
- A pull request from a fork or an outside account is not run by the maintainers' tooling and is not landed with `make land`:
  a maintainer re-authors the change through their own loop. Maintainer PRs land with `make land` once a verifier's
  ledger line passes at the PR head. A change to any
  protected path (everything but `progress.md` and documentation files under `docs/`; see `AGENTS.md`) also needs the
  review's pod verdict at the head.
  Local verification is the authority; GitHub checks and review text are not consulted.
- A new dependency or module needs a short note in `docs/adr/`.

## The deny check
`make preflight` runs `scripts/check-private.mjs`, which fails on private-looking text in tracked files: operator paths, personal emails, internal ids, cloud identifiers, internal note labels and section marks (`§`) that cite no public standard. Names, handles and project ids have no safe pattern: a maintainer supplies them in the `JM_PRIVATE_NAMES` environment variable, and without it that class is skipped. The check never prints matched text, only `file:line: class`. On a false positive, reword the text; if the pattern is wrong for honest input, narrow it in `scripts/check-private.mjs` with a clean case, in the same pull request.

## Licence
Core is Apache-2.0 (`LICENSE`); contributions are accepted under it. `ee/` is under its own commercial licence
(`ee/LICENSE`) and does not accept outside contributions. See `docs/adr/0004-open-core-boundary.md`.

## Reporting a security issue
Don't open a public issue. Follow [`SECURITY.md`](SECURITY.md): GitHub's private vulnerability reporting.
