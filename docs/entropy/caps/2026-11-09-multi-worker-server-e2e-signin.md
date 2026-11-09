# Cap note 2026-11-09: multi-worker-server-e2e-signin

2026-11-09: worker 2,040 -> 2,160, server 3,480 -> 3,620, e2e 7,420 -> 7,730; the sign-in hardening (ADR 0008: token expiry, the start limit, the sweep, GitHub timeouts, race-safe first sign-in, logout revoking both tokens and closing their sockets) in both adapters and TestAUTHH, on the tree merged with main (#41, #44, #46): it measures 2,156 / 3,612 / 7,723, and each cap is the count rounded up to ten. No other cap changes.
