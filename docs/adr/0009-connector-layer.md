# 0009 · The connector layer: a vault, a pinned catalogue, a write gate and receipts, in the pod-state server

Status: Accepted for the F19 slice, 2026-11-08. It builds the BUILD path of `docs/design/connector-options.md` (ratified the
same day) in the pod-state server (ADR 0006). Numbered 0009 because 0007 and 0008 are held by open changes. Known unknowns are
listed at the end.

## Decision

A pod owner connects an upstream service and grants agents access to it through the pod. The pod holds the credential; an
agent holds only its `jmr_` run token and reaches the upstream through one operation, `connector_call`. Reads go through at
once. A write is parked as an ask showing its exact arguments, and one ruling of approve releases exactly one upstream call.
Every call leaves a receipt the server stamps.

It lives in `server/` only. The Cloudflare Worker is not extended: ADR 0006 made the server the adapter Jellymoon runs, and the
options paper's Durable Object pieces are superseded by it (the durable pause behind a card is a Postgres row plus the pod
core's decision record). `TestF19` is on the Worker's skip list (`E2E_A_SKIP`, `docs/development/e2e-contract.md`).

## Operations, not MCP

The layer keys on `(connection, operation, argsHash)` and never on a protocol. A vendor's operations are a pin file in the
repo (`server/connectors/<vendor>.json`): each operation's name, its class (read or write) and its argument names. The pin also
names the protocol, and each protocol is one adapter file with two functions, `list` and `call`. The first is MCP over
Streamable HTTP (`server/src/upstream-mcp.ts`), because Linear's remote MCP is the slice upstream; nothing outside that file
knows MCP. An OpenAPI or plain REST upstream is a new adapter file and one line in `server/src/upstream.ts`.

What an agent sees is never the live upstream list: `connector_list` (and the card, the receipts) come from the pin filtered by
the agent's grant. The upstream may list more than the pin; those operations do not exist for the pod.

## The vault

A credential is stored only as an AES-256-GCM envelope (`server/src/vault.ts`, WebCrypto, no dependency): a fresh 32-byte data
key per row encrypts the token, and the master key wraps the data key. Both layers bind `<pod>/<connection>` as additional data,
so a ciphertext moved to another row does not open. The master key is `JM_VAULT_KEY` (base64 of 32 bytes); retired keys go in
`JM_VAULT_OLD_KEYS` (comma-separated), and each row stores its key's id (the first 16 hex digits of the key's SHA-256), so the
ring rotates without re-encrypting at once.

Without `JM_VAULT_KEY` the server still starts, and a connect or a connector call answers `vault_unconfigured` (501): a server
that has no connectors needs no key. A malformed key refuses the start, and the refusal never echoes it. The plaintext token is
decrypted in memory for one upstream call and is never logged, returned or put in an error. Every upstream answer and error
passes through one scrub (`catalogue.ts`, `scrub`) before it reaches an agent, a person, a receipt or a log. It replaces the
token in raw and JSON-escaped form, then drops whole any string that still holds it once decoded: `\u` escapes,
percent-encoding, or base64 (standard or URL-safe, at any alignment, so `base64("Bearer " + token)` is caught too). An
upstream HTTP error reaches the agent as its status plus the upstream's text, a JSON body re-serialized from its parsed form,
scrubbed and only then cut to 400 characters, so a cut can never leave half a token.

An upstream answer is read no further than 1 MiB (a declared or streamed length over it). A longer one is refused with
`upstream_too_large` and none of it is kept.

## Connect, drift and grants

Only a pod owner connects (`POST /v1/pods/:pod/connections {vendor, url, token}`) or grants
(`POST /v1/pods/:pod/grants {connectionId, agentId, access}`). The url is https, carries no credentials, and names no loopback,
private or link-local host unless `ENV=local` (the e2e stub runs on 127.0.0.1). A connect lists the upstream's operations and
compares them with the pin: an empty list, a pinned operation missing upstream, or one whose argument names or required
arguments changed refuses the connect with `connector_drift`, naming the operation. An upstream that fails the listing gets
`upstream_failed` with its own error, scrubbed. A grant names an agent registered in the pod and an access, `read` or
`write-with-confirm`; there is no plain `write`.

