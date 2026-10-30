# The e2e contract suite

`e2e/` is one Go suite that judges any server implementing [protocol.md](../protocol.md) plus the test-route
contract below. It is black box: HTTP and the pod WebSocket only, real `jmu_`, `jmc_` and `jmr_` tokens, no
import of server code. It writes no server. `daemon/internal/protocol` is Go-internal and cannot be imported
from `e2e/`, so the suite keeps its own small wire structs.

Run it: `JM_E2E_SERVER='<shell command>' make e2e`. With `JM_E2E_SERVER` unset every test skips, so
`make test` stays green. `make e2e` ends with `E2E_MS=<n> STEPS=<k>/<k>`.

## How the suite starts the server

The suite owns the process. It runs `JM_E2E_SERVER` with `sh -c`, from the repo root, with env `PORT`,
`ENV=local` and `JM_DATA_DIR=<fresh dir>`, in its own process group, and waits for `GET /healthz` = 200.
Each test gets a fresh server and data dir. A restart is `kill -9` of the process group plus the same
command again, on the same port and the same data dir. The server must bind with `SO_REUSEADDR` (Bun, Node
and wrangler do by default).

## Test routes (ENV=local only)

| Route | Behaviour |
|---|---|
| `GET /healthz` | 200 |
| `POST /__test/session` `{name}` | 200 `{token, user}`: a `jmu_` user token and the user's Actor. No device flow. |
| `POST /__test/clock` `{now}` | Sets the server clock, in unix ms. 200. |
| `GET /__test/clock` | 200 `{now}` |

Clock rules, which every server under test must meet:

- **Before the first `/__test/clock` call the server uses its real clock. After it, `now` is the set value and
  moves only by another call.** The suite sets 2040-01-01T00:00:00Z first thing after every boot, before it
  mints a session, so a restarted server on its real clock never finds a lease due.
- **When the clock moves, the server applies any due `tick` at once, independent of its real timer or
  alarm.** Expiry must fire with zero other traffic. The response may come before events are delivered;
  the suite waits up to 5 s.
- **The real timer or alarm path is exercised only in real-clock mode** (`JM_E2E_REAL_CLOCK=1`).
  Fake-clock mode proves the rules; real-clock mode proves the alarm.
- Everything else goes through the real routes in `schema/pod-api.json` and `schema/operations.json`.

## Persistence rule

- Every command's new state is persisted **before the reply or the emission**, with one exception: renewals
  caused only by heartbeats, which a server may coalesce.
- A server that coalesces declares how far its saved heartbeat renewals can lag as the core's `boot {staleMs}`
  (0 to 60 000) and tells the suite by env `JM_E2E_STALE_MS` (default 0). Saved state is never more than
  `staleMs` behind on heartbeat renewals.
- The server applies the core's `boot` command right after restoring state, before serving.
- A coalesced save of anything else fails `TestKillMidLease` as an early expiry.

## Assumptions the protocol does not state

The suite asserts these; a server that disagrees should change the protocol, not the suite.

