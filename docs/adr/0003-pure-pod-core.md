# 0003 · A pure pod core

Status: Proposed (units P09, P10, P11). It adds a module (`podcore/`), so it needs this note. The server shape is
still open; this decision holds under every shape being weighed.

## Decision

The rules of ADR 0002 (claims, leases, recourse) live in `podcore/`, a pure TypeScript module.

- `apply(state, cmd, now)` returns `{ state, out }`. `nextDeadline(state)` returns the next time the core has
  work, or `null`. `state` is plain JSON data; `cmd` is a tagged union; `out` is an ordered list of members
  events, `run.wake` and `run.stop` for a computer, and replies (ok, or a schema error code).
- Time is an argument. The core has no clock, no timer, no randomness and no I/O. Every command first settles
  whatever is due at `now` (lapsed leases, run tokens past 2 h), so a late tick and an early request agree.
- Authorization is decided from state (pod owners, each run's agent owner, the claim's holder). Actors arrive
  already authenticated and stamped; the core never trusts a role in the input.
- Event and error shapes come from `gen/`. Nothing in `podcore/` is hand-copied from the schema.
- Purity is enforced twice: the `podcore-is-pure` dependency rule (imports only `podcore/` and `gen/`) and
  `podcore/test/purity.test.ts`, which reads `podcore/src` recursively and bans `Date`, timers, `Math.random`,
  `crypto`, `fetch`, `process`, `performance`, `globalThis`, `await`, `Intl`, `eval`, `Function(`, `Atomics`,
  `Bun.`, `Temporal`, `queueMicrotask`, `WeakRef`, `FinalizationRegistry`, `.then(` and bracket access on any
  banned global. `isoOf` throws on a non-integer `now`; adapters pass integer milliseconds.

## Why

- Tests are deterministic and need no platform: advancing the clock is passing a bigger `now`. A restart is a
  JSON round-trip of the state, and a property test proves it changes nothing.
- One adapter file per server shape. Changing where the pod runs changes one file, not the rules.
- Expiry belongs to our code, not to a platform alarm's behaviour, so a test can advance it.

## Time: one timeline

- Expiry is AT `leaseUntil` (`<=`): a command arriving at that instant finds the claim already gone.
- `settle` runs before every command and walks ONE timeline. It collects every due deadline (a lease at
  `leaseUntil`, a run cap at `startedAt` + 2 h) with deadline `<=` now and handles them in ascending deadline order.
  On a tie the run cap wins, so the claim is released with `run_ended`. A claim that ended earlier on the timeline
  is not ended again.
- Every effect of a passed deadline is stamped with that deadline: the envelope `ts`, `task.updatedAt`. It is never
  stamped with the settling command's `now`. A late tick and an on-time tick emit identical output, and `ts` never
  decreases because every command settles everything due before it runs.
- A claim is lost as `released` (never restorable) only when a PERSON released it. A run releasing its own claim may
  claim it again, and an operation by that run on the now-free task behaves as on any free task. Lost claims are
  stored by run, then task.

## Who may call

- A caller that is a run which has ended (any outcome) gets `unauthorized` on EVERY command, before any other check
  (ADR 0002, F13). A live run calling `run_stop` gets `forbidden`.
- A caller that is a user must be a member of the pod, or gets `forbidden` on every command, before `not_found`, so a
  stranger cannot probe ids. Before `pod_create`, every command gets `not_found`.
- A refused command never renews anything. A successful run operation renews all of the run's claims.
- A run whose claim a person released gets `forbidden` from `task_release` and `task_update` on that task too.
- `run_end` carries the reporting `computerId`; a computer that does not host the run gets `forbidden`. `run_end` after
  `run_stop` keeps the outcome `cancelled`.

## What an adapter must do

1. Persist the new state atomically with emitting `out`, for every command, heartbeats included, before replying or
   emitting: never one without the other. The one exception is below.
2. Own the single timer: call `apply(state, { type: 'tick' }, now)` at or after `nextDeadline(state)`. A second
   timer firing is harmless; a repeated tick at the same `now` emits nothing.
3. Own the seq log and replay: keep the members events and re-deliver them from `seq`. The core keeps only state; it
   keeps no messages. (`run.wake` and `run.stop` are re-sent by the core, below.)
