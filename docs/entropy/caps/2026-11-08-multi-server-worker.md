# Cap note 2026-11-08: multi-server-worker

2026-11-08: server cap 1,990 → 2,030 and worker cap 1,930 → 1,960, the browser session (ADR 0008, "Browser session") in both adapters: one authentication function each (`authOf`: Bearer first, else the session cookie, with the JSON and own-Origin rules for a state change and the socket) and `POST /v1/auth/logout`. The gate printed `LOC server 2008 / 1990` and `LOC worker 1945 / 1930` before this raise; each cap is the count plus a little for the fix pass, not 15%. The total is 18,680. No other cap changes. BSESS.
