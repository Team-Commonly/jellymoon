# 0004 · The open-core boundary

Status: Proposed. Nothing here is built: `ee/` is empty and no server package exists yet. It becomes Accepted when the
maintainers ratify it, which settles none of the open items at the end.

## Decision

Core is Apache-2.0. Everything under `ee/` is under the Commonly Enterprise licence (`ee/LICENSE`). The licence is adapted
from PostHog's `ee/` licence and has not had legal review. The root `LICENSE` stays Apache-2.0 and says nothing about
`ee/`; `ee/README.md` and the README say that `ee/` is under its own commercial licence, not Apache-2.0.

**Why this licence.** `ee/` is the paid part. A licence that allows free production self-hosting, such as ELv2, would leave
only the key check protecting it. BSL gives the code away after its conversion date. The `ee/` licence keeps the source
readable, lets anyone copy and modify it for development and testing without a subscription, and requires a paid
subscription with Commonly for production.

**Self-hosting stays free.** Self-hosting core stays free under Apache-2.0, with no limit, and this ADR changes nothing about that. Which hosted plans include which `ee/` features is a pricing decision, kept outside this repository.

### What may live in `ee/`
Features that only make sense for a paid deployment and that core can run without: for example single sign-on, audit export,
retention controls and organisation-wide policy. Never in `ee/`: the wire protocol, the pod core, claims and leases, the
daemon, the CLI, MCP and anything an agent needs to join a pod. Those are the open product (ADR 0003 keeps the pod core pure).
`ee/` takes no outside contributions (`CONTRIBUTING.md`).

### Core never imports `ee/`
Enforced by the `ee-stays-in-ee` rule in `.dependency-cruiser.cjs`: no module outside `ee/` may depend on a module in `ee/`.
`ee/` may import core. The rule is pinned by the tracked `boundary-fixture/` (a module outside `ee/` importing one inside it), which
`scripts/check-boundary-rule.mjs` runs in `make entropy`: it must report exactly one violation, named `ee-stays-in-ee`.
Deleting, downgrading or exempting the rule fails preflight. When `ee/` gains TypeScript and `worker/` arrives, add both `ee` and `worker` to the `check:boundaries` roots. Go code has no such rule yet.

### The entitlement interface
Core asks one question and never learns where the answer comes from:

```ts
export type FeatureKey = `ee.${string}`;
export interface Entitlements {
  isFeatureEnabled(feature: FeatureKey, ctx: { podId: string }): boolean;
}
export const noEntitlements: Entitlements = { isFeatureEnabled: () => false };
```

`isFeatureEnabled` is synchronous and reads state that is already loaded; any key check, refresh or network call happens in
the `ee/` implementation, before the request, never inside it. A feature key starts with `ee.` so a core feature can never be
gated by accident.

**Where it lives.** Not in `podcore/`, which is pure and has no I/O (ADR 0003). It belongs in the server package
(`worker/src/entitlements.ts`), which does not exist yet, so this unit adds **no code**. The worker's factory takes an
`Entitlements` argument and defaults it to `noEntitlements`. A paid deployment builds its own entrypoint, outside core, that
imports core and `ee/` and passes the `ee/` implementation in. That is how core stays free of `ee/` imports.

### With no `ee/` implementation
The feature is off and nothing fails: `noEntitlements` answers `false` for every key, a gated code path takes its default branch,
and no error, log line or placeholder reaches a user. The worker's tests must run with `noEntitlements`.

## Open items
- The licence has had no legal review.
- The names and number of gated features are not chosen.
- How an `ee/` implementation verifies a subscription (a key, an online check, or both) is not decided.
- Whether the interface takes more context than `podId` (a person or an organisation) waits for the first gated feature.
