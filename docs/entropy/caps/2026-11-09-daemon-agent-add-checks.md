# Cap note 2026-11-09: daemon-agent-add-checks

2026-11-09: daemon cap 7,810 -> 7,960; `agent add` now refuses a display name no @mention can address and a harness with no program configured, before registering, and each refusal has tests (the name rule, the CLI refusal, and the no-row check). Merged with main at 66cf55b (which has the import token checks), the tree printed `LOC budget: daemon 7958 > 7810`; the cap is the count rounded up to ten. No other cap changes.