4. Authenticate: map a token to its pod and check the token is valid, stamp the caller, handle invites (the adapter calls
   `member_add` after one is accepted), mint every id (tasks, messages, runs, run tokens, and wakes for `change`), and
   map reply codes to HTTP statuses. The core derives its own wake ids for assignments and mentions from a counter.
5. After restoring state on a (re)start, apply `boot {staleMs}` once, with `now`. `staleMs` is an integer from 0 to
   `LEASE_MS` (anything else throws): the most that the adapter's saved heartbeat renewals can lag the truth. An
   adapter that persists every command passes 0. `boot` runs WITHOUT the leading settle: it first sets every live
   claim's `leaseUntil = max(leaseUntil, min(leaseUntil + staleMs, now + LEASE_MS))`, then settles at `now`, deadline
   stamped, and emits only what that settle emits.
   Why: if settle ran first, a claim whose saved `leaseUntil` lags its true deadline would be expired early at boot.
   With this rule, expiry after a restart is never early, and lateness is bounded by the larger of the outage and
   `staleMs`. That lets an adapter coalesce saves of heartbeat-only renewals; every other command is still
   persisted before its reply or emission.

## Commands

`pod_create`, `member_add`, `agent_register`, `task_create`, `task_update`, `message_post`, `change` (kind `ruling`
only), `claim`, `op`, `heartbeat`, `task_release`, `run_end`, `run_stop`, `wake_ack`, `run_claim`, `tick`, `boot`.
`task_create` and `task_update` write the `task_assigned` wake, and `message_post` the `mention` wakes, through the
same writer `change` uses. An `@name` token is a mention when `name` equals a registered agent's name exactly and the
token is not inside a backtick code span; a run's message never wakes its own agent. Text with no mention writes no wake.

## Wakes and run start (P10)

All in `podcore/src/wakes.ts`, over the same state. The adapter extracts the addressing facts; the core decides.

- `agent_register` tells the core who owns an agent and which computer hosts it. `change` carries
  `{kind: ruling, agentId, refs, wakeId}`; an unknown kind writes nothing. Assignments and mentions are derived
  by `task_create`, `task_update` and `message_post`; an ambient message writes no wake.
- One pending wake per agent, structurally (`wakes` is keyed by agent). A `change` for an agent with a pending
  wake merges its refs into it, sends nothing and keeps the wakeId. `WakeRefs` holds one id per kind, so on a
  clash the earlier ref stays; the agent reads the pod for the rest. The reason is the first change's.
  Delivery is one to-computer `run.wake` to the hosting computer, with no members event and no seq.
- Re-send: on a `heartbeat` from the hosting computer, an un-acked wake whose last send is 60 000 ms old or
  more (>=) is sent again, and the clock restarts. Another computer's heartbeat sends nothing. `wake_ack` from a
  computer that does not host the agent is ignored. An acked wake is never re-sent.
- `run.stop` is re-sent on every heartbeat from the hosting computer, with no wait, until `run_end` clears it.
- `run_claim {wakeId, computerId, runId}` creates the Run (expiry is the 2 h cap from now), consumes the wake
  and replies `{runId, expiresAt, reason, refs}` for the adapter to build the brief. Unknown or consumed wake:
  `not_found`. A computer that does not host the agent: `forbidden`, and the wake stays.
- A `change` for an unregistered agent is dropped, not held. A held wake has no computer to go to, and holding
  it would need a delivery rule on register; the agent reads the pod when it first starts, so nothing is lost.
- Re-send has no deadline: nothing is due until a heartbeat arrives, so `nextDeadline` is unchanged.

## Not here yet

- Decisions (rulings) are not modelled; `change` carries them until they are.
- `task_update` takes `status` and `assignee`; the schema's `TaskUpdateInput` has no `assignee` yet.

## Open unknowns (carried from ADR 0002)

- 60 s is a guess. Measure expiries followed by a restore within 5 minutes.
- The 2 h run cap ends long tasks mid-work; whether the churn matters is unmeasured. This core ends a capped
  run as `failed`; the schema does not say which outcome a capped run gets.
- Heartbeat renewal cannot tell a hung harness from a busy one.
- Nothing is inherited from v3's 90 s lease.
