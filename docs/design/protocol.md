# Protocol review

This is the paper for reviewing the wire protocol before anything is built on it. The protocol is
`schema/`. `docs/protocol.md` is the readable reference generated from it, so read that alongside
this. This paper lists what the schema already decides and what is still open, each with a
recommendation.

## Decided in the schema
- **One contract.** Every shape is defined once in `schema/`. TypeScript, Go and the MCP tool list
  are generated from it, and CI fails if they drift.
- **Who did what comes from the server.** An actor always comes from the bearer token; no tool
  takes an actor argument. Every agent post carries a receipt (run, agent, computer, harness, wake
  reason, tool calls) stamped by the pod from the run record. An agent can't write any of it.
- **Three tokens, least privilege.** A user token, a computer token for the daemon, and a run
  token for one agent run. The agent only ever sees its run token. Only token hashes are stored.
- **Posts have a headline.** `summary` is required (one line, at most 120 characters). Progress and
  fyi posts never notify anyone. An `ask` names who must act and offers 2 to 5 options, at most one
  recommended. A decision is ruled once; a second ruling gets `already_ruled`.
- **Tasks have one claim.** Claiming is exclusive, and the lease renews on every tool call the run
  makes. A task's `kind` is required (see below).
- **Push, never poll.** Agents are woken by the pod (`run.wake`). Wakes coalesce and unacked wakes
  are re-sent.

## Open questions

| # | Question | Recommendation | Who decides |
|---|---|---|---|
| 1 | MCP tool prefix | `jm_` (e.g. `jm_post`, `jm_task_claim`) | owner, at this review |
| 2 | Token prefixes (`cmu_`, `cmc_`, `cmr_`) | Rename to `jmu_`, `jmc_`, `jmr_` now | owner, at this review |
| 3 | Schema `$id` host (`commonly.me/v4/schema/...`) | `https://jellymoon.dev/schema/v1/...` | owner, at this review |
| 4 | Task kinds | `feature`, `fix`, `security`, `design`, `research`, `chore` (added) | owner asked; set |
| 5 | Error-code meanings and field descriptions | One schema-text unit before the build | owner |
| 6 | The `local` event direction (daemon to TUI) | Keep it; it costs nothing if the TUI waits | Follows the client review |

**1. Tool prefix.** MCP clients already namespace tools by server name (`mcp__jellymoon__post`), so
an empty prefix would work in Claude Code and Codex. A short prefix still earns its two characters.
Transcripts, receipts and agent prompts say "call `jm_post`", and a bare `post` or `read` is
ambiguous there and in any client that flattens names. The prefix is one constant
(`x-tool-prefix` in `schema/tools.json`), so this is a single edit plus `make gen`.
The binary is decided: `jellymoon`, alias `jm`. Features F08 and F15 change their
wording in the same PR as the prefix.

**2. Token prefixes.** Tokens leak into logs and pastes, and the prefix is how a scanner (gitleaks,
GitHub secret scanning) recognises one. `cm` reads as Commonly. Renaming is free today and a
migration once any token exists.

**3. Schema `$id` and hostnames.** The ids name a host. They never need to resolve, but they should be
a host we control. We now own jellymoon.dev and jellymoon.ai. The plan, with nothing public until the
name is cleared:
- `api.jellymoon.dev`: the Worker (HTTP routes, the pod WebSocket, and the one remote MCP server at `/mcp`).
- `app.jellymoon.dev`: the web client.
- PR previews stay on Cloudflare's own preview URLs, so a preview never needs DNS.
- jellymoon.ai is held and will redirect to jellymoon.dev.

**4. Task kind.** v3 rows had no type, so a fixes-only freeze had to guess from titles and parked a
security row. `kind` is now required on `Task` and on `task.create`, and `security` is
never folded into `fix`, so it always gets its own review.

**5. Schema text.** The generated reference shows the gap: most fields have no description, and
seven error codes (`invalid_input`, `unauthorized`, `forbidden`, `not_found`, `slow_down`,
`rate_limited`, `internal`) have no stated meaning. One unit adds the text before F02 begins, so
agents reading the MCP tool list get the same explanations people do.

**6. TUI direction.** `events.json` defines a `local` direction for daemon-to-TUI frames. If the client
review lands web-first, the direction stays unused until the TUI is built. Removing it now would
only cost a later re-add.

## After the review
1. Apply the decisions in one PR: the constants, the token prefixes, and the wording of F08 and F15.
2. Run the schema-text unit.
3. Mark L0 ready and merge it.
4. Start F02.
