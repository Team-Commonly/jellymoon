# 0007 · The daemon answers an agent's permission requests by policy, and cancels a session before it signals

Status: Accepted (maintainers, 2026-11-08). It settles two ACP exchanges ADR 0005 left unhandled. One question stays open
(below).

## Decision

**Permission requests.** An agent may ask the client for permission before a tool call (`session/request_permission`). The
daemon always answers; it never leaves the request open and never answers `-32601` to it.

- **Default: deny.** The daemon answers with the request's own reject option: the first of kind `reject_once`, else
  `reject_always`. If the request offers no reject option, it answers the `cancelled` outcome.
- **Opt-in: an allow setting.** `jellymoon agent add <harness> --pod <id> --allow-permissions` stores `allowPermissions` with
  the agent (`agents.json`, the control socket request). For that agent the daemon answers with the first `allow_once` option.
  It never selects `allow_always`: a remembered allow is the agent's to keep, and the daemon could not revoke it later. If an
  allow-set agent's request offers no `allow_once`, the default applies.
- **Logged.** The daemon logs every request and its answer: the tool call's id and kind, and the answer's option kind. It does not
  log the tool call's title or input, which can hold a command line or a secret. ACP's request carries no separate tool-name
  field, so the id and kind are what can be logged safely.
- **Any other agent-initiated method** still gets `-32601`, as before.
- A request that arrives after the daemon has sent `session/cancel` is answered `cancelled`, as ACP requires.

**Cancel.** When a run ends from outside the turn, the daemon sends `session/cancel` for the session, waits up to 2 s for
the `session/prompt` request to return (stop reason `cancelled`), and only then does what ADR 0005 does: SIGTERM to the
agent's process group, SIGKILL 2 s later. An agent that ends its turn on the cancel and exits is never signalled. The three
triggers and the outcome each reports to the pod:

| Trigger | Outcome | Why |
| --- | --- | --- |
| `run.stop` for the run | `cancelled` | ADR 0002: a stopped run ends `cancelled`. |
| The run's deadline (its `expiresAt`, the run token's expiry) | `failed` | ADR 0005: anything but a finished turn is `failed`. |
| The daemon stops | `failed` | ADR 0005. |

`run.stop` reaches the daemon on the pod socket; the pod re-sends it on each heartbeat until the daemon reports the run ended
(`docs/protocol.md`), so a stop that arrives before the run is claimed here, or after it ended, is ignored. A cancel sent before
`session/new` has answered has no session to cancel: the run's context ends the call at once, as it did.

2026-11-08: after `session/cancel`, however the session ends (stop reason `cancelled`, a JSON-RPC error on `session/prompt`, or
the agent exiting without answering) the run ends with the cause that cancelled it, so a stop is `cancelled` in all three and a
deadline or daemon stop is `failed` in all three. Before it, an error or an exit read as a failure and hid the stop.

2026-11-08: an option with a missing, non-string or empty `optionId` is never selected. If no usable option of the wanted kind
is left, the answer is the `cancelled` outcome.

## ACP shapes used

Read from the protocol's published documentation (agentclientprotocol.com, pages "Tool Calls" and "Prompt Turn"), 2026-11-08:

- Request, agent to client: `{"method":"session/request_permission","params":{"sessionId","toolCall":{"toolCallId", ...},"options":[{"optionId","name","kind"}]}}`.
  `kind` is one of `allow_once`, `allow_always`, `reject_once`, `reject_always`.
- Response: `{"result":{"outcome":{"outcome":"selected","optionId":"<id>"}}}`, or `{"result":{"outcome":{"outcome":"cancelled"}}}`.
  "If the current prompt turn gets cancelled, the Client MUST respond with the cancelled outcome."
- Cancel, client to agent, a notification (no `id`): `{"method":"session/cancel","params":{"sessionId"}}`. The client must answer
  pending permission requests `cancelled`; the agent then "MUST respond to the original `session/prompt` request with the
  `cancelled` stop reason".

## Consequences

- The deadline is the claim response's `expiresAt` read against the daemon's clock. A daemon whose clock runs far from the
  server's cuts runs early or late. The pod revokes the run token at expiry either way, so a late cut leaves an agent without
  access, not with more.
- The fake agent (`e2e/testdata/fakeagent`) gains `permission=<kinds>`, `cancel` and `deaf`, so the daemon tests and the e2e
  cases drive both exchanges without a real harness.
- Allowing is per agent, not per tool. An operator who wants finer rulings needs the open item below.

## Open items

- Relaying a permission request to the pod as an ask a person rules on (F12's rulings), once a harness needs a human in the
  loop. Until then a harness that asks is either denied or, with the allow setting, allowed for that one call.
- The 2 s cancel wait is fixed, like ADR 0005's SIGTERM grace. A harness that needs longer to wind a turn down gets signalled.
