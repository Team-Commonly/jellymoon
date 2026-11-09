# Cap note 2026-11-09: daemon-import-token-errors

2026-11-09: daemon cap 7,670 -> 7,750; `jellymoon import commonly` now refuses a token with whitespace or control characters before any request, and turns a 401, 403 or 404 on the identity check (and any other status) into a message that names the cause and the host, with five tests that none of them holds the token. The tree printed `LOC budget: daemon 7744 > 7670`; the cap is the count rounded up to ten. No other cap changes.
