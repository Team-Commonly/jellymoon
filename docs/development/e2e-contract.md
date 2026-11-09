# The e2e contract suite

`e2e/` is one Go suite that judges any server implementing [protocol.md](../protocol.md) plus the test-route
contract below. It is black box: HTTP and the pod WebSocket only, real `jmu_`, `jmc_` and `jmr_` tokens, no
import of server code. It writes no server. `daemon/internal/protocol` is Go-internal and cannot be imported
from `e2e/`, so the suite keeps its own small wire structs.

Run it against the pod-state server (ADR 0006) with `make e2e` (the same as `make e2e-c`), against the Cloudflare adapter
with `make e2e-a`, or against any other server with `JM_E2E_SERVER='<shell command>' make e2e-any`. With `JM_E2E_SERVER` unset
every test skips, so `make test` stays green. `make e2e`, `make e2e-c` and `make e2e-a` end with `E2E_MS=<n> STEPS=<k>/<k>`.
`make e2e-deployed` (CI's `e2e-deployed` job) runs the same suite with the server's release bundle as the server under test, after
`make deployed-boot` has started that bundle as its own process and checked it by URL (ADR 0006, "Deployed form"). In the hosted
shape that run ends with the read-only checks, which also run alone against any running instance with
`make deployed-boot DEPLOYED_URL=<base>`: they only read, send no credential, and start, stop and sign in nothing, so they are
safe against an instance with real users (ADR 0010, "CI deploy"; the Makefile comment above `deployed-check` lists each check).

## How the suite starts the server

The suite owns the process. It runs `JM_E2E_SERVER` with `sh -c`, from the repo root, with env `PORT`,
`ENV=local` and `JM_DATA_DIR=<fresh dir>`, in its own process group, and waits for `GET /healthz` = 200.
Each test gets a fresh server and data dir, on its own ports, so the tests run in parallel (below). A restart is `kill -9` of the process group plus the same
command again, on the same port and the same data dir. The server must bind with `SO_REUSEADDR` (Bun, Node
and wrangler do by default).

Every server also gets the suite's stub GitHub (`e2e/f15_stub_test.go`, ADR 0008) in its environment: `GITHUB_URL` (the
stub's base; the server reads GitHub's OAuth web flow at `<base>/login/oauth/*` and `GET /user` at `<base>/api/v3/user`),
`GITHUB_CLIENT_ID` and `GITHUB_CLIENT_SECRET`. The suite sets them in its own process before any test runs, so every server
inherits them; only `TestF15`, `TestBSESS`, `TestAUTHH` and `TestDENY` sign anyone in. `TestBSESS`, `TestAUTHH` and `TestDENY` play the stub's authorize hop themselves, with their own identities, so `TestF15` steering the stub's next login cannot race them. The suite never calls the real GitHub. `worker/dev.sh` passes the three on
to wrangler as `--var`s when they are set.

## Test routes (ENV=local only)

| Route | Behaviour |
|---|---|
| `GET /healthz` | 200 |
| `POST /__test/session` `{name}` | 200 `{token, user}`: a `jmu_` user token and the user's Actor. No device flow. |
| `POST /__test/clock` `{now}` | Sets the server clock, in unix ms. 200. |
| `GET /__test/clock` | 200 `{now}` |
| `POST /__test/evict/:pod` | The pod's object forgets everything it holds in memory and restores from storage, as a woken hibernated object does; its WebSockets stay attached. 200. Needed only by `TestF04`; a server with no such object may make it a no-op. |
| `POST /__test/discard/:pod` `{now?, alarmMs?, alarmNow?}` | The runtime throws the pod's object away, not just its fields: in-flight requests end, every WebSocket drops, and the next request or alarm builds a new instance. `now` (unix ms) is stored as the server clock with no tick. `alarmMs` moves the alarm the object itself stored to fire that many real ms ahead (it plants no new one; 409 when the object has none), and `alarmNow` is the clock once that alarm runs. 200 `{discarded, alarm}` before the object goes, `alarm` being the object's stored alarm time before the move. Needed only by `TestEvictMidClaim`, `TestEvictAlarmWake` and `TestEvictPastDeadline`; a server with no such object cannot run them. |
| `GET /__test/instance/:pod` | 200 `{instance, requests}`: an id minted when the object was built, and how many requests it has served (this one excluded). A discard changes the id. |
| `GET /__test/stored-alarm/:pod` | 200 `{at}`: the alarm time the object itself has stored (`ctx.storage.getAlarm()`), unix ms or `null`. Under a fixed clock the object stores its own deadline, a far-future time that never fires by itself. Not counted as a request. |
| `GET /__test/woke/:pod` | 200 `{instance, requests, was}` recorded by the object's alarm handler when it last ran (read without waking the object), or `null`; `was` is the stored alarm time a discard moved, which is the alarm that ran. |

