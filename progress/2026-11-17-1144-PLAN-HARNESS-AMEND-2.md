# PLAN-HARNESS-AMEND-2: the amendment of ADR 0021, read before it merged

**What changed.** On branch `plan-amend-2`, on `11147ef` (the amendment, which sits on main `bb2a384`). The
amendment was read before it merged and failed on one fault with nine notes. No Rust.

- **Steps no command can show.** F79 step 3 and F80 steps 2 and 3 are reworded to what a command shows. The
  panic's line and the rule of one line for a code are tests of functions riding in F79 and F80. No way to
  panic on purpose is shipped.
- **Dead rows are in the list.** `feature_list.json` has a key `dead` beside `features`: F67, F68, F70 and F73,
  each with the rows that stand in for it. The scoreboard prints `dead` for them (`init.sh`), and step 3 of
  `AGENTS.md` skips them and takes the ADR's table for the order of ADR 0021's rows.
- **The door** also refuses a full-path `CLAUDE_CONFIG_DIR`, `--logs`, `--settings` and `--repo` outside the
  temporary folders, with links resolved, and F74 names the four tests that start `jellymoon` through another
  program.
- **The login shell's wait is 5 seconds**, and its group is gone before the command returns. `app.log` has no
  lock file. The store's place has one rule, pinned at `jellymoon day` and at `layout` in F69.

`2026-11-17-1045-PLAN-HARNESS-AMEND.md` says "on main `02393d1`"; that commit sits on `bb2a384`.

**The evidence.** `cargo fmt --all --check` and `cargo xtask check`, with this file staged. The scoreboard's
awk program alone, run on the list: F67, F68, F70 and F73 print `dead`, "31/57 passing, 4 dead". Only `init.sh`
reads the list, so xtask needed no change. `./init.sh` whole was not run: it installs tools.

**What's next.** F74, the door.

**Surprising.** Rows F74 to F82 were reworded here because they were not on main yet. Once they are, they are
held to the rule like any other.
