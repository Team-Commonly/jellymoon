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
| `layout-sticker`, `layout-plain` | `jellymoon-app layout` on that page, in each built-in theme: every wish is a character and the clash is a choice note |
| `broken-page` | a file that is not page data is refused in one line, exit 1, nothing on stdout |
| `empty-day` | a day nobody typed in is an empty page, made with no reading call, and the app draws it |
| `hook-install` | `jellymoon hook install` puts its entry into made-up harness settings in the temporary home |
| `rule` | `jellymoon rule` keeps a ruling, and one waits in the store |
| `hook-told`, `hook-again` | `jellymoon hook`, given a made-up step, hands the session the ruling; its next step is told nothing |
| `uninstall` | `jellymoon uninstall` removes a stand-in app and command and leaves their folders empty; the hook is out and the settings are as they were |
| `window` (with `--window`) | `jellymoon-app --page` opens a window and, with `JELLYMOON_QUIT_AFTER=3`, exits 0 within 30 seconds with no panic |

A made-up run starts each program with no variable of the caller's: a scratch `HOME`, `CLAUDE_CONFIG_DIR`,
`JELLYMOON_HOME`, `JELLYMOON_APP_DIR` and `JELLYMOON_BIN_DIR` in the temporary home, and `JELLYMOON_CLAUDE` naming the copy of
`fake-claude` there. It never runs Claude Code and never reads a real log.

## The three modes

- **Default.** Made-up logs and `fake-claude`. CI runs it on macOS, Linux and Windows after the tests.
- **`--window`** adds the window step. It needs a display, so that step alone sees the caller's variables. CI tries
  it on macOS without failing the job, to learn whether a hosted runner can open a GPUI window.
- **`--real`**, for a maintainer on their own machine, never in CI or by an agent. The same steps over today's real
  logs and the real Claude Code on `PATH`: one real reading call. `logs`, `empty-day`, the delivery steps and `uninstall` are left out;
  `--skip` is the reader's `--skip`. What Jellymoon writes still goes to the temporary home. It prints counts, states
  and exit codes only: a failed run says how many bytes a program printed, never what.

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
