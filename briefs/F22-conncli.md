# F22 · CONNCLI · A pod owner connects, grants and names approvers from the CLI

No template for briefs exists in this repo yet; this one uses the fields the batch rows share: governing ADRs, decided rules,
test, non-goals.

GOVERNING ADRs: docs/adr/0009-connector-layer.md:10-11 "A pod owner connects an upstream service and grants agents access to it through the pod. The pod holds the credential; an agent holds only its `jmr_` run token" | 0009:52-53 "Only a pod owner connects (`POST /v1/pods/:pod/connections {vendor, url, token}`) or grants (`POST /v1/pods/:pod/grants {connectionId, agentId, access}`)." | 0009:53 "The url must pass the egress check below." | 0009:53-56 "A connect lists the upstream's operations and compares them with the pin: an empty list, a pinned operation missing upstream, or one whose argument names or required arguments changed refuses the connect with `connector_drift`, naming the operation. An upstream that fails the listing gets `upstream_failed` with its own error, scrubbed." | 0009:56-57 "A grant names an agent registered in the pod and an access, `read` or `write-with-confirm`; there is no plain `write`." | 0009:59-60 "OAuth (with dynamic client registration, as Linear's remote MCP offers) is out of this slice: the owner pastes a token." | 0009:39-40 "The plaintext token is decrypted in memory for one upstream call and is never logged, returned or put in an error." | 0009:104-108 "A pod owner rules on a connector write's ask, the same role that connects and grants, and what the card's `needs.to.role: owner` says. So does a member on that connection's approver set: a pod owner names it with `POST /v1/pods/:pod/approvers {connectionId, userIds}`, members only, each once, replacing the last set (an empty list clears it; stored in the `approvers` table)." | 0009:126 "Every call that names an existing connection writes a `connector_calls` row, stamped by the server from its own records" | 0009:129-130 "No route writes one. Members and runs read them." | docs/adr/0008-github-sign-in.md:42 "The token never reaches argv, stdout, stderr or a log." (said of `jellymoon login`; the CLI keeps the same rule for the connector credential and for the user token) | 0008:91-94 "Past it the token is no caller on every route and on the socket upgrade, as a Bearer and as the cookie: `unauthorized`." (a lapsed user token reaches the CLI as `unauthorized`, so the CLI says to run `jellymoon login`) | docs/adr/0007-agent-permission-and-cancel.md: no clause is relied on or touched. It governs the daemon's answers to an agent's ACP permission requests; the connector commands start no agent and answer no permission request.

ADR check: no rule in the row contradicts an Accepted ADR. Two wording gaps, neither a conflict. The row says "pod admin"; ADR 0009
says "pod owner", and the server checks the pod owner, so the CLI says owner. The row says `connector list` "shows pinned
operations by grant", but `GET /v1/pods/:pod/connectors` answers a person with each connection's pinned operations and no
grants (access appears only when an agent asks). With no server change allowed, the list shows what each access level would let
an agent call (a `read` grant: the reads; `write-with-confirm`: all, each write parked for approval), derived from the pin's
classes the route returns. A route that lists the stored grants is a separate row.

## Decided

1. `jellymoon connector connect <vendor> --pod P --url U` reads the credential from `JELLYMOON_CONNECTOR_TOKEN` (removed from
   the process environment as soon as it is read) or, with no variable set, from a hidden prompt on the terminal. There is no
   `--token` flag and a stray argument is refused without echoing it. With neither source it fails before any request. The
   credential is sent once, in the body of the connect request, and appears nowhere else: not argv, not stdout or stderr
   (errors are scrubbed of it too), not a log, not a file in the config dir. The command prints the connection id, the vendor
   and the pinned operations.
2. `jellymoon connector grant --pod P --connection C --agent A --access read|write-with-confirm`. Any other access, `write`
   included, is refused before a request is made, with the reason "there is no plain write; use write-with-confirm".
3. `jellymoon connector approvers --pod P --connection C --users u1,u2` replaces the set. `--users ""` sends an empty list and
   clears it. Omitting `--users` is an error, so a typo cannot clear the set.
4. `jellymoon connector list --pod P` shows each connection and its pinned operations by access level, never a credential.
   `jellymoon connector receipts --pod P [--decision D] [--limit N]` prints the server's receipts (id, outcome, operation,
   agent, decision, approver, upstream status, reason).
5. A server refusal is printed as the server sent it, `<code>: <message>`, and the exit code is 1. So a drift refusal shows
   `connector_drift` and names the operation, and a non-owner's connect, grant or approvers shows `forbidden`.
6. Auth is the person's `jmu_` token, from `JELLYMOON_USER_TOKEN` or the file `jellymoon login` writes, as `import commonly`
   does. The server is `JELLYMOON_SERVER`. The CLI adds no state file and writes nothing to the config dir.

Test: `go test ./e2e -run TestF22 (against the stub Linear F19 uses; asserts argv, env, stdout, stderr, the config dir and the recorded requests hold no credential)`,
run with `make e2e-c`. Unit tests in `daemon/internal/cli` cover the argument rules and the environment removal. The Worker has
no connector routes, so `TestF22` joins `TestF19` in `E2E_A_SKIP`, with its reason in `docs/development/e2e-contract.md`.

## Non-goals

- No server change and no schema change: only the routes ADR 0009 describes.
- No web screen for connections.
- No OAuth: pasting a token is the slice (ADR 0009 above). OAuth is a blocked row.
- No rename of the CLI binary or the npm scope, which are under a trademark hold, and no publish.
- The hidden prompt is not under e2e test: it needs a pseudo-terminal, so the e2e runs the environment path and proves the
  no-terminal path fails closed. The prompt is the same code `import commonly` uses for its token.
