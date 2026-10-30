# v4 connector layer: options paper

Research only, read 2026-10-29. Licenses were read from each repo's LICENSE file via the GitHub API or raw file (URLs in the appendix). Stars and last-push dates are from the same pass. "Unverified" means I could not confirm it from a primary source.

## 1. Recommendation

**The finding that decides it:** no open-source project I found combines a credential vault, agent-invisible tokens and approval of the exact call arguments in a form we could run on Workers. A search for this exact combination turned up only early or partial repos: [ZoeyMCP](https://github.com/RyanLeeUPC/ZoeyMCP) is design-phase with nothing built, and [mcp-human-approval-gate](https://github.com/Daniel221287/mcp-human-approval-gate) has no vault. The pieces that are commodity are the MCP client, the OAuth client flow, and the vendors' own remote MCP servers. Cloudflare's Agents SDK already provides the first two on our stack. The gate (approve, release one write, stamp a receipt) is the part nobody ships, and it is the product.

### ADOPT (permissive licence, fits Workers + DO + TypeScript)
- **Linear's official remote MCP** as the upstream, via `https://mcp.linear.app/mcp/readonly`, which "only ever exposes read tools" ([Linear docs](https://linear.app/docs/mcp)). The server is vendor-hosted, so we run nothing.
- **Cloudflare Agents SDK MCP client** (`cloudflare/agents`, MIT, 5.9k stars, pushed 2026-10-28). `addMcpServer()` runs the OAuth redirect, stores tokens in the Durable Object's SQLite, discovers tools, and blocks private-IP URLs ([MCP client API](https://developers.cloudflare.com/agents/model-context-protocol/mcp-client-api/)).
- **MCP TypeScript SDK** for our own `/mcp` server (already assumed).
- **Cloudflare Workflows `waitForApproval`** as the durable pause behind a decision card. It can wait months without the agent running ([human-in-the-loop guide](https://developers.cloudflare.com/agents/guides/human-in-the-loop/)).
- **Deferred, not now:** [`vercel/chat`](https://github.com/vercel/chat) (MIT, 2.4k stars, pushed 2026-10-29) for channel bridges. It is TypeScript with adapters for Slack, Teams, Google Chat, Discord, Telegram, GitHub, Linear, WhatsApp, X and more. Whether it runs on Workers is unverified (its README example uses Next.js and Redis).
- **Deferred, not now:** [Infisical Agent Vault](https://github.com/Infisical/agent-vault) for the E2B hosted-agent case, where credentials should be kept outside the VM behind a proxy. It is a Go MITM credential proxy, MIT with an `ee/` carve-out, in research preview as of April 2026 ([launch coverage](https://infisical.com/blog/agent-vault-the-open-source-credential-proxy-and-vault-for-agents)). It protects non-MCP traffic (git, LLM keys) that our broker never sees.

### CONTRIBUTE UPSTREAM
- **`cloudflare/agents`:** the docs show no per-tool filter or wrapper hook on the MCP client; `getAITools(filter?)` filters by server only (the docs page states no hook, and I could not confirm there is none in the source). We need a `callTool` interception point so our gate sits inside the client rather than around it. We also need an encrypted storage adapter for OAuth tokens; the docs allow a custom adapter (`DurableObjectOAuthClientProvider`). Send both as PRs rather than forking, per the upstream-first rule. Risk: the SDK is 0.x and `McpAgent` was deprecated in v0.20 (earlier research notes, Cloudflare).
- **`vercel/chat`** adapters, when we reach channels.
- **Agent Vault,** if we adopt it (an approval hook for matched hosts).

### BUILD (about 1 to 2 engineer-weeks, mostly ports from v3)
v3 already contains each piece, in about 3,000 lines across `toolBrokerService.ts`, `hostedMcpCredentialService.ts`, `hostedMcpDcrRegistry.ts`, `connectorSecrets.ts` and `routes/grants.ts`:
1. **Envelope-encrypted credential store** in the DO. v3's `connectorSecrets.ts` (188 lines) is AES-256-GCM with a key ring; port it with a Workers secret as the master key.
2. **Tool catalogue pin and drift check.** v3 pins a real `tools/list` per vendor (Linear: 38 tools, re-pinned 2026-10-14 after the vendor added parameters) and refuses calls on drift.
3. **The gate:** classify each call from the pin, park writes as a decision card, release exactly one call on approval, and write the server-stamped receipt.
4. **Grant model** (who may use which connection in which pod).

### Operations, not MCP
An owner ruling after this paper was first written: the protocol is plain HTTP operations, and MCP
is one adapter over them, so the connector layer must not be MCP-shaped either. An upstream is
modelled as **a set of operations**, whatever it speaks:

| Upstream speaks | How its operations are listed | How one is called |
|---|---|---|
| Remote MCP (Linear) | `tools/list` | `tools/call` |
| OpenAPI | the upstream's OpenAPI spec `operationId`s | HTTP, per the upstream's OpenAPI spec |
| Plain REST, no spec (or a spec too large to pin whole) | a hand-written list in the pin file | HTTP |

The vault, the catalogue pin, the drift check, the write gate and the receipt all key on
`(connection, operationName, argsHash)` and never on a protocol. Each protocol is one adapter file
with two functions (`list`, `call`). Linear's remote MCP stays the slice upstream because it is the
fastest path to a working read, not because the layer depends on MCP.

**The same gate, over plain REST (GitHub):** a pod connects GitHub with a fine-grained token held in
the DO. The pin file lists the operations the pod allows, e.g. `issues.list` as
`GET /repos/{owner}/{repo}/issues` (read) and `issues.create` as `POST /repos/{owner}/{repo}/issues`
(write). The agent calls our `connector_call` operation (or its MCP form) with `issues.create` and
the arguments. The gate classifies it from the pin as a write, parks `{runId, operation, argsHash}`,
and posts a decision card with the exact arguments. On approval the DO makes that one HTTP call
with the token and writes the receipt. Nothing in that path knows MCP exists.

**Agents SDK lock-in check:** the SDK is used only inside the MCP adapter file (its OAuth client and
`tools/list`/`tools/call`). The vault, pin, gate and receipts are ours. Swapping or dropping the
SDK touches that one file, and a REST upstream never loads it.

## 2. The Linear READ path under this choice

1. A pod admin clicks Connect Linear. The pod's DO (one connection object per pod) calls `addMcpServer("linear", "https://mcp.linear.app/mcp/readonly")`. The vendor runs OAuth 2.1 with dynamic client registration ([Linear docs](https://linear.app/docs/mcp)); the Agents SDK handles the redirect and callback.
2. The access and refresh tokens land in DO SQLite, encrypted by our envelope layer. They never leave the DO.
3. The agent holds only `jmr_`. It calls our `POST /mcp` with `tools/list`. Our server answers from the pinned catalogue filtered by the pod's grant, never from the live upstream list.
4. On `tools/call`, the Worker derives the actor from the `jmr_` run token (`docs/protocol.md`) and asks the DO to run the call. The DO attaches the token and calls Linear. The result returns to the agent, and a receipt row is written.
5. Because the endpoint is `/mcp/readonly`, even a bug in our gate cannot write; the vendor refuses it. This matches v3, which pinned a `read`-scope token.

**How a WRITE goes through a decision card (designed, not in the slice):** a write needs a second connection, made at read-write scope on a separate consent, because Linear's consent has no workspace picker and the scope is chosen at consent time (observed on v3). An agent's `tools/call` for a write tool is parked as `{runId, tool, argsHash}` and a decision card is created with the exact arguments. On a human's approve, the DO executes that one call with the write token, marks the parked call consumed, and the receipt links card, approver and result. Reject or expiry releases nothing. The v3 broker already had `write-with-confirm` (`backend/routes/grants.ts` line 339 refuses plain `write` on hosted grants).

## 3. Per-candidate answers (serious ones only)

| Candidate | Agents avoid raw tokens? | A WRITE through a decision card? | Exit cost |
|---|---|---|---|
| **Our build on Agents SDK** | Token stays in the DO; agent has `jmr_` only | Park, card, release once (above) | Low: ciphertext rows plus the pin files, in our code |
| **HumanLayer** | Not a vault | See section 4: not adoptable as an approvals SDK | n/a |
| **Activepieces** | Connections encrypted; "no API to read stored connections back" ([MCP page](https://www.activepieces.com/mcp/mcp)) | No approval feature mentioned on that page | Medium: a Node server plus its Postgres and Redis; pieces are MIT packages |
| **Nango (ELv2)** | Tokens stay in Nango's store | Not provided; free self-host is auth and proxy only ([docs search](https://nango.dev/docs/guides/platform/free-self-hosting.md)) | Medium: tokens sit in its database |
| **Composio / Pipedream Connect / Arcade** | Vendor holds tokens (Arcade's engine "stays in Arcade's cloud") | Vendor-side policy, not our card | High: the credential store is theirs |
| **ACI.dev** (Apache) | Multi-tenant auth and secrets inside ACI | Not shown | Medium: a Python/FastAPI service we would run |
| **Agent Vault** | Strong: agent sees dummy values; proxy injects | A matched-host hook would be needed | Low: it is a proxy in front of traffic |

## 4. Verdicts on the named options

- **HumanLayer: not a fit, and not what we remembered.** The repo (Apache-2.0, 12.7k stars, pushed 2026-10-29) is now a coding-agent steering product: the README lists a desktop app, a daemon that runs Claude Code and Codex, and a Linear CLI ([README](https://github.com/humanlayer/humanlayer)). That makes it closer to a competitor than a component. The old approvals SDK (`@humanlayer/sdk` 0.7.9, Apache-2.0) was last modified on npm 2026-01-05 and the repo root no longer holds SDK code. Treat it as design inspiration for approval flows only.
- **Nango: the licence is ELv2, not open source.** The LICENSE file and README badge both say Elastic, and the README still calls it "open source." ELv2 bars providing it "as a hosted or managed service" exposing a substantial set of its features, and bars circumventing its license-key functionality. We would also be running a Node server with its own database. Do not copy its provider catalogue into an Apache-2.0 repo.
- **Composio:** the repo is MIT, but it is the SDK; the backend is not in it. Self-hosting is described as an enterprise offer in a support-forum reply ([thread](https://support.composio.dev/m/1340459156334837791); unverified). One aggregator claims Elastic-2.0; the repo's own LICENSE says MIT.
- **Pipedream:** the whole repo is under the "Pipedream Source Available License," unchanged since 2023-01-03, which excludes "any commercial use" for an Excluded Purpose. Connect is a hosted service ("encrypted at rest" on Pipedream's side, [docs](https://pipedream.com/docs/connect)). Neither is usable as a base.
- **Arcade:** `arcade-mcp` is MIT (1.1k stars), but the engine that holds tokens stays in Arcade's cloud ([docs](https://docs.arcade.dev/en/guides/deployment-hosting/on-prem)).
- **Klavis:** Apache-2.0, 5.8k stars, last push 2026-06-01 (five months idle). Open MCP servers and a Strata router; the hosted platform is the product.
- **Airbyte: licence changed.** The 2025-07-23 commit "migrate licenses to elv2" moved the repo to ELv2. The README still shows both an MIT and an ELv2 badge; which parts are which is unverified. Singer is a spec with no LICENSE file in its guide repo; taps are licensed one by one. Meltano is MIT (2.7k stars). Data sync is out of the slice; Python taps do not run on Workers. If we need sync later, use Workflows plus vendor webhooks.
- **modelcontextprotocol/servers:** MIT-to-Apache-2.0 transition in progress ("undergoing a licensing transition"); its own README says the servers are reference implementations, "not production-ready." Use as examples.
- **Vendor remote servers:** Linear (verified above, read-only URL). Sentry: OAuth only, scoped by URL path to an org or project ([mcp.sentry.dev](https://mcp.sentry.dev/)); its open repo `getsentry/sentry-mcp` is **FSL-1.1-Apache-2.0**, not OSI. Notion: hosted, OAuth; the page says nothing on bearer tokens or read-only scopes (unverified). GitHub: `github/github-mcp-server` MIT, 34k stars, remote at `api.githubcopilot.com/mcp/`; PAT authentication works with MCP hosts, per its README. Atlassian: in the v3 walk, `tools/list` verification failed and our code dropped the vendor's real error (memory above).
- **Docker MCP gateway:** MIT, 1.6k stars; built around Docker Desktop secrets and container-isolated local servers. A local-daemon tool, not a Worker component.
- **Cloudflare `workers-oauth-provider`:** MIT. It is the OAuth *provider* side (for our own `/mcp` front door, useful if people authenticate CLIs with OAuth). It is not the client for third-party upstreams. The Agents SDK client is.
- **Channels:** Matterbridge (`42wim`, Apache-2.0, 7.6k stars) was last pushed 2024-12-12; it relays text between chat networks and has no per-agent identity. Our own v3 channel code is: `backend/integrations/providers/` (Discord, GroupMe, Instagram, Slack, Telegram, X) and OpenClaw extensions (`_external/clawdbot/extensions/`: Slack, Telegram, Discord, Matrix, WhatsApp, Signal, Teams and about 25 more; MIT, forked from OpenClaw). Good as protocol references. They are Node and Mongo shaped, so they are not a drop-in for DOs.
- **Credentials:** Infisical main repo is MIT with an `ee/` carve-out (30k stars). It is a Node + Postgres server aimed at operator secrets, not per-user OAuth tokens. OpenBao is MPL-2.0 (8.8k stars), a heavy Go server for our size. 1Password Connect's repo has no LICENSE file (GitHub returns 404 for it); its server licensing is unverified. A DO-encrypted vault is a few hundred lines and keeps tokens beside the code that uses them.
- **Not on your list (credible):** (1) **Infisical Agent Vault**, above. (2) **agentgateway** (Apache-2.0, 5.8k stars, pushed 2026-10-28): MCP tool federation, OAuth, RBAC with a CEL policy engine. Rust; a candidate if we later self-host a gateway off Workers. (3) **Obot** (MIT, 1.1k stars) and **IBM ContextForge** (Apache-2.0, 4.7k stars) as full MCP gateways. **Watch the licence traps:** Archestra is AGPL-3.0 with an enterprise carve-out, and Metorial is Functional Source License.

## 5. Risks

1. **Agents SDK churn.** It is 0.x (npm 0.31.0) and `McpAgent` was deprecated within about a year. Pin the version and keep it inside the MCP adapter file: the gate, vault and pin are ours and protocol-neutral (see "Operations, not MCP"), so a client swap touches one file.
2. **Tool drift and silent zero tools.** v3 bit on both: a grant with `audience: []` surfaced zero tools (observed on v3), and Linear added parameters that made calls fail until re-pinned. Reject empty audiences at grant time, assert a non-empty `tools/list` after connect, and pass the vendor's real error through.
3. **Token refresh and OAuth edge cases in a DO.** Concurrent refresh, DCR registration tied to the callback host (v3 kept a per-host registry), and the missing workspace picker on Linear's consent.
4. **Prompt injection in results.** Linear issue text flows to the agent. The read path is safe from writes only because of `/mcp/readonly`; the write path needs the gate to key on args, not on the agent's explanation.
5. **Proxy does not make a token harmless.** A third-party review of Agent Vault notes that an agent can still do whatever the token permits ([coverage](https://runtimewire.com/article/infisical-launches-agent-proxy-ai-agent-api-keys)). Hence the separate read-only token.
6. **Licence drift.** Eight projects above changed or carry non-OSI terms (Nango, Airbyte, Pipedream, Sentry-mcp, Metorial, Archestra, Composio's disputed claims, the MCP repo's transition). Re-read before any adoption.

---

## Appendix: comparison table

Columns: option | licence (verified 2026-10-29) | stars / last push | self-host | replaces | fit | notes.

| Option | Licence | Stars / last push | Self-host | Replaces | Fit (Workers+DO+TS, cmr_) | Notes |
|---|---|---|---|---|---|---|
| Cloudflare Agents SDK [1] | MIT | 5.9k / 2026-10-28 | n/a (library) | MCP client, OAuth client, token store | Native; tokens in DO SQLite | Adopt. 0.x churn. No per-tool hook documented |
| workers-oauth-provider [2] | MIT | 1.9k / 2026-10-27 | n/a (library) | OAuth provider for our `/mcp` | Native | Provider side only |
| MCP TypeScript SDK [3] | MIT to Apache transition (unverified) | 13.7k (unverified) | n/a | MCP server plumbing | Native | Already assumed |
| Linear remote MCP [4] | Vendor-hosted; server source not found (unverified) | n/a | None needed | Linear tool catalogue | Fits; OAuth 2.1 + DCR, `/mcp/readonly` | The slice upstream |
| GitHub MCP server [5] | MIT | 34.1k / 2026-10-29 | Local or remote | GitHub catalogue | Remote works over HTTP | PAT works with hosts |
| Sentry MCP [6] | FSL-1.1-Apache-2.0 (not OSI) | 949 / 2026-10-29 | Remote hosted | Sentry catalogue | OAuth only | Use hosted, do not vendor |
| Notion MCP [7] | MIT (`notion-mcp-server`) | 4.7k / 2026-10-28 | Remote hosted | Notion catalogue | OAuth; headless unverified | |
| modelcontextprotocol/servers [8] | MIT to Apache-2.0 transition | 91.7k / 2026-10-29 | Local | Examples only | Reference only | "Not production-ready" |
| HumanLayer [9] | Apache-2.0 | 12.7k / 2026-10-29 | Docker compose, guides "in progress" | Nothing (no approvals SDK in repo) | Poor: AWS, Electron, Bun | npm SDK 0.7.9 last modified 2026-01-05 |
| Activepieces [10] | MIT outside `ee/` dirs; `ee/` commercial | 25.3k / 2026-10-29 | Docker; MCP on self-host per vendor | Long-tail tool catalogue | Node server + DB, not Workers | Pieces MIT; MCP also vendor-claimed |
| ACI.dev [11] | Apache-2.0 | 4.9k / 2026-10-23 | Docker compose (Python) | Vault + OAuth + catalogue | Separate Python service | Heavier than needed |
| Klavis [12] | Apache-2.0 | 5.8k / 2026-06-01 | Docker per server; `strata-mcp` | MCP server catalogue | Separate processes | Idle 5 months |
| Nango [13] | **ELv2** | 13.0k / 2026-10-29 | Free: auth + proxy only; paid for syncs/tools | OAuth connect + vault | Node + Postgres; hosting limits | Avoid |
| Composio [14] | MIT (SDK repo) | 30.8k / 2026-10-29 | Enterprise only (forum, unverified) | Catalogue + auth (hosted) | Vendor holds tokens | High exit cost |
| Arcade (`arcade-mcp`) [15] | MIT | 1.1k / 2026-10-29 | Workers only; engine in their cloud | Tool SDK | Engine hosted | Pattern: tokens injected, never shown to the model |
| Pipedream [16] | **Source Available** (no commercial use) | 11.8k / 2026-10-28 | Not self-hostable (unverified for Connect) | Components, Connect | Not permitted for us | Avoid |
| Airbyte [17] | **ELv2** (commit 2025-07-23); README also shows MIT badge | 22.3k / 2026-10-29 | Docker/K8s | Data sync | Python/Java, off Workers | Out of slice |
| Singer / Meltano [18] | Singer guide repo: no LICENSE; Meltano MIT | 1.4k (2025-08-08) / 2.7k (2026-10-29) | Python | Data sync | Off Workers | Out of slice |
| Docker MCP gateway [19] | MIT | 1.6k / 2026-10-26 | Docker Desktop / CE | Local MCP runner | Not a Worker | Local daemon idea only |
| agentgateway [20] | Apache-2.0 | 5.8k / 2026-10-28 | Single Rust binary | MCP proxy/gateway | Off Workers | New option |
| Obot [21] | MIT | 1.1k / 2026-10-28 | Self-host | MCP gateway + catalogue | Off Workers | New option |
| IBM ContextForge [22] | Apache-2.0 | 4.7k / 2026-10-28 | Self-host (Python) | Gateway + registry | Off Workers | New option |
| Vercel chat SDK [23] | MIT | 2.4k / 2026-10-29 | n/a (library) | Channel bridges | TS; Workers unverified | Exists; adapters listed in section 1 |
| Matterbridge [24] | Apache-2.0 | 7.6k / 2024-12-12 | Go binary | Text relay | Off Workers; stale | Avoid |
| v3 channel code + OpenClaw ext. [25] | OpenClaw fork MIT | local repo | n/a | Channel protocol reference | Node/Mongo shaped | Port, not drop-in |
| Infisical [26] | MIT outside `ee/`; `ee/` commercial | 30.1k / 2026-10-29 | Node + Postgres | Operator secrets | Off Workers | Not per-user OAuth |
| Infisical Agent Vault [27] | MIT outside `ee/`; `ee/` commercial | 2.4k / 2026-10-27 (v0.40.2) | Single Go binary | Credential proxy | Daemon/E2B side | Research preview since April 2026 |
| OpenBao [28] | MPL-2.0 | 8.8k / 2026-10-29 | Go server | Vault/KMS | Overkill | |
| 1Password Connect [29] | No LICENSE file in repo | 232 / 2026-06-06 | Connect server | Vault | Unverified | |
| DO-encrypted vault (build) | Apache-2.0 (ours) | n/a | Part of the Worker | Vault | Native | Port of v3 `connectorSecrets.ts` |

### Sources

[1] https://github.com/cloudflare/agents/blob/main/LICENSE · https://developers.cloudflare.com/agents/model-context-protocol/mcp-client-api/ · https://developers.cloudflare.com/agents/guides/human-in-the-loop/ · https://www.npmjs.com/package/agents (0.31.0 via `npm view`, 2026-10-29)
[2] https://github.com/cloudflare/workers-oauth-provider/blob/main/LICENSE.txt · https://github.com/cloudflare/workers-oauth-provider
[3] https://github.com/modelcontextprotocol/typescript-sdk (license and star count not re-read for this paper)
[4] https://linear.app/docs/mcp
[5] https://github.com/github/github-mcp-server/blob/main/LICENSE · https://github.com/github/github-mcp-server (README)
[6] https://mcp.sentry.dev/ · https://github.com/getsentry/sentry-mcp (LICENSE: FSL-1.1-Apache-2.0)
[7] https://developers.notion.com/docs/mcp · https://github.com/makenotion/notion-mcp-server
[8] https://github.com/modelcontextprotocol/servers (LICENSE and README)
[9] https://github.com/humanlayer/humanlayer (LICENSE, README, root listing) · https://registry.npmjs.org/@humanlayer/sdk
[10] https://github.com/activepieces/activepieces/blob/main/LICENSE · https://www.activepieces.com/mcp/mcp
[11] https://github.com/aipotheosis-labs/aci
[12] https://github.com/Klavis-AI/klavis
[13] https://github.com/NangoHQ/nango/blob/master/LICENSE · https://github.com/NangoHQ/nango (README) · https://nango.dev/docs/guides/platform/free-self-hosting.md
[14] https://github.com/ComposioHQ/composio/blob/next/LICENSE · https://support.composio.dev/m/1340459156334837791
[15] https://github.com/ArcadeAI/arcade-mcp · https://docs.arcade.dev/en/guides/deployment-hosting/on-prem
[16] https://github.com/PipedreamHQ/pipedream/blob/master/LICENSE · https://pipedream.com/docs/connect
[17] https://github.com/airbytehq/airbyte (LICENSE, README; commit history of LICENSE)
[18] https://github.com/singer-io/getting-started · https://github.com/meltano/meltano
[19] https://github.com/docker/mcp-gateway
[20] https://github.com/agentgateway/agentgateway
[21] https://github.com/obot-platform/obot
[22] https://github.com/IBM/mcp-context-forge
[23] https://github.com/vercel/chat (README, LICENSE, `packages/`) · https://www.npmjs.com/package/chat (4.43.0)
[24] https://github.com/42wim/matterbridge
[25] v3 source, Team-Commonly/commonly: `backend/integrations/providers/`, `_external/clawdbot/extensions/`, `_external/clawdbot/LICENSE` (MIT)
[26] https://github.com/Infisical/infisical/blob/main/LICENSE
[27] https://github.com/Infisical/agent-vault · https://infisical.com/blog/agent-vault-the-open-source-credential-proxy-and-vault-for-agents
[28] https://github.com/openbao/openbao/blob/main/LICENSE
[29] https://github.com/1Password/connect
Other: https://github.com/RyanLeeUPC/ZoeyMCP · https://github.com/Daniel221287/mcp-human-approval-gate · https://github.com/archestra-ai/archestra (LICENSE router: AGPL-3.0 + enterprise) · https://github.com/metorial/metorial (Functional Source License 1.1) · https://runtimewire.com/article/infisical-launches-agent-proxy-ai-agent-api-keys · v3 internals: Team-Commonly/commonly (`backend/`)
