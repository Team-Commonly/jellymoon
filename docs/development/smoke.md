# The smoke chain: `cargo xtask smoke`

Each feature is proved by its own test at one binary. The smoke chain is the one place the built programs run
together: it builds `jellymoon`, `jellymoon-app` and the made-up harness `fake-claude` once, then runs them in order
in a fresh temporary home under the target folder, which it removes at the end. Every step is a plain program run
whose exit code and JSON output are checked. The first step that fails ends the run with `smoke: FAILED at <step>:`
and what the program printed. After the build it takes about two seconds.

```sh
make smoke                    # the features the Makefile picks for this machine
cargo xtask smoke [--window] [--real [--skip <names>]] [-- <cargo build arguments>]
```

The chain runs the programs of the debug build in the target folder. A build argument with which it would run
what an earlier build left there is refused with one line, before anything is built:

- one that makes cargo put the programs somewhere else, or build another workspace's: `--release`, `-r` alone or
  among other short flags (`-vr`), `--profile`, `--target`, `--target-dir`, `--config`, `--manifest-path`;
- one that builds only some targets, and so could leave `jellymoon`, `jellymoon-app` or `fake-claude` as it was:
  `--bin`, `--bins`, `--lib`, `--example`, `--examples`, `--test`, `--tests`, `--bench`, `--benches`, `--exclude`.

What only adds to the build is taken: `-p`, `--package`, `--workspace`, `--features`, `-F`. The build is started
without `CARGO_BUILD_TARGET` and `CARGO_BUILD_TARGET_DIR`, which move it as the flags do. Not held: a
`build.target` or `build.target-dir` in a `.cargo/config.toml` above the repo also moves the build, and the chain
then runs what `target/debug` holds. The repo has no such file.

