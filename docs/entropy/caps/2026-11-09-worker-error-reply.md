# Cap note 2026-11-09: worker-error-reply

2026-11-09: worker cap 2,480 -> 2,500; `handle` now catches a route that throws, logs it and answers with the `internal` error body through the same Referrer-Policy path, as the server's onError does, with a test in `worker/test/referrer.test.ts` and one updated expectation in `worker/test/signin.test.ts`. The tree printed `LOC budget: worker 2492 > 2480`; the cap is the count rounded up to ten. No other cap changes.
