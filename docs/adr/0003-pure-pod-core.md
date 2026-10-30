# 0003 · A pure pod core

Status: Proposed (units P09, P10). It adds a module (`podcore/`), so it needs this note. The server shape is
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
  `podcore/test/purity.test.ts` (no `Date`, timers, `Math.random`, `crypto`, `fetch`, `process`,
  `performance`, `globalThis` or `await` in `podcore/src`).

## Why

- Tests are deterministic and need no platform: advancing the clock is passing a bigger `now`. A restart is a
  JSON round-trip of the state, and a property test proves it changes nothing.
- One adapter file per server shape. Changing where the pod runs changes one file, not the rules.
- Expiry belongs to our code, not to a platform alarm's behaviour, so a test can advance it.

## What an adapter must do

1. Persist the new state atomically with emitting `out`: never one without the other.
2. Own the single timer: call `apply(state, { type: 'tick' }, now)` at or after `nextDeadline(state)`. A second
   timer firing is harmless; a repeated tick at the same `now` emits nothing.
3. Re-deliver members events from `seq`. (`run.wake` and `run.stop` are re-sent by the core, below.)
4. Stamp the caller, seed pods and tasks into state, mint every id and token (wakes, runs, run tokens), and map
   reply codes to HTTP statuses.

## Wakes and run start (P10)

All in `podcore/src/wakes.ts`, over the same state. The adapter extracts the addressing facts; the core decides.

- `agent_register` tells the core who owns an agent and which computer hosts it. `change` carries
  `{kind: task_assigned | mention | ruling, agentId, refs, wakeId}`; an ambient message never reaches the core,
  and an unknown kind writes nothing.
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

- Creating pods and tasks: the adapter writes those rows into state until commands exist for them (F06).

## Open unknowns (carried from ADR 0002)

- 60 s is a guess. Measure expiries followed by a restore within 5 minutes.
- The 2 h run cap ends long tasks mid-work; whether the churn matters is unmeasured. This core ends a capped
  run as `failed`; the schema does not say which outcome a capped run gets.
- Heartbeat renewal cannot tell a hung harness from a busy one.
- Nothing is inherited from v3's 90 s lease.
