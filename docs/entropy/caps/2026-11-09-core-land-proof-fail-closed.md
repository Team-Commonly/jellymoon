# Cap note 2026-11-09: core-land-proof-fail-closed

2026-11-09: core cap 4,290 -> 4,300; `landSuiteRuns` (`scripts/land-decide.mjs`) now fails closed: a suite list that is not an array of ids, or a run with no numeric suite id, leaves no runs, so it can no longer pass runs through as proof. New cases in `scripts/check-land.mjs` and `scripts/check-land-e2e.mjs` pin it and the failing or paged `actions/runs` read. The tree printed `LOC budget: core 4294 > 4290`; the cap is the count rounded up to ten. No other cap changes.