A deployed build (no `ENV=local`) opens these routes only behind `Authorization: Bearer <TEST_BEARER>`, has no `/__test/clock`, and
a build with no bearer has no `/__test` handler at all. The pod-state server serves `session`, `clock` and `evict`; its `evict`
drops the pod's in-memory state and restores it from Postgres, with the sockets attached, which is the whole of what `TestF04`
asks. It has no object to discard, so it does not serve `discard`, `instance`, `stored-alarm` or `woke`.

Clock rules, which every server under test must meet:

- **Before the first `/__test/clock` call the server uses its real clock. After it, `now` is the set value and
  moves only by another call.** The suite sets 2040-01-01T00:00:00Z first thing after every boot, before it
  mints a session, so a restarted server on its real clock never finds a lease due.
- **A restarted server is a new process and starts on its real clock again** (`kill -9` plus the same command). The suite sets the clock again
  after a restart when a test needs fake time.
- **When the clock moves through `/__test/clock`, the server applies any due `tick` at once, independent of its real timer or
  alarm.** Expiry must fire with zero other traffic. The response may come before events are delivered;
  the suite waits up to 5 s. The one exception is `/__test/discard`, whose `now` moves the clock with no tick: the object is
  being thrown away, and whatever releases the claim afterwards is the restore or the alarm.
- **The real timer or alarm path is exercised end to end only in real-clock mode** (`JM_E2E_REAL_CLOCK=1`).
  Fake-clock mode proves the rules; real-clock mode proves the alarm. The fake clock still proves the object's own alarm: it stores
  its deadline (read back by `/__test/stored-alarm`), and `TestEvictAlarmWake` has the runtime fire that stored alarm on a discarded
  object, with the clock stored for the alarm's tick.
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
that is, on `POST /v1/runs/:run/end`: `harnessVersion`. A task update has no `note`: the schema does not offer one, so it is refused as an unknown field. A post stores
every field its definition allows (every `kind`, `status`, `needs`, `options`, `context`, `taskId`, `links` and `caveat`). A run's post needs its `summary` (the core stores it as the headline).

## Invites

