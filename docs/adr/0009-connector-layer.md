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
(`POST /v1/pods/:pod/grants {connectionId, agentId, access}`). The url must pass the egress check below. A connect lists the upstream's operations and
compares them with the pin: an empty list, a pinned operation missing upstream, or one whose argument names or required
arguments changed refuses the connect with `connector_drift`, naming the operation. An upstream that fails the listing gets
`upstream_failed` with its own error, scrubbed. A grant names an agent registered in the pod and an access, `read` or
`write-with-confirm`; there is no plain `write`.

OAuth (with dynamic client registration, as Linear's remote MCP offers) is out of this slice: the owner pastes a token. The
vault stores whatever credential the connect hands it, so an OAuth flow later changes the connect route, not the vault.

## Egress

The credential goes only where `server/src/egress.ts` allows, and the check runs before every upstream request, not only at
connect: connect's listing, each read, and each released write all check the stored url again. A connection that passed at
connect and whose name later resolves inside is refused (`upstream_failed`, "was not called", a `failed` receipt) and nothing
is sent.

- **The url.** https, no credentials in it. The host is lowercased and its trailing dots are stripped, so `localhost.` is
  `localhost`. `localhost` and `*.localhost` are refused by name. WHATWG URL parsing has already turned odd IPv4 spellings
  (`2130706433`, `0x7f.1`, `127.1`, `0177.0.0.1`) into dotted decimal and `[::127.0.0.1]` into `[::7f00:1]`; the tests
  assert it.
- **One classifier, `internalAddress`.** It parses the address rather than matching prefixes, and anything that does not parse
  counts as internal. IPv4: 0/8, 10/8, 100.64/10, 127/8, 169.254/16, 172.16/12, 192.0.0/24, 192.168/16, 198.18/15, and 224/4
  and up (so 240/4 and 255.255.255.255), plus the documentation ranges 192.0.2/24, 198.51.100/24 and 203.0.113/24 and the
  old 6to4 relay 192.88.99/24. IPv6: ::/96 (`::`, `::1` and every v4-compatible `::a.b.c.d`), fc00::/7, fe80::/10, ff00::/8,
  64:ff9b:1::/48 (local-use NAT64) and 2001::/32 (Teredo) are refused. v4-mapped ::ffff:0:0/96, v4-translated
  ::ffff:0:0:0/96, NAT64 64:ff9b::/96 and 6to4 2002::/16 are judged by the IPv4 address they embed.
- **Resolve, then pin.** A name is resolved with `lookup(host, { all: true, verbatim: true })`, given 2 s (well inside the
  request's 10 s); a lookup that has not answered by then counts as unresolved. It is refused if it does not resolve, resolves
  to nothing, or if any address it resolves to is internal. The request is then pinned to one checked address: the first IPv4
  if there is one, else the first address. There is no fallback to the other checked addresses if that one cannot be reached.
  The request is sent to that IP, with the name as the `Host` header and as the TLS server name (Bun's `fetch` `tls.serverName`),
  so the certificate is still verified against the name and a second lookup cannot move the request. Verified on Bun 1.4.4
  against a local TLS server: a matching name connects, while a wrong name, no name and an untrusted certificate are each refused.
  A literal public address is sent as is; its certificate must name that address. If `HTTPS_PROXY` is set in the server's
  environment, Bun's `fetch` sends the request through the proxy, and the pinned IP (not the name) is the proxy's CONNECT
  target, so the proxy cannot resolve the name again; TLS is still end to end, against the name.
- **ENV=local.** Only in local mode, and only for a literal loopback address (127/8 or ::1), is http allowed and the
  classifier skipped: that is the e2e stub on 127.0.0.1. Every other host gets the full check in local mode too. Local mode is
  the process environment `ENV=local`, read once at startup (`server/src/config.ts`). No request can set it, and a production
  server cannot run with it: startup refuses `ENV=local` alongside `DATABASE_URL` unless the server is confined to a
  throwaway schema (`JM_PG_SCHEMA`, which the e2e suite sets per server).

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
- **Names that resolve publicly but mean something internal.** The egress check classifies addresses, so a name that a
  split-horizon resolver answers with a public address it routes internally is not caught. An egress proxy or a network policy
  around the server is the layer for that.
- **One check per call.** A call is up to three requests (initialize, the notification, the
  call), and each is pinned to the address checked once for the whole call. Pinning closes the rebinding window inside a call;
  a name that moves between calls is checked again on the next one.
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
