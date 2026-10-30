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
  `podcore/test/purity.test.ts`, which reads `podcore/src` recursively, comments included, and bans `Date`, timers,
  `crypto`, `fetch`, `process`, `performance`, `globalThis`, `self`, `global`, `window`, `console`, `Reflect`,
  `Intl`, `Atomics`, `Temporal`, `Bun`, `WeakRef`, `FinalizationRegistry`, `queueMicrotask`, `await`, `eval`,
  `require`, `import(`, `Function (` and `.then(`. Bare `Math` is banned except the calls `Math.floor`, `Math.min`
  and `Math.max`, so `Math.random`, `Math['random']` and `const M = Math` all fail. It is a lexical backstop, not a
  proof: it reads text, so a spelling it did not list gets through, and the import rule is the other half.
  `now` must be a non-negative integer of milliseconds.

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
  (ADR 0002, F13). A live run calling `run_stop` gets `forbidden` before any lookup,
  unknown run ids included.
- A caller that is a user must be a member of the pod, or gets `forbidden` on every command, before `not_found`, so a
  stranger cannot probe ids. Before `pod_create`, every command gets `not_found`.
- Renewal follows ADR 0002: its lease renews on any operation that run's token makes. A live run's command renews
  all of that run's claims BEFORE any refusal (`forbidden`, `already_claimed`, `not_found`, `invalid_input`), so
  what a command is answered never changes when a lease ends. An ended run gets `unauthorized` and renews nothing.
  This is also what keeps "expiry after a restart is never early" true below. (An earlier draft said a refused
  command never renews; it contradicted ADR 0002 and broke that line.)
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
   `member_add` after one is accepted), mint every id (tasks, messages, runs, run tokens and wakes), validate `reason`
   length (160) before `apply`, and map reply codes to HTTP statuses. Wake ids are globally unique and minted by the
   adapter like the rest: `task_create`, `task_update`, `message_post` and `change` each carry ONE
   fresh `wakeId`. The k-th new wake a command opens (text order) gets `nthWakeId(wakeId, k)`: k = 0 is the id itself,
   k > 0 adds k to the ULID's 80-bit random part with carry. So the seed's random part must be new CSPRNG output for
   every command, never a monotonic ULID factory's +1: with one, seed+1 is the next command's seed and two wakes share
   an id. Under fresh random bits, a clash needs two seeds within k of each other in the same millisecond. A command that
   coalesces uses no id; a malformed seed or an overflow is `invalid_input`. `POST /v1/runs {wakeId}` names no pod, so the adapter keeps
   a wakeId to pod index.
   Callers pass a non-decreasing `now`; the core does not clamp it.
5. After restoring state on a (re)start, apply `boot {staleMs}` once, with `now`. `staleMs` is an integer from 0 to
   `LEASE_MS` (anything else is `invalid_input`): the most that the adapter's saved heartbeat renewals can lag the truth. An
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
token is outside CommonMark code: a backtick run of length n opens a span that closes at the next run of exactly n and
may cross a newline, a fence of three or more backticks or tildes (up to 3 spaces indented) runs to a matching fence or
the end of the text, and a backslash-escaped backtick is literal. An unclosed single backtick is literal, so text after
it still mentions. A run never wakes its own agent, through any command. Text with no mention writes no wake.
`message_post` sets `summary` to the first non-blank line (split on CR, LF or CRLF), trimmed and cut to 120 code points,
or `(no text)`. `task_update` refuses status `open` with `invalid_input`.

## Wakes and run start (P10)

All in `podcore/src/wakes.ts`, over the same state. The adapter extracts the addressing facts; the core decides.

- `agent_register` tells the core who owns an agent and which computer hosts it. `change` carries
  `{kind: ruling, agentId, refs, wakeId}`; an unknown kind writes nothing. Assignments and mentions are derived
  by `task_create`, `task_update` and `message_post`; an ambient message writes no wake.
