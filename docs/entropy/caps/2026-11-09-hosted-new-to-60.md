# Cap note 2026-11-09: hosted-new-to-60

2026-11-09: hosted cap (new) 60, the hosted instance's front Worker (hosted/src/index.ts, ADR 0010: forwards every request to one
container and keeps it up) is 54 lines at the first green run; the cap is that count plus about 15%; the total is 36,150. A front
that needs more than this is deciding something, which ADR 0010 says it must not. No other cap changes.
