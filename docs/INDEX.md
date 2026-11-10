# Docs index

Every file under `docs/` is listed here with one line. A PR that adds, moves or removes a doc edits this list.

| File | What it is for |
|---|---|
| `GLOSSARY.md` | Terms used precisely across code, docs and tests |
| `adr/0011-the-journal-app.md` | The design: a local app that reads agent sessions into one page per day; the reader, the reading call, the kit, the GPUI shell, how a ruling reaches a running session, and a run's own copy of the repo |
| `adr/0012-restart-as-the-journal-app-in-rust.md` | Why the repo is Rust only and this small: what the restart decided, what it removed and where the old code remains |
| `development/workflow.md` | How Jellymoon is built: the loop, the size rules, the fresh-context review and its verdict lines, and how a PR lands |
| `development/reader.md` | `jellymoon day`: usage, what counts as typed by a person, what is cut and replaced, which files are skipped |
| `development/page.md` | `jellymoon page`: usage, the exact harness command line, what goes in on stdin, and every check code makes on what the model returns |
| `runbooks/old-hosted-instance.md` | The hosted instance of the removed product, which may still be running: what it is, what this repo no longer does to it, and how to stop it |
| `skills/SOURCES.md` | Where the vendored stage skills in `.agents/skills/` come from, their pins and how to update them |
