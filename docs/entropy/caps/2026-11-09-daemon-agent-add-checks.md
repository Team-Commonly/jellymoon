# Cap note 2026-11-09: daemon-agent-add-checks

2026-11-09: daemon cap 7,670 -> 7,820; `agent add` now refuses a display name no @mention can address and a harness with no program configured, before registering, and each refusal has tests (the name rule, the CLI refusal, and the no-row check). The tree printed `LOC budget: daemon 7817 > 7670`; the cap is the count rounded up to ten. No other cap changes.
