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
| `POST /__test/evict/:pod` | The pod's object forgets everything it holds in memory and restores from storage, as a woken hibernated object does; its WebSockets stay attached. 200. Needed only by `TestF04`; a server with no such object may make it a no-op. |
| `POST /__test/discard/:pod` `{now?, alarmMs?}` | The runtime throws the pod's object away, not just its fields: in-flight requests end, every WebSocket drops, and the next request or alarm builds a new instance. `now` (unix ms) is stored as the server clock with no tick; `alarmMs` stores an alarm that many real ms ahead. 200 before the object goes. Needed only by `TestEvictMidClaim` and `TestEvictAlarmWake`; a server with no such object cannot run them. |
| `GET /__test/instance/:pod` | 200 `{instance, requests}`: an id minted when the object was built, and how many requests it has served (this one excluded). A discard changes the id. |
| `GET /__test/woke/:pod` | 200 `{instance, requests}` recorded by the object's alarm handler when it last ran (read without waking the object), or `null`. |

Clock rules, which every server under test must meet:

- **Before the first `/__test/clock` call the server uses its real clock. After it, `now` is the set value and
  moves only by another call.** The suite sets 2040-01-01T00:00:00Z first thing after every boot, before it
  mints a session, so a restarted server on its real clock never finds a lease due.
- **A restarted server is a new process and starts on its real clock again** (`kill -9` plus the same command). The suite sets the clock again
  after a restart when a test needs fake time.
- **When the clock moves, the server applies any due `tick` at once, independent of its real timer or
  alarm.** Expiry must fire with zero other traffic. The response may come before events are delivered;
  the suite waits up to 5 s.
- **The real timer or alarm path is exercised only in real-clock mode** (`JM_E2E_REAL_CLOCK=1`).
  Fake-clock mode proves the rules; real-clock mode proves the alarm.
- Everything else goes through the real routes in `schema/pod-api.json` and `schema/operations.json`.

## Persistence rule

The rule is ADR 0003, "What an adapter must do", item 5. A server persists every command that moves a lease, heartbeats
included, and never coalesces saves: a lagging saved lease would expire early after a restart, against ADR 0002's
"`leaseUntil` is the last renewal plus 60 s".

- Every command's new state is persisted **before the reply or the emission**.
- The server applies the core's `boot {staleMs: 0}` right after restoring state, before serving. `staleMs` is 0 and
  nothing else: the core refuses any other value. The suite has no setting for it; `JM_E2E_STALE_MS` set to anything but
  0 stops the run with an error.
- A coalesced or skipped save fails `TestKillMidLease` as an early expiry, and a skipped `boot` fails
  `TestRestartPastDeadline`.

## Input validation

Every route that takes a body checks it against the request definition in `schema/` before anything runs. A body the
schema refuses (a wrong type, a value over a limit such as a 16,385-character `body`, a two-line `summary`, a missing
`kind`, an unknown field, a field the URL already supplies) answers `400 invalid_input` and changes nothing. Nothing is
dropped or truncated: a body the schema allows is stored whole.

A field the schema allows but the pod core has no place for is also `400 invalid_input`, never silently dropped. Today
that is, on `POST /v1/pods/:pod/messages`: `kind` other than `fyi`, `status` other than `done`, `needs`, `options`,
`context`, `taskId`, `links` and `caveat`; on `PATCH /v1/pods/:pod/tasks/:task`: `note`; on `POST /v1/runs/:run/end`:
`harnessVersion`. A run's post needs its `summary` (the core stores it as the headline).

## Invites

