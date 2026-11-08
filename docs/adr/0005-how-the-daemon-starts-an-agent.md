# 0005 · How the daemon starts an agent

Status: Accepted (maintainers, 2026-10-30; the run claim work, F09, #3). The open items at the end are not settled by it.

## Decision

On `run.wake` the daemon acks the wake, calls `POST /v1/runs` with its computer token, and starts the agent for that run.

- **The agent is a child that speaks ACP on stdio** (JSON-RPC 2.0, one message per line): `initialize`, `session/new`
  with `cwd` and `mcpServers`, then `session/prompt` with the brief. It runs in its own process group. Code: `daemon/internal/acp`
  (the socket and process work is the adapter's, ADR 0003); the claim, the checks and the run end are `daemon/internal/daemon/run.go`.
- **The run token travels once, in the MCP server config of `session/new`** (an `Authorization` header on the pod's `/mcp`).
  It is never in argv, and the agent's environment is a short allow-list (`PATH`, `HOME`, `TMPDIR`, `USER`, `LANG`, `LC_ALL`,
  `TERM`), so the daemon's own variables, the user token and the computer token never reach it.
- **The MCP URL is checked before the run token is used**: https, or http on localhost or 127.0.0.1 (the server rule, in
  `api.CheckMCP`). A refused URL never starts the agent and the run is ended `failed`.
- **The program for a harness comes from `JELLYMOON_HARNESS_<NAME>`** (the harness in capitals, `-` as `_`; the program and its
  arguments, split on spaces). Only `fake` is used so far; `claude-code` and `codex` get their own adapters later.
- **A run ends `completed` when the turn ends and `failed` otherwise** (agent error or exit, refused URL, daemon stopping), through
  `POST /v1/runs/:run/end`. When the daemon stops it terminates every agent's process group (SIGTERM, then SIGKILL after 2 s).
- **The Worker serves `/mcp`** as the generated adapter over the operations: each tool is its operation's own HTTP route, called
  with the caller's token (`worker/src/mcp.ts`).

## Open items

- A daemon killed with SIGKILL cannot signal its agent. A well-behaved agent exits when its stdin closes; one that ignores that stays.
- ~~The agent's working directory is the daemon's.~~ Closed 2026-11-08 by the update below.
- `run.stop` is not handled yet, and neither are a second wake for an agent that is mid-run or a tool call report (`run.toolcall`).

## Update 2026-11-08: every run gets its own working directory

The daemon makes a directory for each run before it starts the agent, passes it as `session/new`'s `cwd`, and removes it when
the run ends. Code: `daemon/internal/workspace`, called from `daemon/internal/daemon/run.go`.

- **Where.** `<config dir>-workspaces`, the config dir's sibling (0700), named for the run id: `<root>/<run id>`. Never the
  agent's repo and never the daemon's own working directory. Run ids come from the pod and are checked as plain names.
- **What.** An agent added without `--repo` gets a new empty directory. An agent added with `--repo <path>` gets a git worktree
  of that repo's HEAD on the branch `jellymoon/run-<run id>`. A run id is used once, so the branch cannot collide.
- **When it goes.** A `completed` run's directory is removed, with its worktree record and branch. A `failed` run's stays for
  inspection, marked by an empty `<run id>.kept` file beside it, and only the newest K = 5 stay: enough to look at the last few
  failures, few enough that an agent failing in a loop (each worktree is a full checkout) cannot fill the disk.
- **After a crash.** A daemon killed mid-run leaves its run directories. On start, once the control socket is held (so no other
  daemon runs on this config dir), the daemon removes every run directory not marked kept, and any marker whose directory is gone.
- **The guard.** Cleanup removes a path only if, once cleaned, it is a valid run name directly inside the resolved root and a real
  directory there. A `..`, a path elsewhere and a symlink (even one inside the root) are refused. It deletes only the branch
  named for that run. The run is over when cleanup happens, so what the directory holds, uncommitted work included, is
  disposable; that is why it needs no `--force` decision.
- **Not covered.** A worktree shares its repo's object store and config, so an agent with write access to the worktree can
  change that repo's git config; the allow-listed environment and the run token are the boundary, not the checkout. Two agents
  on one repo run in separate worktrees but can still push the same remote branch.
