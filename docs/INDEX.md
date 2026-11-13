# Docs index

Every file under `docs/` is listed here with one line. A PR that adds, moves or removes a doc edits this list.

| File | What it is for |
|---|---|
| `GLOSSARY.md` | Terms used precisely across code, docs and tests |
| `adr/0011-the-journal-app.md` | The design: a local app that reads agent sessions into one page per day; the reader, the reading call, the kit, the GPUI shell, how a ruling reaches a running session, and a run's own copy of the repo |
| `adr/0012-restart-as-the-journal-app-in-rust.md` | Why the repo is Rust only and this small: what the restart decided, what it removed and where the old code remains |
| `adr/0013-the-shell-dependencies.md` | What the kit and the shell depend on: GPUI's pinned commit and how to move it, `ttf-parser` and what was rejected, the build features, and the one test that runs `sh` |
| `adr/0014-a-run-is-started-headless.md` | How a run is started: headless with the person's harness and not over ACP, what it may do fixed at its start, its work committed by core and kept on its branch; F30 cut into F37 to F40; what was measured on Claude Code 2.1.322, and what is not confirmed |
| `adr/0015-the-art-door-reads-with-the-renderers-parser.md` | Superseded by 0016. Why the kit once depended on `usvg` directly, at GPUI's version: the art door parsed a theme's picture with the renderer's parser and judged its tree |
| `adr/0016-theme-files-name-no-pictures.md` | Why a theme file names no pictures: four reviews each found an accepted picture the renderer choked on, the trial design that was built and closed, what was removed, and when and from where art in a theme comes back |
| `adr/0017-a-runs-work-is-shown-kept-or-tossed.md` | The loop a person plays (wish, watch, answer, see, keep or toss) and its last two steps: how a run's work is shown from the repo's store, kept as one commit on the person's branch or tossed, what is refused, and features F44 to F47 |
| `adr/0018-a-run-asks-and-is-told-more.md` | Step 3 of the loop: a run that must ask ends with its question as data, the person's answer resumes the run's kept session in a new process, a follow-up is a new made wish on a fork of that session, why the reader leaves out a run's copy, what was measured, and features F50 to F54 |
| `development/workflow.md` | How Jellymoon is built: the loop, the size rules, the fresh-context review and its verdict lines, and how a PR lands |
| `development/reader.md` | `jellymoon day`: usage, what counts as typed by a person, what is cut and replaced, which files are skipped |
| `development/page.md` | `jellymoon page`: usage, the exact harness command line, what goes in on stdin, and every check code makes on what the model returns |
| `development/delivery.md` | `jellymoon rule` and `jellymoon hook`: the ruling store, the Claude Code hook event and output used and why, how exactly-once is kept, install and uninstall, and what is unproved |
| `development/wish.md` | `jellymoon wish`: a made wish, where and how it is kept, every refusal, how it comes onto the page, and `--run`, which starts its run |
| `development/app.md` | `jellymoon-app`: usage, which piece each part of page data becomes, the `layout` output, the two ways text is measured, and how to build with and without GPUI |
| `development/copy.md` | `jellymoon copy make` and `copy clean`: where a run's copy and its record are, what cleanup never looks at, every git setting switched off and what could not be, who the commit is by, and the limits |
| `development/run.md` | `jellymoon run`: the order of a run, the harness's exact command line and variables, what is read of its stream and when a run has finished, the time limit and the stop, what is kept and what the record names of the repo, a run that was cut off, what a kill leaves in the record, a run started apart by `jellymoon wish --run`, the three runs that go at once, and every refusal |
| `development/show.md` | `jellymoon run show`: what a finished run made, read from the repo's store with nothing written: what is printed, how the run and the repo are found, what the record names and how far it is trusted, every question asked of git and where reading stops, the cleaning, the names tools obey, every refusal, and what it does not know |
| `development/keep.md` | `jellymoon run keep`: a finished run's work landed on the branch the person stands on: what is printed, `--ask` and `--with-settings`, the checks and the five steps with what a cut after each leaves, the one git command that writes the checkout and why no hook, filter or config of the repo can start a program, every question asked of git, every refusal, and what is not closed |
| `development/install.md` | Installing on a Mac: what `install.sh` does and trusts, why Gatekeeper does not ask, `jellymoon uninstall`, how a release is cut, and what only a person on a clean Mac can prove |
| `development/smoke.md` | `cargo xtask smoke`: the chain that runs the built programs together, its three modes, what it does not prove, how a feature adds a step, and the checklist before a release |
| `runbooks/old-hosted-instance.md` | The hosted instance of the removed product, which may still be running: what it is, what this repo no longer does to it, and how to stop it |
| `skills/SOURCES.md` | Where the vendored stage skills in `.agents/skills/` come from, their pins and how to update them |
