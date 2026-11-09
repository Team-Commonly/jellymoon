# Cap note 2026-11-10: core-land-deterministic-commit

2026-11-10: core cap 4,240 -> 4,280; `make land` now builds its land commit deterministically (`landCommitEnv` in `scripts/land-decide.mjs`: fixed identity, date taken from the parents) and reads the check runs GitHub already holds for that SHA before pushing, with tests in `scripts/check-land.mjs` and `scripts/check-land-e2e.mjs`. The tree printed `LOC budget: core 4276 > 4240`; the cap is the count rounded up to ten. No other cap changes.
