# Cap note 2026-11-09: multi-worker-server-signin-followups

2026-11-09: worker 2,160 -> 2,330, server 3,620 -> 3,700; the sign-in hardening follow-ups (ADR 0008: TRUST_PROXY behind TLS, user tokens with no expiry, the D1 migrations, the token_pods order, hub.sockets pruned, logout with allSettled). Most of the growth is tests: the worker's D1 fake over SQLite and its three test files, and the server's signin-hardening test. The tree measures 2,330 / 3,691, and each cap is the count rounded up to ten. No other cap changes.
