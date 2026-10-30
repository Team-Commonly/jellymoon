# 0003 · A pure pod core

Status: Proposed (unit P09). It adds a module (`podcore/`), so it needs this note. The server shape is
still open; this decision holds under every shape being weighed.

## Decision

The rules of ADR 0002 (claims, leases, recourse) live in `podcore/`, a pure TypeScript module.

- `apply(state, cmd, now)` returns `{ state, out }`. `nextDeadline(state)` returns the next time the core has
  work, or `null`. `state` is plain JSON data; `cmd` is a tagged union; `out` is an ordered list of members
  events, `run.stop` for a computer, and replies (ok, or a schema error code).
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
3. Re-deliver output that was not acknowledged (members events from `seq`; `run.stop` until the daemon
   reports the run ended).
4. Stamp the caller, seed pods, runs and tasks into state, and map reply codes to HTTP statuses.

## Not here yet

- Wakes (addressed-only, coalescing, resend after 60 s) are the next unit. They will be another command family
  over the same state; the `Cmd` union and `PodState` have room for them.
- Creating pods, tasks and runs: the adapter writes those rows into state until commands exist for them.

## Open unknowns (carried from ADR 0002)

- 60 s is a guess. Measure expiries followed by a restore within 5 minutes.
- The 2 h run cap ends long tasks mid-work; whether the churn matters is unmeasured. This core ends a capped
  run as `failed`; the schema does not say which outcome a capped run gets.
- Heartbeat renewal cannot tell a hung harness from a busy one.
- Nothing is inherited from v3's 90 s lease.
