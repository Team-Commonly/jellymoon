# Cap note 2026-11-09: multi-worker-server-signin-followups

2026-11-09: worker 2,160 -> 2,390 and server 3,910 -> 4,070. e2e needs no raise: main's 7,880 (#52) holds it. These cover the sign-in hardening follow-ups in ADR 0008:
- TRUST_PROXY behind TLS;
- user tokens with no expiry;
- the D1 migrations;
- the token_pods order;
- hub.sockets pruning;
- logout with allSettled.

They also cover the fix rounds from their review:
- the PUBLIC_URL scheme check;
- a logout racing a socket's open, on both adapters;
- the migration shape test's indexes and foreign keys;
- a token check that fails when a server socket opens, which now closes that socket;
- TestAUTHH's start-limit step sending caller-set forwarding headers.

Most of the growth is tests: the worker's D1 fake over SQLite and its three test files, and the server's signin-hardening test with its real sockets. On the tree merged with main (#48, #50, #51 and #52) it measures 2,390 / 4,069 / 7,877, and each raised cap is the count rounded up to ten. No other cap changes.
