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

## Still open

- **Interrupting.** Whether the stream mode takes a supported interrupt message on its input is not confirmed, nor how
  a child is interrupted on Windows. Until it is, a stop is the signal and then the kill.
- **Hooks and protected paths.** Whether hooks load from a settings file given on the command line is not confirmed,
  nor the harness's exact list of protected paths, and so whether `.git` inside the copy is covered by the edit allow.
- **The flags.** The coordinator checked these against the `--help` of Claude Code 2.1.322: `--permission-mode` (with
  `dontAsk`), `--permission-prompts none`, `--strict-mcp-config`, `--setting-sources`, `--no-session-persistence`,
  `--tools`, `--allowedTools`, `--disallowedTools`, `--add-dir`, `--output-format stream-json`, and `--restricted`
  (no built-in tool that runs commands or code, no WebFetch, the person's, the project's and the local settings files
  ignored, file tools held to the working folders). F39 starts from `--restricted --strict-mcp-config
  --permission-prompts none` and a named `--tools` list. A Claude Code too old to know `--restricted` is refused in a
  plain line and is never run without it. What each flag does was read from `--help`, not measured: F39's tests prove
  what core passes, and a person's own run proves what the harness does with it.
- **Workers never run the real `claude`.** Every test uses a made-up agent that prints a scripted stream.