- One pending wake per agent, structurally (`wakes` is keyed by agent). A `change` for an agent with a pending
  wake merges its refs into it, sends nothing and keeps the wakeId. `WakeRefs` holds one id per kind, so on a
  clash the earlier ref stays; the agent reads the pod for the rest. The reason is the first change's.
  Delivery is one to-computer `run.wake` to the hosting computer, with no members event and no seq.
- Re-send: on a `heartbeat` from the hosting computer, a pending wake whose last send is 60 000 ms old or
  more (>=) is sent again, and the clock restarts. Only `run_claim` consumes the wake (the exit): `wake_ack` is a
  valid command that changes nothing, so an acked wake is re-sent like any other. Another computer's heartbeat
  sends nothing. `agent_register` that moves an agent to another computer sends its pending wake to the new one at
  once, clock restarted.
- `run.stop` is re-sent on every heartbeat from the hosting computer, with no wait, until `run_end` clears it.
- `run_claim {wakeId, computerId, runId}` creates the Run (expiry is the 2 h cap from now), consumes the wake
  and replies `{runId, expiresAt, reason, refs}` for the adapter to build the brief. Unknown or consumed wake:
  `not_found`. A computer that does not host the agent: `forbidden`, and the wake stays. A `runId` that already
  exists, live or ended: `invalid_input` (the schema has no `conflict` code), and the wake stays. A run's owner is
  captured here, so a later owner change of the agent does not move authority over a run already live.
- A `change` for an unregistered agent is dropped, not held. A held wake has no computer to go to, and holding
  it would need a delivery rule on register; the agent reads the pod when it first starts, so nothing is lost.
- Re-send has no deadline: nothing is due until a heartbeat arrives, so `nextDeadline` is unchanged.

## Throws

`apply` never throws on adapter input: a non-integer or negative `now`, an unknown command type, a bad `staleMs`, a
malformed or exhausted `wakeId` and a non-string message `text`, a `now` past the end of year 9999, and a `run_end` outcome outside the schema enum are all `invalid_input` and change nothing. An id that is `__proto__` or names an `Object.prototype` member (`constructor`, `toString`, ...) in any `*Id` field or `assignee` is `not_found`, refused before any lookup or write; every record lookup also goes through `own()` (`Object.hasOwn`). What can
still throw is an invariant violation, i.e. a bug or a corrupt saved state: `structuredClone` of a non-JSON state,
`isoOf` on a non-integer (unreachable once `now` is checked), and a missing record that state invariants guarantee
(a wake's agent, a claim's run).

## Accepted limits

- Computers can tell an unknown id from a foreign one by the reply code. Accepted: ids are unguessable ULIDs.
- The boot stretch can turn an `expired` release near the cap into `run_ended`, because a stretched lease may now
  meet the 2 h run cap first.
- Size: `podcore/` is capped at 1,750 lines in `entropy.json`, tests included; the raise is noted in ADR 0001. No
  exception was added; it ratchets down from here.
- Indented code blocks are NOT recognised as code, so a mention inside one wakes (deliberate).
- The adapter mints `messageId` and owns the message log, so message id uniqueness is the adapter's; the core keeps no set of them.
- The adapter bounds message body size: `stripSpans` is super-linear on growing backtick runs.
- A run's `task_update` on a task another live run holds is `already_claimed` naming the holder (one claim per task,
  `operations.json`); a person may still update.

## Not here yet

- Decisions (rulings) are not modelled; `change` carries them until they are.
- `task_update` takes `status` and `assignee`; the schema's `TaskUpdateInput` has no `assignee` yet.

## Open unknowns (carried from ADR 0002)

- 60 s is a guess. Measure expiries followed by a restore within 5 minutes.
- The 2 h run cap ends long tasks mid-work; whether the churn matters is unmeasured. This core ends a capped
  run as `failed`; the schema does not say which outcome a capped run gets.
- Heartbeat renewal cannot tell a hung harness from a busy one.
- Nothing is inherited from v3's 90 s lease.
