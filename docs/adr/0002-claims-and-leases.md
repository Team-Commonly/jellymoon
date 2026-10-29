# 0002 · Claims and leases

Status: Accepted (owner ruling, 2026-10-29). The unknowns below stay open;
accepting this ADR does not settle them.

The schema, `docs/design/protocol.md` and feature F10 described lease renewal three different ways. This
note is the one rule they now follow. The schema carries it as `x-rules` in `schema/task.json`.

## Decision

- A claim belongs to a run. Its lease renews on (a) any operation that run's token makes, through any
  adapter, and (b) every daemon heartbeat (every 15 s) from the computer that hosts the run, while the
  run has not ended. A long local tool call (a 20-minute test suite) keeps its claim; silence on the pod is
  not abandonment.
- `leaseUntil` is the last renewal plus 60 s (four missed heartbeats).
- On expiry the pod drops the claim and emits `task.updated` with no claim. The task's status and
  assignee are unchanged and the run is not stopped. If that run later operates on the task and nobody
  else has claimed it, its claim is restored; if someone has, it gets `already_claimed` naming the holder.
- When a run ends (`POST /v1/runs/:run/end`) or its token expires (at most 2 h), its claims are released
  at once.

## Open unknowns

- 60 s is a guess. Measure it in the slice: expiries followed by a restore within 5 minutes mean it is
  too short (laptop sleep, network flaps).
- The 2 h run cap ends long tasks mid-work; the next wake re-claims. It is unmeasured whether that churn
  matters.
- Heartbeat renewal cannot tell a hung harness from a busy one; a hung run holds its claim until the 2 h
  cap.
- This is not v3's ADR-018 90 s lease, which was also a guess. Nothing is inherited from it.

## Recourse

A hung harness no longer has to wait for the 2 h cap. Two operations let a person step in (schema:
`task_release`, `run_stop`; rules in `schema/operations.json`).

- **`task_release`** (`POST /v1/pods/{podId}/tasks/{taskId}/release`) drops a task's claim. Callers: the run
  that holds it, the owner of the claiming agent (the user whose computer registered it), or a pod owner.
  Anyone else gets `forbidden`. Members receive `task.updated` with no claim. Releasing an unclaimed task is
  a no-op that returns the task.
- **`run_stop`** (`POST /v1/pods/{podId}/runs/{runId}/stop`) is user-only: the agent's owner or a pod owner.
  At once, the run token is revoked (its next call gets `unauthorized`), its claims are released, the run
  ends as `cancelled`, and the pod sends the `run.stop` event to the hosting computer, whose daemon cancels
  the harness session. Revocation does not wait for the daemon, so a stop works with the computer offline.
  Stopping an ended run returns its outcome unchanged.
- **No restore.** A claim a person released is not restorable by that run; if it later operates on the task
  it gets `forbidden`. The restore rule under Decision applies only to lease expiry.
- **Record.** Both show up as ordinary `task.updated` events and the run-ended fact; there is no separate
  audit store. The optional `reason` (at most 160 characters) rides on the operation and `run.stop`.

This closes the "no recourse" half of the hung-harness unknown. The hang itself is still undetectable by
the pod; a person has to notice it.
