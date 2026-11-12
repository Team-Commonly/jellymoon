# 0014 · A run is started headless, and its work stays on its branch

Status: Accepted, by the coordinator's decision, 2026-11-10, when F30 was cut into slices (F37 to F40).
[ADR 0011](0011-the-journal-app.md), section 7, said how a run is started and stopped in words carried over from the Go
program this repo used to hold. This note changes three of those sentences and one about the run's branch. Section 7 is
edited to match and points here. Nothing else in ADR 0011 changes.

## Decisions

1. **A run is a headless child of the person's own harness, not a child over ACP.** Core starts `claude` with `-p` and
   `--output-format stream-json`, the way the reading call is started already, and reads its stream. ACP needed an
   adapter that is a Node package (the Go program used `@agentclientprotocol/claude-agent-acp`). That fails the install
   bar: a person installs Jellymoon and has Claude Code, and nothing else is asked of them. A starter for Codex goes
   behind the same seam later.
2. **Refusing by default is the harness's own rule, fixed when the run starts.** There is no permission request to
   answer while a run goes: what the run may do is a list given at its start, and anything that would ask is denied by
   the harness and counted. The person's own allow rules, MCP servers and project hooks never apply to a run, and a
   run's session is not kept, so the next day's reader never reads a run back as something the person typed. The first
   list is reading and editing files inside the run's copy. No shell, no web, no MCP. Allowing a shell is a later
   switch the person turns on in so many words.
3. **A stop interrupts, then signals, then kills.** There is no `session/cancel` without ACP. Core asks the child to
   stop, waits, signals one that did not, and kills one deaf to the signal.
4. **The work stays on its branch.** With no shell the agent cannot commit. When a run ends, core commits what is in
   the copy, with git's hooks off, on the run's own branch, and removes only the worktree folder. The branch is kept:
   it is the work. ADR 0011 said the copy was "removed with its branch when the run completes".
   [ADR 0017](0017-a-runs-work-is-shown-kept-or-tossed.md) says what becomes of the branch once the person has seen
   the work.
5. **The star is core's.** Core adds a wish's star when the stream's result is a success and the child exits 0. The
   star's words are fixed and are never the agent's.
6. **A made wish names one project.** A name that matches two folders is refused and nothing is written.
7. **A made wish is kept beside the day's page, not in it.** A reading rewrites `pages/<day>.json`, so made wishes
   live in `pages/<day>.wishes.json`, written under the day's lock, and are merged when the page is drawn, the way a
   settlement is. Their ids can never be one the reading gives. A run's record goes in `runs/<id>`.

## What ADR 0011, section 7, now says

| It said | It says |
|---|---|
| "A child over ACP ... a child process speaking ACP on stdio" | A headless child of the person's harness, with the same short list of variables |
| "A stop sends `session/cancel` first and waits" | A stop interrupts first and waits; then the signal; then the kill |
| "Where the person has turned allowing on, the answer is for that one call" | What a run may do is a list fixed at its start; nothing is asked or answered while it goes, and no allow is remembered |
| "removed with its branch when the run completes" | Core commits the copy with hooks off; the folder is removed and the branch is kept |

## The slices

F30 stays as it was written and is built last. `feature_list.json` lets only `passes` change, so the edge is stated
here: **F30 waits on F40.**

| Feature | What it adds | Waits on | Changes what leaves the machine |
|---|---|---|---|
| F37 | `jellymoon wish`: a typed wish is kept and drawn, with no run | nothing | no |
| F38 | `jellymoon copy make` and `clean`: a run's copy, and its safe removal | nothing | no |
| F39 | `jellymoon run`: the harness started in the copy, and the star | F37, F38 | yes: it waits for `GATE: PASS` |
| F40 | `jellymoon run stop`, and denials counted | F39 | yes |
| F30 | The held key: a helper prints the words, and core treats them as a typed wish | F40 | no |

## Measured

The coordinator measured these on Claude Code 2.1.322, on a Mac, in a scratch repo, before F39 was built. F39
starts a run with exactly this command line (`docs/development/run.md`).

- **The command line.** `-p --restricted --strict-mcp-config --permission-prompts none --permission-mode acceptEdits
  --tools Read Edit Write Glob Grep --no-session-persistence --output-format stream-json --verbose`, with the wish
  on the input.
- **What the harness then has.** The stream's first line is `system`/`init`: the tools are `Edit`, `Glob`, `Grep`,
  `Read` and `Write`, there are no MCP servers, and the permission mode is `acceptEdits`.
- **A write outside the folder is denied.** It gives one `system`/`permission_denied` line, and the result's
  `permission_denials` then holds one entry, which names `Write`.
- **The end.** The last line is `type: result` with `subtype: success` and `is_error: false`; the exit is 0 and
  stderr is empty. No session is kept.

F39 adds one dependency for this: `rustix`, to core, under `[target.'cfg(unix)'.dependencies]` only, with its
default features off and `std` and `process` on. The standard library can only kill its own child outright
(`Child::kill`): it has no call that asks a process to stop first, and none that signals a process group or a pid
it did not start, and `rustix::process::kill_process_group` is that call without `unsafe` (`run/child.rs`,
`run/dead.rs`). It is the version `tempfile` already brings into the build, so no new package is downloaded.
`cargo xtask check` asks nothing more of a new dependency than `entropy.toml`'s `[depends]` and `[forbidden]`,
which it passes; this note is the rule's.

## The stop (2026-11-11, F40)

Decision 3 is built as `jellymoon run stop --wish <id>` (`docs/development/run.md`, "A stop the person asks for").

- **The stop is a request, and the run stops itself.** The child belongs to the `jellymoon run` that started it.
  `run stop` leaves an empty file, `runs/<id>/stop`, and that `jellymoon run` finds it while it waits for its
  child. This works the same on every system, and only one process ever ends the child and writes the end, so a
  stop against the run's own finish gives one end. Signalling the child from `run stop` was not chosen: two
  processes would then race to write the end, and Windows has no signal to send.
- **Only the file's being there is read.** A request left by hand stops the run whose folder it is in and does
  nothing else. It needs no lock.
- **The interrupt is `SIGINT`** to the child's group, then five seconds, then `SIGTERM`, then five seconds, then
  `SIGKILL`. On Windows the child is ended at once.
- **`cancelled` is a fourth outcome.** The wish needs the person, has no star and has had its one run; the copy
  is kept as a failed run's is. A stop decides the end even after a result that said success.
- **A run that was cut off is not cancelled by a stop.** `run stop` writes it down as cut off, as a page does.
- **Denials are counted and never decide anything.** The list is fixed by the command line and checked on every
  init line; the measured stream shows nothing else of what was allowed, so no other check was invented.

## Still open

- **Interrupting.** Whether the stream mode takes a supported interrupt message on its input was not measured, nor
  whether Claude Code ends its work cleanly at `SIGINT`, nor how a child is interrupted on Windows. A stop sends
  `SIGINT`, then the signal, then the kill; on Windows, the kill.
- **A repo with its own instructions.** A run in a repo that has its own `CLAUDE.md`, settings or hooks was not
  measured, nor the harness's exact list of protected paths, and so whether `.git` inside the copy is covered by
  the edit allow.
- **Windows, and an older Claude Code.** Neither was measured. A Claude Code too old to know `--restricted` is
  told by its exit and the words `unknown option` and `--restricted` on stderr. Those words are what such a harness
  is expected to print, not what one was seen to print: a harness that says it another way gives a failed run with
  its copy kept, never a run without the flag.
- **Workers never run the real `claude`.** Every test uses a made-up agent that prints a scripted stream.
