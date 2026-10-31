# 0005 · How the daemon starts an agent

Status: Accepted (owner ruling, 2026-10-30), for the slice. The open items at the end are not settled by it.

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
- The agent's working directory is the daemon's. Per-run workspaces belong with the real harness adapters.
- `run.stop` is not handled yet, and neither are a second wake for an agent that is mid-run or a tool call report (`run.toolcall`).