`POST /v1/pods/:pod/invites` returns the invite's `code` once. The server keeps only its SHA-256 (as it does tokens), looks an
accept up by that hash, and builds `url` from its configured public base URL (`PUBLIC_URL` on the Cloudflare adapter, the
request's own origin when unset), never from a fixed host.

## Assumptions the protocol does not state

The suite asserts these; a server that disagrees should change the protocol, not the suite.

- The server sends `hello` right after the WebSocket upgrade; the client then sends `subscribe`. Once the replay is sent
  and the socket is live, the server sends `hello` once more, so presence that changed in between (it has no `seq`) is not
  lost. A client takes the latest `hello` as current.
- The WebSocket token travels in `Authorization: Bearer ...` on the upgrade request, or a browser's `jm_session` cookie does, from the server's own Origin (ADR 0008, "Browser session"). Never in the query string.
- The pod sends each Envelope as one unfragmented text frame (the suite's client does not reassemble fragments).
- Any run of an agent in the pod may claim any task, assigned or not.
- `already_claimed` names the holder in `ref` or `message`: the holder's run id, agent id or display name.
- `task.create` is a socket event (no HTTP route creates tasks); its `task.created` answers it.
- Every sequenced frame is replayed, and sequence numbers have no gaps. Frames with no `seq` (`run.wake`,
  `run.stop`, `hello`) are not part of the log.
- Releasing as the claiming agent's owner means the user whose computer registered the agent.
- `presence.changed` has no `seq`: it is not in the log and not replayed. A client reads current presence from `hello`. A computer is online from its socket until the socket closes or 45 s pass without a heartbeat on the server clock.

## The renewal ledger

No server reports a deadline and events carry no renewal times, so the suite keeps one ledger
(`world.stamp`, `world.expected` in `e2e/world_test.go`). It records every run operation that succeeded and
every heartbeat the suite sent for a live run, stamped with the clock the suite set (fake mode) or the time
it sent the request (real-clock mode). The expected expiry is the last renewal plus 60 000 ms. Every
early-expiry and lateness check reads it, so every server is measured identically.

## The tests

| Test | What it proves |
|---|---|
| `TestF02` | F02: the server's own command line (read from the process table) carries `--persist-to <the run's data dir>/state` and that dir holds state, `/healthz` answers 200 within 10 s, and a failing step names itself and prints the last 20 events. The closing line is judged by `make e2e-c` and `make e2e-a`, not by a test. `TestStepPrinter` checks the printer alone, with a fake failing step. |
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
| `TestPODNAME` | A member's `hello` carries `podName`, the name the pod was created with. `GET /v1/invites/:code` with no token returns exactly `podName`, `inviterName` and `expiresAt` (the expiry the invite was created with); a one-use invite read three times can still be accepted. Unknown, used and expired codes get one and the same 404 body. `GET /v1/session` is 200 with no user signed out and with a dead token, and 200 with the user for a live one, while `GET /v1/me` signed out stays 401. |
| `TestRunTokenOtherPod` | A run token on another pod's routes (read, list, feed, post) gets `403 forbidden`; on its own pod it works. |
| `TestEvictMidClaim` | The runtime discards the pod's object while a run holds a claim and the server process stays up: before the discard the object's own stored alarm is its earliest deadline, which with both computers online is their 45 s staleness bound (F08), and on the new object it equals the claim's deadline; the instance id changes, every socket drops, and with the clock stored one second short of the deadline the new object has released nothing and still holds the old `leaseUntil`. A renewal after the eviction moves `leaseUntil` to that clock plus 60 s and the stored alarm with it (the computers rejoin after that read), a second run's claim is refused with `already_claimed` naming the holder, the old deadline passes without a release, and the new one gives exactly one `claim.released{expired}`. |
| `TestEvictAlarmWake` | The object's own stored alarm equals the claim's deadline once the clock is one millisecond short of it, when both computers have gone stale (F08), and the request counter is above 0. The object is discarded with the clock stored one millisecond short of the deadline, and that stored alarm, which the discard reports and records, is moved to fire a few real ms ahead; the clock becomes the deadline when it runs. Nothing is sent to it. The runtime builds a new instance (its restore releases nothing) and calls its alarm, whose tick finds the deadline due: the alarm handler's record names a new instance that had served no request and the stored alarm time it was moved from, `alarm_fired` moved and then holds still, and every member's replay has exactly one `claim.released{expired}`, stamped at the deadline or later, and still one after the clock moves on. |
| `TestEvictPastDeadline` | The object is discarded with the clock stored five seconds past a claim's deadline; the first thing to reach the new instance is a socket reconnect, which runs no command, and it already carries the `claim.released{expired}`, exactly once. It needs the restore to settle (`boot`). |
| `TestF08` | F08: builds the repo's `daemon/cmd/jellymoon` and runs it as a black box (own process group, output captured, config dir `JELLYMOON_HOME`, server `JELLYMOON_SERVER`, first-registration jmu_ token in `JELLYMOON_USER_TOKEN`, never argv). It registers the computer and stores `computer.json` at 0600 in a 0700 dir; `agent add fake --pod` registers an agent; members get `presence.changed` online within 5 s; a daemon stopped with SIGSTOP (socket open, no heartbeat) goes offline when the fake clock is exactly 45 s past its last sign of life and not 1 ms before; a second start reuses the stored computer; `kill -9` gives offline within 45 s; no token appears in any output or in any process's argv; plain http to a non-local host is refused; and after a server crash with the daemon gone, hello lists the agent offline. |
| `TestF09` | F09: behind a recording proxy, a task assigned to the agent makes the built daemon claim the wake with `POST /v1/runs` and its computer token; the response carries a `run_` id, a 47-character `jmr_` token, a brief and the `/mcp` URL; the fake ACP agent starts with that MCP server (HTTP, `Authorization: Bearer <the run's jmr_>`) in `session/new` and the brief as its first prompt; its `jm_whoami` call over MCP answers with the agent, pod and run; no `jmc_` or `jmu_` token and no token-shaped string is in its argv or environment; a finished agent ends the run `completed`; a daemon restarted with a user token in its environment and none of the allowed names still hands the agent only allow-listed names; a daemon stopped while an agent that ignores SIGTERM runs takes its process group with it and ends that run (matched by its own id) `failed`; and no token reaches any daemon output. |
| `TestF15` | F15: `jellymoon login` (the built daemon) prints the user code and the verification URL; a device code is `authorization_pending` before approval and `slow_down` when polled again at once; a cookie-jar browser walks start, the stub GitHub and the callback, lands on `next` with a session cookie, and `GET /v1/me` returns the Actor; approving the CLI's code makes it write a 0600 `user-token` holding a new `jmu_` for that user and print `Signed in as <login>` and never the token; exactly one exchange, then `invalid_grant`; approving twice `gone`; an unknown code `not_found`; approve by cookie without a JSON content type, with a foreign Origin or with none `forbidden`; the callback refuses a missing state, another browser's state and a reused one; an absolute, `//` or `/\` `next` lands on `/`; a stub `/user` 401 is `unauthorized`; one GitHub id is one user across sign-ins; no GitHub token is in the data dir or the server log; and, last because it moves the clock, an expired code is `expired_token` on approve and on token. |
| `TestBSESS` | ADR 0008, "Browser session": two browsers sign in through the stub; with only the session cookie one creates a pod and an invite, the other accepts it, opens the pod WebSocket from the server's own Origin and gets `hello`, and a post by cookie fans out to it and reads back from the feed; a cookie POST without a JSON content type, from a foreign Origin, `null` or with no Origin is `forbidden` and adds nothing; the socket from a foreign Origin or with none is refused (403); a `jmr_` in the cookie is `unauthorized` while the same token as a Bearer works; a Bearer POST needs no Origin, and a Bearer beside a cookie wins; logout by cookie revokes the token (as cookie and as Bearer, both `unauthorized`) and clears the cookie, and a logout by Bearer revokes that token. |
| `TestAUTHH` | ADR 0008, "Hardening": six callbacks at once for one new GitHub account all sign in, as one user; approve takes the user code in lower case and without its hyphen; a logout with a Bearer `jmu_` and the cookie revokes both and clears the cookie; logout closes the pod socket opened with the revoked token within 5 s and leaves the person's socket of another token open; `POST /v1/auth/device` and `GET /v1/auth/github/start` each answer `rate_limited` with `Retry-After: 60` after at least 5 requests in a fresh minute, and answer again the next minute; a `jmu_` works one second before its 30 days and is `unauthorized` at them on `GET /v1/me`, a pod route and the socket, by Bearer and by cookie; and a device code an hour past its expiry is swept (`invalid_grant`) while a live one stays `authorization_pending`. It moves the clock forward only, so its steps run in order. |
| `TestDENY` | ADR 0008 step 4, `POST /v1/auth/device/deny`: a signed-in person refuses a pending code; every poll of it is then `access_denied`, and a second deny, in lower case without the hyphen, is 200 `{}` again. `jellymoon login` (the built daemon) whose code is denied prints `jellymoon: the sign-in was refused in the browser`, exits non-zero and writes no `user-token`. Deny after approve and approve after deny both get approve's used-code body; deny of an unknown, an exchanged and an expired code each get approve's body for that code, byte for byte. With the start limit spent from the address, 40 denies and an approve still answer 200 (approve's rate limit, which is none). No session, a bad Bearer, a cookie write without JSON, from a foreign Origin, `null` or none is refused and the code stays `authorization_pending`. It moves the clock forward only, so its steps run in order. |
| `TestF14Budget` | The whole suite run (fake clock) passes in under 60 s. The suite tests run in parallel, so this one is parallel too and waits for every one of them before it judges the run. It fails if a suite test failed, if one never reported in a whole-suite run, or if the run took 60 s or more; it skips only when `-run` narrowed the run. `TestBudgetVerdict` checks that judgment with made-up outcomes and needs no server. |
| `TestF14` | F14: the whole slice as one walk, on F09's world (built daemon, fake ACP agent, recording proxy) with nobody signed in. Two people sign in (through the test session route, until F15), one creates the pod and invites, the other joins, and ben's hello lists both with their roles. The daemon registers and the fake agent is added; both see it online. A task assigned to the agent makes the daemon claim the wake (`POST /v1/runs` with its computer token, a brief naming the task and `task_assigned`); both see `task.claimed` with the run, the agent and a lease 60 s past the fake clock; the agent's ask reaches both as `blocked` with an open decision and a receipt (run, agent, computer, harness `fake`, `task_assigned`, the task, and `toolCalls` equal to the two MCP calls the agent made before it). A member rules, both see the decision ruled with the option and who chose it, the daemon claims a second wake whose brief names `decision_ruled` and the decision, the woken run's result carries a `decision_ruled` receipt for the new run, both see `task.updated` done, and both runs end `completed`. The walk, `go build` of the daemon included, takes under 60 s. The fake agent runs a script file read at each prompt, so the test swaps the script between the two runs; `{{taskId}}` in a script argument is the first `tsk_` id in the agent's brief. The ruling's wake names the decision and the ask, not the task, so the second script carries the id as a literal. |

`TestKillMidLease` claims Y (no renewal after the claim; its deadline falls inside the outage) and X
(renewed by a heartbeat; its deadline falls after the restart), then crashes. For each it asserts: not
released before its recorded deadline; released no later than `max(restart, deadline) + 6 s`
once the clock passes it; one `claim.released` over live delivery and over a replay from seq 0.

## Where the suite's time goes (measured before the parallel run)

Every test but `TestStepPrinter`, `TestPortRetry` and `TestF14Budget` boots its own server, and they ran one after another, so
the suite's wall time was the sum of the tests. Per-test seconds, serial: one pod-state run and one Worker run on a dev machine
at load below 4, and the latest CI run of each on `main` after #8. PR 13 failed only `TestF14Budget`, at 61.1 s and then 64.9 s
on the Worker.

| Test | Pod-state, local | Worker, local | Worker, CI | What it waits on |
|---|---|---|---|---|
| `TestRunTokenOtherPod` | 0.4 | 5.8 (first boot) | 2.3 | one server start; the first Worker start of a run is cold |
| `TestEvictMidClaim` / `TestEvictAlarmWake` / `TestEvictPastDeadline` | skipped | 2.0 / 2.2 / 0.8 | 2.6 / 3.1 / 1.9 | a start each, 300 to 500 ms of real sleep |
| `TestF02` | 0.4 | 0.6 | 1.6 | a start |
| `TestF03` | 0.5 | 0.5 | 1.4 | a start |
| `TestF04` | 1.8 | 2.4 | 4.2 | a start, two 400 ms quiet windows, a kill and restart |
| `TestF05` | 0.4 | 0.7 | 1.6 | a start |
| `TestF06` | 0.4 | 1.2 | 1.6 | a start |
| `TestF07` | 2.3 | 3.0 | 3.4 | a start, a 200 ms sleep |
| `TestF08` | 2.1 | 3.1 | 4.6 | `go build` of the daemon, a start, daemon registration and presence, a 300 ms sleep |
| `TestF09` | 4.0 | 4.6 | 4.7 | the same, then an agent process and a stop that waits for its process group |
| `TestF10` / `TestOneReleasePerCause` | 1.7 / 1.6 | 1.9 / 2.2 | 3.1 / 2.9 | a start, a 300 ms sleep in `TestF10` |
| `TestF11` | 2.7 | 1.8 | 2.8 | a start, a daemon |
| `TestFakeAgentHoldStaysUp` | 1.0 | 1.1 | 0.9 | `go build` of the fake agent, a 500 ms hold |
| `TestInputValidation` | 0.5 | 0.6 | 1.8 | a start |
| `TestExpiryZeroTraffic` / `TestRenewRace` | 1.0 / 0.5 | 1.3 / 0.8 | 2.2 / 2.0 | a start, a 300 ms sleep in `TestExpiryZeroTraffic` |
| `TestKillMidLease` | 2.2 | 2.9 | 4.4 | a start, a kill and restart, a 300 and a 500 ms sleep |
| `TestRestartPastDeadline` / `TestReplayAcrossRestart` | 0.6 / 1.4 | 1.2 / 1.8 | 2.9 / 3.5 | a start, a kill and restart, two 200 ms sleeps in the replay test |
| **Suite (`E2E_MS`)** | **26.0** (CI 19.1) | **43.2** | **59.7** | |

The cost is serial server starts, not sleeps: the real sleeps in the whole suite add up to about 4 s, a Worker start takes 1.4 to
2 s on a CI runner, and no two tests overlapped. The fake clock already replaces the long waits. The 60 s budget did not change;
the Worker suite on the CI runner sat 0.3 s under it before PR 13 added one test and tipped it over.

After the parallel run (same machine, same STEPS counts: 58/58 on the pod-state server, 61/61 on the Worker): pod-state 26.0 s to
about 11 s (10.4 to 13.8 s over four runs, some under load 9 to 16), Worker 43.2 s to 10.0 s (10.0 to 15.2 s over four runs). With
`GOMAXPROCS=4`, the slot count of a four-core CI runner, the Worker suite took 14.4 s at load 9. The cut is the overlap of server
starts; no sleep, wait or assertion changed.

## Running in parallel

The suite tests call `par(t)` right after `need`/`needFake`, so they run at the same time, up to `go test`'s `-parallel`. Unless
`-parallel` is given, `TestMain` sets it to the CPU count plus one: the extra slot is the one `TestF14Budget` holds while it waits
(below), so the tests that work still get one slot per CPU. That is safe because each of them builds its own world: its own server process group, ports, data dir, pod,
users and sockets. Two tests that shared a pod, a data dir or a port would see each other's events, so a new test must not.
`TestStepPrinter` and `TestPortRetry` (which starts a child test process) stay serial. The only state the tests share is the harness's
bookkeeping (`outcomes`, the step counters), which is a `sync.Map` and atomics.

`TestF14Budget` is parallel as well and waits on `suiteWG`, which every `par` test joins before it pauses and leaves in its last
cleanup, so it judges after the outcomes and the server kills. It needs `-parallel` 2 or more and says so if given 1. A server
start is CPU-bound (a Worker start is `wrangler dev` plus `workerd`), so a runner with few cores overlaps fewer of them; with
four slots (`GOMAXPROCS=4`) the Worker suite took 14 s on a dev machine against 10 s with ten, far under the budget.

The suite grew, and by November the Worker leg on the CI runner was at or over the budget (61.8 s, then 64.8 s). Three things made
the run longer than its work. `TestF14Budget` held one of the runner's four slots from the moment it resumed, so most of the run
had three. `TestF13` did not call `par` and ran alone before any parallel test started (6.7 s on CI). And `TestIMP` ran its nine
importer steps one after another, 25 s of mostly sleeping (an importer run waits at least 250 ms between two reads), and it often
resumed late, so it was the run's tail. Now the budget test's slot is an extra one, `TestF13` and `TestF08Redaction` are parallel,
and `TestIMP`'s steps run at the same time: each has its own fixture, config dir, work dir and recorder in front of the one shared
server, and they are plain subtests started from goroutines, so together they hold `TestIMP`'s one slot. No wait, deadline or
assertion changed. With `GOMAXPROCS=4` on a dev machine the Worker leg went from 30.6 s to 17.5 to 19.6 s (three runs), and
`TestIMP` from 23.2 s to 5.2 s.

## Skipped on the server

`JM_E2E_SKIP` is a comma-separated list of test names. `make e2e-c` sets it to the three tests below; each one skips, says so,
and still counts as passed for `TestF14Budget`. The make targets set the variable themselves, so an inherited `JM_E2E_SKIP` never applies: `make e2e-a` clears it (a deliberate skip there is `make e2e-a E2E_A_SKIP=<names>`) and `make e2e-c` takes its list from `E2E_C_SKIP`. A skip needs a reason here. Nothing else is skipped, and no assertion differs
between servers.

| Test | Reason |
|---|---|
| `TestEvictMidClaim` | Needs `/__test/discard`, `/__test/instance` and `/__test/stored-alarm`: the runtime throwing away a Durable Object and the alarm that object stored. The server is one process with one timer and no object to discard. Its restore path is covered by `TestF04` (evict), `TestKillMidLease` and `TestRestartPastDeadline` (kill -9 and restart). |
| `TestEvictAlarmWake` | Needs `/__test/discard` with `alarmMs` and `/__test/woke`: the runtime firing an object's own stored alarm on a new instance. The server's single timer is covered by `TestExpiryZeroTraffic` (fake clock) and, in real-clock mode, by `TestKillMidLease`. |
| `TestEvictPastDeadline` | Needs `/__test/discard` with `now`: an object thrown away with a deadline already past, then woken by a reconnect. `TestRestartPastDeadline` proves the same rule (the restore settles an expired claim before the first frame) for a restart. |

## Skipped on the Worker

`make e2e-a` sets `JM_E2E_SKIP` to the tests of features that are built on the pod-state server only. ADR 0006 made the server
the adapter Jellymoon runs and supersedes the Durable Object for new features, so the Worker is not extended for them.

| Test | Reason |
|---|---|
| `TestF19` | The connector layer (vault, catalogue pin, write gate, receipts) lives in the pod-state server (ADR 0006, ADR 0009). The Worker has no connector routes or operations. |
| `TestF21` | Evidence (the tool-call records and the `evidence` read) is served by the pod-state server only (ADR 0006, "Evidence"). The Worker still counts MCP calls for the receipt but keeps no records, and answers `not_found` for the read. |
| `TestF22` | `jellymoon connector connect\|grant\|approvers\|list\|receipts` call the connector routes of the pod-state server (ADR 0009), which the Worker does not serve. |
| `TestF25` | Grounding cites tool-call records, which the Worker does not keep. It runs the same core check with the messages its log holds and no tool calls, so an ask citing a tool call is `ungrounded` there and one citing only messages is checked as on the server (`worker/test/tool-desc.test.ts`). The asks of `TestF11`, `TestF12` and `TestF14` cite a person's message, so they run on both. |

`TestF19` starts its server with `JM_VAULT_KEY` (the harness's per-server environment, `startServer(t, env...)`) and a stub
Linear inside the test: an MCP-over-HTTP JSON-RPC server on 127.0.0.1 that requires the stub bearer and records every
`tools/call`. The agent's environment it scans is the run claim answer (`POST /v1/runs`), which is everything the daemon builds
the agent's environment from.

`TestF22` builds the CLI (`jellymoon connector ...`) and runs it as the pod owner and as a member, against the same kind of world
and stub Linear, behind a recording proxy. The CLI has no controlling terminal (`Setsid`), so the hidden token prompt cannot open
and a missing credential must fail. The step that scans for the stub's credential covers every command line, stdout and stderr,
the config dir, the server log and every recorded request and answer; the credential may appear only in the bodies of the three
connects that reached the server.

## The server's partly skipped step

One step is skipped, not a whole test. On a real Postgres (`DATABASE_URL` set, which is how CI runs `make e2e-c`) the state lives
in the database, so the `--persist-to` directory holds no files. `TestF02` still checks that the server was started with
`--persist-to <dir>/state` and prints `STEP PARTLY SKIPPED` for the one check that the directory is not empty (`make e2e-c` sets
`JM_E2E_STATE_IN_DB=1`). That the state persisted is proved against the same database by `TestReplayAcrossRestart` and
`TestRestartPastDeadline`, which kill -9 the server and read back events and a lease. The server writes no marker file: a file
made only to satisfy the check would prove nothing. Against PGlite (no `DATABASE_URL`) the directory holds the database files
and the check runs in full.

## Runs on a real Postgres only

`TestBootLock` starts two servers on one database and needs that database to be shared, so it runs only when state lives in a
real Postgres (`JM_E2E_STATE_IN_DB=1`, which is how CI runs `make e2e-c`); anywhere else it skips and says so. The first server
holds the boot lock (ADR 0010, "A rollout's overlap"), a session-level advisory lock keyed to its schema. The second, given the
same data dir and so the same schema, must exit before `/healthz` answers, name the lock, and accept no write. After the first is
killed with `kill -9`, the same start succeeds and serves the first server's pod. On PGlite the data dir is one process's own
database files and on the Worker each process keeps its own state, so there is no second process to meet. Without a local
Postgres the order of the boot (the lock before the first statement, nothing restored when it is refused) is pinned by
`server/test/boot-lock.test.ts`.

The lock behaviour: a refused start retries for 2 s (a just-killed owner's session can take a moment to close), then exits 1.
The holder re-checks its lock every 5 s and exits if it is gone, and its session carries `idle_session_timeout` (30 s), so an
owner whose host vanished frees the lock within about that long instead of waiting on TCP keepalive.

## Output lines

- `E2E_MS=<n> STEPS=<k>/<n>`: the closing line, printed once after every test has ended. `n` is the suite wall clock after `go test`
  finishes its tests. The steps are every named step of the tests that name them (`TestF02`) plus one step for each other test that
  ran; a skipped test counts nothing. `k` is the steps that passed, so a red run prints `k < n`. A failing named step prints its
  name and the last 20 events the suite saw before it. A test cannot see the run's last line, so `make e2e-c` and `make e2e-a` judge it: the
  last line before `go test`'s own `ok`/`FAIL`/`PASS` lines (blank lines ignored) must match `^E2E_MS=[0-9]+ STEPS=[0-9]+/[0-9]+$`, with `k = n` on a green run and `k < n` on a red one.
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
