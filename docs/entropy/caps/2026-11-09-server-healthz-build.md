# Cap note 2026-11-09: server-healthz-build

2026-11-09: server cap 4,640 -> 4,660; `/healthz` names the commit the process runs in an `X-Jm-Build` header when `JM_BUILD` is a commit id (7 to 40 lowercase hex), so the hosted deploy (`hosted/ship.sh`) accepts health only from its own new container during a rollout, with one test, in its own describe block (the function-length cap), that a valid id is reported and junk is not. Merged on main after #102 (which set 4,640), the tree printed `LOC budget: server 4658 > 4640`; the cap is the count rounded up to ten. No other cap changes.
