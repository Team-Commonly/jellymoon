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
