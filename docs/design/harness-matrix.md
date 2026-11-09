# Harness matrix: agent CLIs over ACP

Measured 2026-11-08 on one macOS arm64 laptop, through the Go daemon, against a local server (loopback, one pod, one task).
Everything not measured is marked with its source and the date it was read (2026-11-08). No adapter code changed.

Versions measured:

| Harness | CLI | ACP adapter |
| --- | --- | --- |
| Claude Code | 2.1.318 | `@agentclientprotocol/claude-agent-acp` 0.99.1 (the pin in the Makefile) |
| Codex | 0.162.1 | `@agentclientprotocol/codex-acp` 2.6.0 (bundles its own Codex binary) |

The older `@zed-industries/codex-acp` 0.16.0 is deprecated on npm in favour of the `@agentclientprotocol` package, so the newer one was used.

How it was run: the daemon started each adapter exactly as it starts any agent (stdio ACP, a clean environment of PATH, HOME, TMPDIR, USER, LANG),
through a recording tee. The agent got the daemon's own brief ("... Call jm_whoami, then jm_task_list, claim a task before working on it and report
with jm_post."), not a custom prompt, because the daemon builds the brief and this unit may not change code. The machine's existing sign-in was used;
no key, no new login. Four real turns in total: one full turn and one cancelled turn per harness.

## Matrix, measured

| Check | Claude Code | Codex |
| --- | --- | --- |
| 1. Handshake | `initialize` answered, protocol version 1. Capabilities: image and embedded-context prompts, MCP over `http` and `sse`, `loadSession`, session list/fork/resume/close/delete. No auth methods advertised (it reads the local sign-in). | Protocol version 1. Prompt capabilities as Claude; MCP over `http` only (`sse` false); session list/fork/resume/close/delete, plus `subagents` and `title`. Advertises two auth methods (API key, ChatGPT sign-in); worked with the existing sign-in. |
| 2. `cwd` and `mcpServers` | Both honoured. The adapter connected to the pod's MCP URL with the run token and listed the `jm_*` tools. The model first had to look the tools up (a ToolSearch step) before calling them. | Both honoured. Connected at `session/new` time and listed tools, with no extra lookup. Tools show as `mcp.jellymoon.jm_*`. |
| 3. One real turn | Called `jm_whoami`, `jm_task_list`, `jm_task_claim`, then `jm_post`. The pod stored the post and returned a message id and sequence number; the stored post also appears on the pod's socket as `message.created`. 36 s from task assigned to run ended. | Same four tools, same outcome with a message id and sequence number. 51 s from task assigned to run ended. |
| 4. `session/request_permission` | None sent in the run. The session starts in "auto" mode, so Claude decides itself. | None sent. Mode was "agent", and each MCP call showed a "Guardian Review" step inside Codex that approved it without asking the client. |
| 5. `session/cancel` mid-turn | The daemon never sends `session/cancel`. Stopping the daemon mid-turn ends its context, which signals the adapter's process group. The daemon logged "context canceled" and ended the run as failed within the same wall-clock second; no Claude process was left. Exact milliseconds were not captured (see findings). | Same path. Every adapter and Codex process, plus the daemon, was gone 0.17 s after the signal. |
| 6. Exit | After a normal run end, no adapter or CLI process was left (three processes at cancel: the tee, the adapter and the CLI, all in one group). | Same (tee, adapter, Codex app-server in one group), none left. |

## Matrix, not measured (read 2026-11-08)

"ACP" says how the harness speaks ACP according to its own docs. Permissions and headless use are as the docs describe them.