- The server sends `hello` right after the WebSocket upgrade; the client then sends `subscribe`.
- The WebSocket token travels in `Authorization: Bearer ...` on the upgrade request.
- The pod sends each Envelope as one unfragmented text frame (the suite's client does not reassemble fragments).
- Any run of an agent in the pod may claim any task, assigned or not.
- `already_claimed` names the holder in `ref` or `message`: the holder's run id, agent id or display name.
- `task.create` is a socket event (no HTTP route creates tasks); its `task.created` answers it.
- Every sequenced frame is replayed, and sequence numbers have no gaps. Frames with no `seq` (`run.wake`,
  `run.stop`, `hello`) are not part of the log.
- Releasing as the claiming agent's owner means the user whose computer registered the agent.

## The renewal ledger

No server reports a deadline and events carry no renewal times, so the suite keeps one ledger
(`world.stamp`, `world.expected` in `e2e/world_test.go`). It records every run operation that succeeded and
every heartbeat the suite sent for a live run, stamped with the clock the suite set (fake mode) or the time
it sent the request (real-clock mode). The expected expiry is the last renewal plus 60 000 ms. Every
early-expiry and lateness check reads it, so every server is measured identically.

## The tests

| Test | What it proves |
|---|---|
| `TestF02` | F02: the harness starts the server with a persisted state dir, `/healthz` answers 200 within 10 s, a failing step names itself and prints the last 20 events, and the run ends with the closing line. `TestStepPrinter` checks the printer alone, with a fake failing step. |
| `TestF07` | F07: ambient messages wake nobody; three assignments make one wake; a pending wake is re-sent on the heartbeat at exactly 60 s and not 1 ms before, acked or not (only a run claim consumes it); only the hosting computer is woken or re-woken. |
| `TestF10` | F10 steps 1 to 7, plus restore after expiry, no restore after a release, and `forbidden` for a member who may not release. |
| `TestExpiryZeroTraffic` | After the last renewal the suite sends nothing. Still claimed at deadline - 1; at the deadline members get `task.updated` (no claim) and one `claim.released{expired}`. |
| `TestRenewRace` | Concurrent claims, renewals and heartbeats from two runs: one holder, `leaseUntil` only forward and equal to the ledger. |
| `TestKillMidLease` | Two claims, `kill -9`, restart. See below. |
| `TestReplayAcrossRestart` | `subscribe sinceSeq=N` after a restart replays N+1..head in order, once each, equal to what a live socket saw; new events continue after head. |
| `TestOneReleasePerCause` | For `expired`, `released`, `run_ended` and `stopped`, each member gets exactly one `claim.released` with that cause, with `by` and `reason` as specified. |
| `TestF14Budget` | The whole suite run (fake clock) passes in under 60 s. It judges the run so far, so it is last; it fails if a suite test failed and skips on a partial run. |

`TestKillMidLease` claims Y (no renewal after the claim; its deadline falls inside the outage) and X
(renewed by a heartbeat; its deadline falls after the restart), then crashes. For each it asserts: not
released before its recorded deadline; released no later than `max(restart, deadline) + JM_E2E_STALE_MS + 6 s`
once the clock passes it; one `claim.released` over live delivery and over a replay from seq 0.

## Output lines

- `E2E_MS=<n> STEPS=<k>/<k>`: the closing line. `n` is the suite wall clock after `go test` finishes its tests; `k/k` counts the named steps that passed over the named steps that ran (F02 names its steps; a failing step prints its name and the last 20 events the suite saw before it).
- `LATENESS_MS <kind> <ms>`: observed expiry minus the ledger's deadline, real-clock mode only. `kind` is
  `steady` (zero-traffic expiry), `after_restart_ahead` (deadline after the restart) or
  `after_restart_inside` (deadline inside the outage).
- `E2E_INVALID <reason>`: the run is invalid, not a server failure. A restart that takes over 10 s from
  `kill -9` to `/healthz` does this. In real-clock mode the suite deliberately holds the server down so that
  Y's deadline passes during the outage; that hold is not counted.

## Real-clock mode

`JM_E2E_REAL_CLOCK=1` never calls `/__test/clock`. Only `TestExpiryZeroTraffic` and `TestKillMidLease` run,
in parallel, and take about 100 s. Thresholds asserted: lateness in [-2, 6000] ms for `steady`; for the
restart kinds, not before the deadline (-2 ms rounding) and not after `max(restart, deadline) + stale + 6 s`.

## Self-check without a server

`e2e/testdata/fakeserver` answers `/healthz` and nothing else. It is not a server under test and counts toward none.

```
go build -o "$S/fakeserver" ./e2e/testdata/fakeserver
JM_E2E_SERVER="$S/fakeserver" go test ./e2e -count=1 -v
```

Every test must fail in under 10 s with a message naming the missing route, and `TestF14Budget` must fail.