Each run works in `target/smoke-home-<pid>-<nanoseconds>`, named for its process and for when it started, and
removes it at its end. A run that was stopped leaves its home, and with `--real` that home holds the person's
page. So a run first removes the homes earlier runs left, before it builds: a build that fails has still removed
them. Each run holds the system's lock on the `.lock` file beside its home for as long as it lives, on every
system, and takes it before it makes the home. A home whose lock can be taken is one no living run has, and since
no later run is ever given that name again, removing the home and then its lock file takes nothing from a run
that lives. (Named for the process alone, a home could be removed under a new run that the system gave an old
run's id, by a third run that had opened the old lock file before it was unlinked.)

| Step | What it proves |
|---|---|
| `logs` | writes a made-up day: two projects, a wish and the change of mind that clashes with it, a wish that finished |
| `day` | `jellymoon day` reads them: 3 turns in 2 projects |
| `page` | `jellymoon page`, through `fake-claude`, prints page data with the 3 wishes, one of them done, and the clash |
| `found-off-path` | as in an app opened from the Finder: with an empty `PATH` and no harness named, `jellymoon page` finds `fake-claude` in the temporary home's `.local/bin` and runs it |
| `layout-sticker`, `layout-plain` | `jellymoon-app layout` on that page, in each built-in theme: every wish is a character and the clash is a choice note |
| `today` | `jellymoon-app layout` with no `--page` reads the made-up day itself, through `fake-claude`: every wish is drawn, and the page is kept in the temporary home |
| `today-again` | the same run again prints the same pieces from the kept page, and `fake-claude` is not called |
| `broken-page` | a file that is not page data is refused in one line, exit 1, nothing on stdout |
| `empty-day` | a day nobody typed in is an empty page, made with no reading call, and the app draws it |
| `no-logs` | `jellymoon day` over a logs folder that is not there says in one line that no session logs were found, and exits 1 |
| `hook-install` | `jellymoon hook install` puts its entry into made-up harness settings in the temporary home |
| `rule` | `jellymoon rule` keeps a ruling, and one waits in the store |
| `hook-told`, `hook-again` | `jellymoon hook`, given a made-up step, hands the session the ruling; its next step is told nothing |
| `tap` | `jellymoon settle`, the tap, on the second statement of the kept page's clash makes one ruling, which waits, and the page keeps it |
| `tap-told` | the session's next step is told that ruling alone, once |
| `today-settled` | `jellymoon-app layout` draws the clash settled, the replaced statement struck, and no delivery note |
| `wish` | `jellymoon wish` keeps a made wish as `m1`, working, with no run, and `jellymoon-app layout` draws it on the kept page with no reading call |
| `copy` | with the real `git` on the caller's `PATH` and no git config of this machine: a repo the chain makes, `jellymoon copy make` on it, an edit in the copy, and `jellymoon copy clean` leaving one commit on the run's branch, the folder gone and the checkout as it was |
| `run` | a repo the chain makes and a wish made for it: `jellymoon run`, through `fake-agent`, ends as completed with one commit on the run's branch, which holds `HELLO.txt` with the one word `hello` (trimmed, in any case), the checkout has no changed file, and `jellymoon page` shows the wish done with one star. With `--real` the harness is the person's own Claude Code, on one harmless fixed wish |
| `stop` | a second wish in that repo and a run whose `fake-agent` does its work and then does not end: `jellymoon run stop` ends it, both commands print `cancelled`, the page shows the wish needing the person with no star, the copy is kept with nothing committed on the run's branch, and the checkout has no changed file |
| `wish-run` | a repo the chain makes in the system's temporary folder, and a made-up log of something typed in it: `jellymoon wish --run` finds the repo by its name, starts the run through `fake-agent` and exits; `jellymoon page` then shows the wish done with one star, the run's command ends by itself with one commit on the run's branch, and the checkout has no changed file. The step removes the repo |
| `uninstall` | `jellymoon uninstall` removes a stand-in app and command and leaves their folders empty; the hook is out and the settings are as they were |
| `window` (with `--window`) | `jellymoon-app --page` opens a window and, with `JELLYMOON_QUIT_AFTER=3`, exits 0 within 30 seconds with no panic |

A made-up run starts each program with no variable of the caller's: a scratch `HOME`, `CLAUDE_CONFIG_DIR`,
`JELLYMOON_HOME`, `JELLYMOON_APP_DIR` and `JELLYMOON_BIN_DIR` in the temporary home, and `JELLYMOON_CLAUDE` naming the copy of
`fake-claude` there. It never runs Claude Code and never reads a real log. The `copy`, `run`, `stop` and `wish-run` steps
alone are given the caller's `PATH`, to find git, with an empty global git config and the system's switched off.
The `run`, `stop` and `wish-run` steps name their harness, a copy of `fake-agent`, with `JELLYMOON_CLAUDE`, so no harness on
that `PATH` is looked for.

## The three modes

- **Default.** Made-up logs and `fake-claude`. CI runs it on macOS, Linux and Windows after the tests.
- **`--window`** adds the window step. It needs a display, so that step alone sees the caller's variables. CI runs
  it on macOS, where a failure fails the job: the hosted runner opens the GPUI window and closes it.
- **`--real`**, for a maintainer on their own machine, never in CI or by an agent. The same steps over today's real
  logs and the real Claude Code on `PATH`: one real reading call. `logs`, `found-off-path`, `today`, `today-again`, `empty-day`,
  `no-logs`, the delivery steps, `wish`, `copy`, `stop`, `wish-run` and `uninstall` are left out;
  `--skip` is the reader's `--skip`. What Jellymoon writes still goes to the temporary home. It prints counts, states
  and exit codes only: a failed run says how many bytes a program printed, never what. Its sixth step is `run`
  (F39): one real run of the person's Claude Code, in a repo the chain makes in the temporary home, on the fixed
  wish "add a file named HELLO.txt that holds the one word hello". The run is stopped after four minutes and the
  step after five, and it prints the outcome, how many stars the page shows, how many things were denied, and yes
  or no for the file on the branch and for the checkout being unchanged: never what a file holds.

A maintainer ran `--real` on the commit the tag `v0.1.0-rc1` points at, and its five steps passed. The `run` step
was added since, and nobody has run it with `--real` yet: no agent may.

## What it does not prove

That a page looks right, that a real model's reading passes the checks (only `--real` tries), that `install.sh`
installs (F33's test), or anything a person must see: Gatekeeper, a double click. It is not a feature's test: a
feature still passes on its own `test` command.

## Adding a step

A feature that adds a command adds its step in `crates/xtask/src/smoke/steps.rs`: one function
`fn(&World) -> Result<String, String>`, and one line in `STEPS` where it belongs in the order. Run the program with
`World::json`, `World::lines` or `World::run`. Return counts and states, never page text. A check that knows what the
made-up day holds goes through `World::made_up`, so `--real` skips it. Mark the step `Only::Synthetic` if it needs the
made-up logs or would change something of the person's.

## Before a release, by a person

1. `make smoke`, then `make smoke args=--real` on a day with sessions, then `make smoke args=--window`.
2. Run the release workflow by hand (`install.md`, "Cutting a release") and install from that rehearsal build into a
   scratch home: `JELLYMOON_RELEASE_URL=file://<folder>`, with `HOME`, `JELLYMOON_APP_DIR` and `JELLYMOON_BIN_DIR`
   set to scratch folders.
3. Double-click `Jellymoon.app` there and look at the page.
4. `jellymoon uninstall` with the same variables, and check both folders are empty.
