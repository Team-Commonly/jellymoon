# Contributing

Agents and people follow the same loop; it lives in [`AGENTS.md`](AGENTS.md). This page is the setup.

## Setup
1. Install Go (version in `go.mod`) and Bun 1.4.4.
2. `./init.sh` — installs repo-local dependencies, runs the baseline and prints the feature scoreboard.
3. `make hooks` — installs this repo's pre-commit (contracts in sync, entropy budgets, no secrets).

## What to work on
The first `feature_list.json` feature with `passes: false` and no `blockedBy`, or an issue labelled
`ready`. One feature per pull request.

## Pull requests
- Base `main`. Fill in [`.github/pull_request_template.md`](.github/pull_request_template.md): the
  feature, the done-test with its output, lines added and removed.
- `make preflight` must pass locally; CI runs the same targets.
- A maintainer merges once CI is green and the PR has a review verdict at its head. Changes to protocol
  or security paths (listed in `AGENTS.md`) also need a maintainer's gate review at the PR head.
- A new dependency or module needs a short note in `docs/adr/`.

## Reporting a security issue
Don't open a public issue. Email the maintainers listed in `.github/CODEOWNERS` with the details.