| Harness | ACP | MCP config | Permission model | Headless | Licence | Source |
| --- | --- | --- | --- | --- | --- | --- |
| Gemini CLI | Native, `gemini --acp` | `settings.json` in the user-level `.gemini` directory; the ACP page says the client registers its server at `initialize` (the exact `session/new` shape is not on that page) | Asks the client via `session/request_permission` for MCP tools; `setSessionMode` can switch to auto-approve | `--output-format json` / `stream-json` documented | Apache-2.0 | the project's README and its ACP-mode doc page |
| OpenCode | Native, `opencode acp` | not stated on the ACP page | "Agents and permissions system" listed as supported; behaviour in ACP not described | not stated on the ACP page | not read | docs.opencode.ai/docs/acp |
| Qwen Code | Native, `qwen --acp` | not stated on the pages read | not stated | `qwen -p "..."` | Apache-2.0 | the project README and its Zed integration page |
| Goose | Listed as ACP-capable (topic and test client in the repo); the docs page read describes Goose as an ACP client of other agents and does not show a `goose acp` command | extensions, passed on to ACP agents as MCP servers | `GOOSE_MODE` auto / smart-approve / approve / chat | not read | Apache-2.0 | the repo README and its ACP providers guide |
| Cline | Native, `cline --acp` | not stated | Nothing auto-approved by default; per-session or `--auto-approve true` | not read | not stated | docs.cline.bot, ACP editor integrations |
| GitHub Copilot CLI | Native, `copilot --acp` (stdio or `--port`), public preview | per session via `session/new` `mcpServers`; no server-side config described | client implements `requestPermission` | not read | not stated | GitHub Docs, Copilot ACP server page |
| Cursor CLI | Native, `agent acp` | `.cursor/mcp.json`, project or user; team servers not supported in ACP | sends `session/request_permission` (allow-once, allow-always, reject-once); an unanswered request can block the tool | not read | not stated | Cursor docs, CLI ACP page |
| Aider | None found (ACP and MCP not mentioned in its README) | none found | not applicable | command-line and Python scripting | Apache-2.0 | the repo README |

The ACP project's public agent list (agentclientprotocol.com/overview/agents) names many more (Kiro CLI, Kimi CLI, Mistral Vibe, Junie, OpenHands,
Factory Droid, Augment, Pi and others). That page gives support modes for only a few (adapters for Claude, Codex, Pi, Bub), so the rest are listed
there without detail and were not chased.

## Findings

- **Nothing is blocking, and the daemon is thinner than the harnesses expect.** It answers every agent-initiated request with "not supported" and
  never sends `session/cancel`. Both harnesses got through a full slice anyway, but only because neither asked. A harness that asks first (Cursor, per
  its docs, can block until answered; Cline and Gemini ask by default) would stall the run with the current client.
- **Neither harness asked permission, for different reasons.** Claude runs in an "auto" mode that the daemon never chose; Codex approves MCP calls
  internally. The daemon has no say in either today. Whether that is the intended trust model is a decision, not a measurement.
- **Cancel is process death, not protocol.** Run end and cancel both end in a signal to the process group. That was clean for both, but it gives
  the model no chance to finish a tool call, and the pod only learns the run "failed".
- **The pod rejected the first two `jm_post` attempts in both harnesses**, because the model sent `taskId` and then `kind: result`, which the server
  does not store yet. Both recovered on their own by retrying as a plain `fyi`. Two bad calls out of three is a signal that the tool description and
  the server's current limits disagree.
- **Tool discovery differs.** Claude loads the MCP tool list lazily, behind a lookup the model must do; Codex has them up front. Time to first tool
  call was not separated from total time.
- **Claude's cancel time is bounded, not measured.** My poller counted an unrelated server process as "still running", so it never reported a time.
  The turn cannot be repeated inside the turn budget. The daemon's own log puts it inside one second.
- **Sign-in details leak into the protocol.** Both adapters send an account status update (plan and email) over ACP. Anything that records ACP traffic
  needs a scrub step (the existing recorder already has one).
- **Sample size is one turn per harness.** Wall times (36 s and 51 s) are single runs on a loaded laptop, not benchmarks.

## Recommendation: Codex is the next adapter after Claude Code

Reasons:

1. **It already works on the unchanged client.** The measured run completed the whole slice (handshake, MCP tools, a stored post with a receipt, a clean
   exit) with no daemon change, which is the cheapest possible next step. Its adapter and CLI are
   Apache-2.0 and ship as a single repo-local npm install.
2. **It is a second, different runtime shape.** It differs from Claude in MCP transport (`http` only), permission handling and tool naming, so it
   will expose any Claude-specific assumption in the daemon before a third harness piles on more.

Risk: the permission behaviour is inside Codex (its own review step), not something the daemon sets, and the daemon will need a real answer to
`session/request_permission` before any harness that asks (Gemini, Cursor, Cline) can be added. Gemini CLI (native ACP, Apache-2.0, documented
headless mode) is the natural third, once permissions are handled.