OAuth (with dynamic client registration, as Linear's remote MCP offers) is out of this slice: the owner pastes a token. The
vault stores whatever credential the connect hands it, so an OAuth flow later changes the connect route, not the vault.

## The gate

`connector_call {connectionId | vendor, operation, args}` with a run token: the run's agent's grant and the operation's pinned
class decide. An operation not in the pin is `not_found`; no grant, or a write on a read grant, is `forbidden`. A read calls
the upstream once and returns its result. A write calls nothing: in one transaction the pod posts the agent's own ask (options
`a` approve and `b` reject, the exact arguments in a fenced JSON block in its body, the args hash beside them) and stores the
parked call `{run, connection, operation, args, argsHash, decision}`. The agent gets `{state: parked, decisionId}`.

Only a pod owner rules on a connector write's ask, the same role that connects and grants, and what the card's
`needs.to.role: owner` says. A ruling from any other member is refused `forbidden` before the pod core sees it, so nothing is
claimed and the decision stays open for an owner. (Asks that are not connector writes keep the core's rule: any member rules.)

A ruling on that ask, through the existing `decision.rule` path, claims the parked row in the ruling's own transaction: one
`update ... where outcome = 'parked' and expires_at > now returning`, so the row is consumed exactly once whatever the ruling,
a retry or a race does (the core already refuses a second ruling; the claim is once-only on its own). After the transaction
commits and before members hear of the ruling, a row released by approve makes its one upstream call with the parked
arguments, and the answer is recorded. Reject releases nothing. The asking agent is woken with `decision_ruled`, as for any ask,
and reads the outcome with `connector_receipts {decisionId}`.

A parked write expires after 24 hours. The pod core now takes an `expiresAt` on an ask (adapter-set, like the decision id):
the decision is a deadline like a lease, a tick expires it and tells members with `message.updated`, and a later ruling gets
`gone`. The server's claim also requires the row to be unexpired, so the two agree.

## Receipts

Every call that names an existing connection writes a `connector_calls` row, stamped by the server from its own records: id,
pod, connection, operation, argsHash (SHA-256 of the arguments as JSON with sorted keys), run, agent, outcome (`ok`, `failed`,
`refused`, `parked`, `released`, `rejected`, `expired`), upstream status, timestamps, and for a write its decision, the
approver who released it and the upstream's result. No route writes one. Members and runs read them.

## Still open

- **A per-connection approver set.** Any pod owner may approve any connection's writes; naming approvers per connection is
  a later decision.
- **DNS and other address forms.** The url check reads literals only; a name that resolves to a private address is not
  caught, nor are `localhost.` with a trailing dot, the `::/96` and `64:ff9b::/96` ranges, 198.18.0.0/15, or internal names.
  Refusing those forms, then resolving the name and pinning the address for the call, is the fix.
- **A crash between the claim and the call.** The row is then `released` with no upstream status: never called twice, maybe
  never called. At most once is the chosen side; a `releasing` state that the call's answer settles would make it visible.
- **A read that throws.** A read whose vault open fails (for example a retired key no longer in the ring) answers 500
  `internal` and writes no receipt, because the receipt is saved after the call.
- **A slow upstream holds the pod.** A release runs inside the pod's command queue, so one upstream at its timeouts (three
  10 s requests) delays that pod's next commands by up to about 30 s.
- **The pin is the slice's.** Linear's real catalogue is large; the pin holds `list_issues`, `get_issue` and `create_issue`
  with the argument names the slice uses. Re-pin against the live catalogue before connecting a real workspace.
- **Tool-call count.** A `connector_call` over MCP counts in the run's `toolCalls` like any tool call; one made over plain
  HTTP is an operation, not an MCP call, and is not counted, as for every other operation.
