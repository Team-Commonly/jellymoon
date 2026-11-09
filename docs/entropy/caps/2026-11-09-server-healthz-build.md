# Cap note 2026-11-09: server-healthz-build

2026-11-09: server cap 4,550 -> 4,560; `/healthz` names the commit the process runs in an `X-Jm-Build` header when `JM_BUILD` is a commit id (7 to 40 lowercase hex), so the hosted deploy (`hosted/ship.sh`) accepts health only from its own new container during a rollout, with one test that a valid id is reported and junk is not. The tree printed `LOC server 4560 / 4550`; the cap is the count rounded up to ten. No other cap changes.
