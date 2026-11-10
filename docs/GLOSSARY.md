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
| **presence** | Whether a member is `online`, `offline` or `working`. An agent is online while its computer is: a socket to the pod, and a sign of life (a heartbeat) within 45 s. Current state, read from hello; `presence.changed` has no seq and is not replayed (ADR 0003) | a message or an event in the log |
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
| **receipt** | Server-stamped facts attached to an agent's message: run, harness, wake reason, tool calls (the MCP `tools/call` requests the pod had answered on the run's token before that post, refused ones included; `initialize` and `tools/list` are not counted) | agent-reported claims |
| **seq** | The pod's monotonic event number; a client resumes from `sinceSeq` | timestamps |
| **gate** | The review's pod verdict (one per head, posted by the operator) (`jellymoon#<pr> <head>` ... last line `GATE: PASS`) required for every land together with the ledger PASS at the same head, matched by the author's immutable user id. Checked by `make land`, the only merge path; GitHub check runs are only a veto (a failed run refuses), never a pass, and secret scans can only refuse | a decision |
| **unit** | One brief given to one worker by the coordinator (`docs/development/workflow.md`), and the slice of the build loop it does. A unit is labelled by its progress entry (a file in `progress/`, or an older `progress.md` entry), for example `P08` the land script, `P09` the pod core, `OS1` the pre-public readiness work and `W0` the Cloudflare adapter; the entries are the full list. `round N` is that unit's Nth review round. A label with no entry of its own (a gap in a series) was folded into a neighbour's entry or never written up; describe the work instead of citing the label | a ticket or a task id; a feature, though often one-to-one |
| **ledger** | The TSV of verifier verdicts keyed on PR and head SHA (`JM_LEDGER`). The verifier's line at the head is the check of record | a record |
| **pod core** | The pure TypeScript module (`podcore/`) holding the claim, lease and recourse rules: `apply(state, cmd, now)` returns the new state and the output, with no I/O or clock (ADR 0003) | the Worker or the pod itself |
| **boot** | The command an adapter applies once after restoring state: a plain settle at `now`; `staleMs` must be 0 (ADR 0003). On a created pod it equals a tick; on an uncreated pod it answers nothing | a stretch of leases |
| **settle** | What the pod core does before every command: end everything due, in deadline order, each stamped with its own deadline | a command |
| **adapter** | The thin shell around the pod core for one server shape: persists state, delivers output, owns the one timer | an MCP adapter, which wraps operations |
| **feature** | One entry of `feature_list.json`: an end-to-end test that defines done | an issue |
| **connection** | An upstream service a pod owner connected, with the credential the pod holds for it, stored only as an envelope (ADR 0009) | a grant |
| **grant** | What one agent may do on one connection: `read`, or `write-with-confirm` (writes park as an ask) | membership |
| **pin** | A vendor's pinned catalogue in the repo: each operation's name, class (read or write) and argument names. An upstream that differs from it is **drift** | the upstream's live list |
| **parked call** | A write held behind its ask until a ruling: approve releases it once, reject or expiry releases nothing | a task |
| **connector receipt** | The server's record of one connector call: operation, args hash, run, outcome, and for a write its decision and approver | a post's receipt |
| **page** | One local day of one person's wishes, read from their agent sessions and shown by the journal app (ADR 0011) | a web page; a pod |
| **wish** | Something a person wants to exist or be true, worked out from what they typed to their agents; carries their own words and a state: working, done, needs you or let go | an ask, which needs a ruling; a task on the pod board |
| **little one** | The character that stands for one wish on a page | an agent |
| **star** | One finished job an agent reported for a wish | a receipt, which the server stamps |
| **clash** | Two things one person said that cannot both stand. Shown as an ask; the person's tap is the ruling | a merge conflict |
| **reader** | The part of the daemon that collects one day of what the person typed from session logs | the reading call, which is the model's part |
| **reading call** | The one headless model call, through the person's own harness with every tool off, that proposes a day's wishes and clashes | the reader, which is code; a run |
| **page data** | What a page is drawn from: the day's wishes, stars and clashes after code has checked them (`schema/page.json`) | the reading call's raw reply |
| **kit** | The fixed set of drawable pieces a page is arranged from. The model picks pieces; it never draws one (ADR 0011) | generated UI |
| **theme** | How a page looks: colours, fonts, paper, the art for each piece and how much things move. Data and art only, never code (ADR 0011) | the kit, which is the pieces and their rules |
| **pack** | Extra pieces added to the kit as data and art only | a plugin, which would run code |
