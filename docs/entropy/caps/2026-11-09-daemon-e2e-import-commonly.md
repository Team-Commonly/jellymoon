# Cap note 2026-11-09: daemon-e2e-import-commonly

2026-11-09: daemon cap 4,940 -> 7,160 (the tree measures 7,156) and e2e cap 7,900 -> 8,460 (it measures 8,460). The unit adds `jellymoon import commonly`: about 2,210 lines in the daemon (the `importer` package, 749 of them unit tests, plus the two API calls and the CLI command) and 558 lines of e2e (a fixture Commonly server, the harness, and nine steps, one per rule that must hold: skip, refuse, fail, dry run, idempotence, kill and resume, backoff, opt-ins, no leaks). The caps are the counts rounded up to ten; no other cap changes. IMP.