`POST /v1/pods/:pod/invites` returns the invite's `code` once. The server keeps only its SHA-256 (as it does tokens), looks an
accept up by that hash, and builds `url` from its configured public base URL (`PUBLIC_URL` on the Cloudflare adapter, the
request's own origin when unset), never from a fixed host.

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
| `TestF02` | F02: the server's own command line (read from the process table) carries `--persist-to <the run's data dir>/state` and that dir holds state, `/healthz` answers 200 within 10 s, and a failing step names itself and prints the last 20 events. The closing line is judged by `make e2e-a`, not by a test. `TestStepPrinter` checks the printer alone, with a fake failing step. |
| `TestF03` | F03: a user seeded through `POST /__test/session` (the contract's seed route) creates a pod, and the first frame on its pod WebSocket is `hello` with `headSeq` 0 and that user as the one owner. |
| `TestF04` | F04: two members subscribe and both get the same `message.created` seq; one disconnects, three posts happen, and `subscribe{sinceSeq}` replays exactly those three in order; after `POST /__test/evict/:pod` a socket that stayed connected keeps receiving with no gap and a reconnect replays exactly the missed events; the same again after a full kill -9 restart. |
| `TestF05` | F05: an invite is accepted and the new member's `hello` lists them; the same single-use invite returns 410 the second time; an invite accepted after its `ttlSeconds` (the suite moves the clock past it) returns 410; a non-member creating an invite gets 403. After each refusal the owner's `hello` still lists exactly the members it had. |
| `TestF06` | F06: a member sends `task.create` with a registered agent as assignee, and every subscriber gets `task.created` with status `open` and that assignee, at one seq. |
| `TestF07` | F07: ambient messages wake nobody; three assignments make one wake; a pending wake is re-sent on the heartbeat at exactly 60 s and not 1 ms before, acked or not (only a run claim consumes it); only the hosting computer is woken or re-woken. |
| `TestF10` | F10 steps 1 to 7, plus restore after expiry, no restore after a release, and `forbidden` for a member who may not release. |
| `TestExpiryZeroTraffic` | After the last renewal the suite sends nothing. Still claimed at deadline - 1; at the deadline members get `task.updated` (no claim) and one `claim.released{expired}`. |
| `TestRenewRace` | Concurrent claims, renewals and heartbeats from two runs: one holder, `leaseUntil` only forward and equal to the ledger. |
| `TestKillMidLease` | Two claims, `kill -9`, restart. See below. |
| `TestReplayAcrossRestart` | `subscribe sinceSeq=N` after a restart replays N+1..head in order, once each, equal to what a live socket saw; new events continue after head. |
| `TestOneReleasePerCause` | For `expired`, `released`, `run_ended` and `stopped`, each member gets exactly one `claim.released` with that cause, with `by` and `reason` as specified. |
| `TestInputValidation` | Each write route refuses what the schema refuses with `400 invalid_input` (a numeric or 16,385-character `body`, a two-line `summary`, a missing `kind`, a note on a task update, and the other routes' bodies), stores a 16,384-character body whole, and lets a run post with its `summary`. |
| `TestRestartPastDeadline` | A claim made on a fake clock two minutes behind real time, then `kill -9` and a restart with no clock call: the restarted server is on its real clock, so the deadline is already past, and the first frames after the restart carry `task.updated` (no claim) and one `claim.released{expired}`. It needs the server to settle when it restores state. |
| `TestRunTokenOtherPod` | A run token on another pod's routes (read, list, feed, post) gets `403 forbidden`; on its own pod it works. |
| `TestEvictMidClaim` | The runtime discards the pod's object while a run holds a claim and the server process stays up: the instance id changes, the sockets drop, and with the clock stored one second short of the deadline the new object has released nothing and still holds the old `leaseUntil`. A renewal after the eviction moves `leaseUntil` to that clock plus 60 s, a second run's claim is refused with `already_claimed` naming the holder, the old deadline passes without a release, and the new one gives exactly one `claim.released{expired}`. |
| `TestEvictAlarmWake` | The object is discarded with a claim live, the clock stored at the deadline and an alarm stored a few real ms ahead, then nothing is sent to it. The runtime builds a new instance and calls its alarm: the alarm handler's record names a new instance that had served no request, `alarm_fired` moved, and members get exactly one `claim.released{expired}`, none before the deadline and none after the clock moves on. |
| `TestF14Budget` | The whole suite run (fake clock) passes in under 60 s. It judges the run so far, so it is last; it fails if a suite test failed and skips on a partial run. |

`TestKillMidLease` claims Y (no renewal after the claim; its deadline falls inside the outage) and X
(renewed by a heartbeat; its deadline falls after the restart), then crashes. For each it asserts: not
released before its recorded deadline; released no later than `max(restart, deadline) + 6 s`
once the clock passes it; one `claim.released` over live delivery and over a replay from seq 0.

## Output lines

- `E2E_MS=<n> STEPS=<k>/<n>`: the closing line, printed once after every test has ended. `n` is the suite wall clock after `go test`
  finishes its tests. The steps are every named step of the tests that name them (`TestF02`) plus one step for each other test that
  ran; a skipped test counts nothing. `k` is the steps that passed, so a red run prints `k < n`. A failing named step prints its
  name and the last 20 events the suite saw before it. A test cannot see the run's last line, so `make e2e-a` judges it: the
  last line before `go test`'s own `ok`/`FAIL` must be this line, with `k = n` on a green run and `k < n` on a red one.
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
