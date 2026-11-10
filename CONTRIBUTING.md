# Contributing

Agents and people follow the same loop; it lives in [`AGENTS.md`](AGENTS.md). This page is the setup.

## Setup
1. `./init.sh` installs the Rust named in `rust-toolchain.toml` and a pinned `gitleaks` into `.tools/` inside the repo,
   runs the baseline and prints the feature scoreboard. Nothing to install first on macOS or Linux.
2. `make hooks` installs this repo's pre-commit (format, the repo's own rules, a secret scan of what is staged).

On Windows, install rustup yourself and run the four commands `make preflight` runs: `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo xtask check` and `cargo test --workspace`.

## What to work on
The first `feature_list.json` feature with `passes: false` and no `blockedBy`, or an issue labelled `ready`. One
feature per pull request.

## Pull requests
- Base `main`. Fill in [`.github/pull_request_template.md`](.github/pull_request_template.md): the feature, the
  done-test with its output, lines added and removed.
- Every commit carries a `Signed-off-by` line matching its author: use `git commit -s`. That certifies the
  [Developer Certificate of Origin](https://developercertificate.org). CI's `dco` job checks each commit a pull request adds.
- `make preflight` must pass locally. CI runs the same checks on macOS, Linux and Windows.
- A pull request merges when CI is green on its head and a review of that head ends in `GATE: PASS`
  ([`docs/development/workflow.md`](docs/development/workflow.md)). A maintainer merges it.
- Code from a pull request by someone outside the maintainers is read, never run on a maintainer's machine, until a
  maintainer has reviewed it. Expect the first reply to come from reading your diff.
- A new dependency or crate needs a short note in `docs/adr/`.

## Session logs are private
Jellymoon reads people's agent session logs. Never put text from a real log in a test, a fixture, an issue or a pull
request. Type fixtures by hand, with made-up projects, words and credential shapes.

## Licence
Apache-2.0 (`LICENSE`); contributions are accepted under it.

## Reporting a security issue
Don't open a public issue. Follow [`SECURITY.md`](SECURITY.md): GitHub's private vulnerability reporting.
