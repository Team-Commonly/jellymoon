# Cap note 2026-11-09: daemon-e2e-reader

2026-11-09: daemon cap 8,180 -> 8,500 and e2e cap 10,300 -> 10,490; F26, the reader of ADR 0011 section 1 (`jellymoon day`). The daemon grows by the new `reader` package, the command in `cli/day.go`, and `redact.Credentials` with its test; e2e grows by `f26_test.go`, which writes its own synthetic session logs. The tree printed `LOC budget: daemon 8497 > 8180` and `LOC budget: e2e 10482 > 10300`; each cap is the count rounded up to ten. No other cap changes.
