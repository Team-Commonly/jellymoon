# Cap note 2026-11-08: no-change-f18-pr-5a

2026-11-08: no cap changes. The browser specs and their harness live in `web-e2e/`, which no LOC budget counts: web measures 1,535 of 1,580 and, on the tree merged with main (F15), e2e 6,518 of 6,520, so neither has room, and the specs are test code of the browser surface, not of the web package or the contract suite. The file and function limits still apply there. F18 PR 5a.
