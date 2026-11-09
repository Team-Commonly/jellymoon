# Cap note 2026-11-09: device-deny

2026-11-09: worker cap 2,680 -> 2,780, server 4,550 -> 4,640, daemon 8,170 -> 8,180, web 2,940 -> 3,020 and e2e 9,960 ->
10,220. Refusing a sign-in code (`POST /v1/auth/device/deny`, ADR 0008 step 4) adds the route to both adapters, a unit test
file for each adapter (the five rules: access_denied on the poll, the approve/deny order, refusals equal to approve's byte for
byte, approve's rate limit, the auth guards), the generated `access_denied` code and the CLI's stop on it, the page's "This
wasn't me" with its unit tests, and TestDENY (eight steps, both adapters, the built CLI included). The tree printed
`LOC budget: worker 2775 > 2680`, `server 4640 > 4550`, `daemon 8173 > 8170`, `web 3011 > 2940` and `e2e 10217 > 9960`; each
cap is the count rounded up to ten. No other cap changes.
