# Cap note 2026-11-09: server-web-error-path-tests

2026-11-09: server cap 4,120 -> 4,130 and web cap 1,890 -> 1,900; the tree measures server 4,126 and web 1,893. Three review follow-ups: a test that a throwing route still carries `Referrer-Policy: no-referrer` on the onError reply (server), removal of a dead `status !== 101` guard (server; Bun's upgrade reply is a 200), and a test that the boot check reads fetch's own AbortError as signed out (web). Each cap is the count rounded up to ten; no other cap changes. ERRPATHS.
