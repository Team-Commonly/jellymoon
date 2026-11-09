# Cap note 2026-11-09: e2e-step-panic

2026-11-09: e2e cap 9,920 -> 9,960; the e2e stepper turns a panic inside a step (a nil client or socket left by a failed earlier step, as TestBSESS hit) into that step's failure, so later steps still run and every failure is reported, with a test in `e2e/stepper_test.go`. Merged with main at e24d145 (the protocol additions), the tree printed `LOC budget: e2e 9958 > 9920`; the cap is the count rounded up to ten. No other cap changes.
