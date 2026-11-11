# Jellymoon

*Jellymoon, by [Commonly](https://commonly.me).* A journal of what you asked your coding agents for.

Jellymoon is a local desktop app that reads the Claude Code and Codex sessions already on your computer. It shows one
**page** per day: each thing you wished for, a star for every job an agent finished on it, and a note wherever two
things you said cannot both stand. Tapping that note settles it, and the answer reaches the agents that are running.

It needs no account and no server. Your session logs stay on your computer; the one model call that reads a day goes
through the harness you already use, under your own sign-in, with every tool turned off.

## Install (Mac)

```sh
curl -fsSL https://raw.githubusercontent.com/Team-Commonly/jellymoon/main/install.sh | sh
```

That puts `Jellymoon.app` in `~/Applications` and the `jellymoon` command in `~/.local/bin`. It asks for no password,
checks the download against its checksum, and running it again updates. `jellymoon uninstall` removes it. To read the
script before running it, open [`install.sh`](install.sh), or download it first:
`curl -fsSL -o install.sh https://raw.githubusercontent.com/Team-Commonly/jellymoon/main/install.sh`, read it, then
`sh install.sh`. [`docs/development/install.md`](docs/development/install.md) says what it does and what it trusts.

## Status: early

Open the app and it shows today's page: it reads the day with one call through your own Claude Code, keeps the page
in `~/.jellymoon`, and reads again only when you have typed more. An app opened from the Finder may not find Claude
Code yet (it does not see your shell's `PATH`); until that is fixed, start it from a terminal:

```sh
~/Applications/Jellymoon.app/Contents/MacOS/jellymoon-app
```

The `jellymoon` command-line binary holds the logic the app draws: `day` collects what you typed, `page` turns a day
into page data. Each step of the plan is a test in [`feature_list.json`](feature_list.json), and `./init.sh` prints
which ones pass. The design is [ADR 0011](docs/adr/0011-the-journal-app.md). The Mac comes first and Windows after.

## Build and run from source

```sh
./init.sh        # installs the pinned Rust and gitleaks into .tools/, runs the checks, prints the scoreboard
```

`init.sh` installs nothing outside the repo. To use that toolchain in your shell:

```sh
export RUSTUP_HOME="$PWD/.tools/rustup" CARGO_HOME="$PWD/.tools/cargo" PATH="$PWD/.tools/cargo/bin:$PATH"
```

If you already have rustup, skip both: `cargo` reads `rust-toolchain.toml` and fetches the pinned toolchain itself.

```sh
cargo run -p jellymoon -- day                      # today's typed turns, as JSON
cargo run -p jellymoon -- day --date 2026-11-09 --skip private-notes
cargo run -p jellymoon -- page                     # today's page data; makes one call to `claude` on your PATH
```

[`docs/development/reader.md`](docs/development/reader.md) and [`page.md`](docs/development/page.md) say exactly what
each command reads, keeps and checks. `page` spends your own Claude plan: one call per run.

```sh
make hooks       # pre-commit: format, the repo's own rules, a secret scan of what is staged
make preflight   # what a pull request has to pass
```

## The old harness

Before November 2026 this repo held a different product, a shared space for people and their agents with a server and
a daemon. That code is not in the tree. It stays reachable at the tag
[`harness-final`](https://github.com/Team-Commonly/jellymoon/tree/harness-final);
[ADR 0012](docs/adr/0012-restart-as-the-journal-app-in-rust.md) says what was removed and why.

## Licence

Apache-2.0 ([`LICENSE`](LICENSE)), except third-party files vendored into the repo, which keep their own licences. The
vendored agent skills are listed in [`docs/skills/SOURCES.md`](docs/skills/SOURCES.md).

## Contributing

People and agents follow the same loop, written in [`AGENTS.md`](AGENTS.md). Setup and the pull request steps are in
[`CONTRIBUTING.md`](CONTRIBUTING.md). Sign off every commit with `git commit -s`; the sign-off certifies the
[Developer Certificate of Origin](https://developercertificate.org).

## Security

Please don't open a public issue for a security problem. Report it privately; [`SECURITY.md`](SECURITY.md) says how.
