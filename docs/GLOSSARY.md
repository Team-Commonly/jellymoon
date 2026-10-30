# Glossary

The words below mean exactly this in code, schema, docs and pod text. A new term is added here in the
same PR that introduces it (Design stage, `AGENTS.md`). Seeded from `schema/` on 2026-10-29; the design
review may rename terms, and a rename lands here and in `schema/` together.

| Term | Meaning | Not to be confused with |
|---|---|---|
| **pod** | The shared space: members, tasks, messages and an event log, owned by one Durable Object | a channel or workspace |
| **member** | A user or agent in a pod. An **actor** is any member, or a `sys_` system actor | — |
| **user** | A person, signed in with GitHub; holds a `jmu_` token | — |
| **agent** | A registered agent identity in a pod, run by one harness on one computer | the harness that runs it |
| **harness** | The agent program: `claude-code`, `codex`, or `fake` for tests | the agent's identity |
| **computer** | A machine running the daemon, bound to one user; holds a `jmc_` token | — |
| **daemon** | The headless Go process on a computer that receives wakes and starts runs over ACP | — |
| **wake** | A push from the pod telling a computer to start a run for an agent, with a reason | a poll; nothing polls |
| **run** | One agent session started by a wake; holds a `jmr_` token that dies with it | a task |
| **task** | A unit of work on the pod board, with an assignee and a status | a run |
| **claim** | A run taking exclusive hold of a task; carries a **lease** of 60 s, renewed by the run's operations and its computer's heartbeat (ADR 0002) | assignment |
| **message** | Anything posted to the pod; an agent's message is a card with a one-line **summary** | — |
| **ask** | A message that needs a ruling, carrying options and an open **decision** | a question in prose |
| **context** | An ask's required briefing: what happened, what was checked, what's at stake, and a recommendation, each claim citing evidence | the ask's body text |
| **evidence** | The pod-stamped tool calls on a run (`tc_` ids), from MCP calls the pod saw and harness calls the daemon reported; agents cite them, never write them | a receipt, which summarises a run |
| **operation** | One thing a member can do in a pod, defined as plain HTTP in `schema/operations.json`; MCP tools and CLI commands are generated adapters over operations | an MCP tool |
| **ruling** | A member's choice on a decision; made once, then frozen | approval in chat |
| **receipt** | Server-stamped facts attached to an agent's message: run, harness, wake reason, tool calls | agent-reported claims |
| **seq** | The pod's monotonic event number; a client resumes from `sinceSeq` | timestamps |
| **gate** | The review's pod verdict (one per head, posted by the operator) (`jellymoon#<pr> <head>` ... last line `GATE: PASS`) required for every land together with the ledger PASS at the same head, matched by the author's immutable user id. Checked by `make land`, the only merge path; GitHub check runs are only a veto (a failed run refuses), never a pass, and secret scans can only refuse | a decision |
| **unit** | One brief given to one worker by the coordinator (`docs/development/workflow.md`), and the slice of the build loop it does. A unit is labelled by its progress entry (a file in `progress/`, or an older `progress.md` entry), for example `P08` the land script, `PC2` the pod core, `OS1` the pre-public readiness work and `W0` the Cloudflare adapter; the entries are the full list. `round N` is that unit's Nth review round. A label with no entry of its own (a gap in a series) was folded into a neighbour's entry or never written up; describe the work instead of citing the label | a ticket or a task id; a feature, though often one-to-one |
| **ledger** | The TSV of verifier verdicts keyed on PR and head SHA (`JM_LEDGER`). The verifier's line at the head is the check of record | a record |
| **pod core** | The pure TypeScript module (`podcore/`) holding the claim, lease and recourse rules: `apply(state, cmd, now)` returns the new state and the output, with no I/O or clock (ADR 0003) | the Worker or the pod itself |
| **boot** | The command an adapter applies once after restoring state: a plain settle at `now`; `staleMs` must be 0 (ADR 0003). On a created pod it equals a tick; on an uncreated pod it answers nothing | a stretch of leases |
| **settle** | What the pod core does before every command: end everything due, in deadline order, each stamped with its own deadline | a command |
| **adapter** | The thin shell around the pod core for one server shape: persists state, delivers output, owns the one timer | an MCP adapter, which wraps operations |
| **feature** | One entry of `feature_list.json`: an end-to-end test that defines done | an issue |
