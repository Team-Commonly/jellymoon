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
| `run` | a repo the chain makes and a wish made for it: `jellymoon run`, through `fake-agent`, ends as completed with one commit on the run's branch, and `jellymoon page` shows the wish done with one star. With `--real` the harness is the person's own Claude Code, on one harmless fixed wish |
| `uninstall` | `jellymoon uninstall` removes a stand-in app and command and leaves their folders empty; the hook is out and the settings are as they were |
| `window` (with `--window`) | `jellymoon-app --page` opens a window and, with `JELLYMOON_QUIT_AFTER=3`, exits 0 within 30 seconds with no panic |

A made-up run starts each program with no variable of the caller's: a scratch `HOME`, `CLAUDE_CONFIG_DIR`,
`JELLYMOON_HOME`, `JELLYMOON_APP_DIR` and `JELLYMOON_BIN_DIR` in the temporary home, and `JELLYMOON_CLAUDE` naming the copy of
`fake-claude` there. It never runs Claude Code and never reads a real log. The `copy` and `run` steps alone are
given the caller's `PATH`, to find git, with an empty global git config and the system's switched off. The `run`
step names its harness, a copy of `fake-agent`, with `JELLYMOON_CLAUDE`, so no harness on that `PATH` is looked for.

## The three modes

- **Default.** Made-up logs and `fake-claude`. CI runs it on macOS, Linux and Windows after the tests.
- **`--window`** adds the window step. It needs a display, so that step alone sees the caller's variables. CI runs
  it on macOS, where a failure fails the job: the hosted runner opens the GPUI window and closes it.
- **`--real`**, for a maintainer on their own machine, never in CI or by an agent. The same steps over today's real
  logs and the real Claude Code on `PATH`: one real reading call. `logs`, `found-off-path`, `today`, `today-again`, `empty-day`,
  `no-logs`, the delivery steps, `wish`, `copy` and `uninstall` are left out;
  `--skip` is the reader's `--skip`. What Jellymoon writes still goes to the temporary home. It prints counts, states
  and exit codes only: a failed run says how many bytes a program printed, never what. Its sixth step is `run`
  (F39): one real run of the person's Claude Code, in a repo the chain makes in the temporary home, on the fixed
  wish "add a file named HELLO.txt that holds the one word hello". The run is stopped after four minutes and the
  step after five, and it prints the outcome, how many stars the page shows and how many things were denied.

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
