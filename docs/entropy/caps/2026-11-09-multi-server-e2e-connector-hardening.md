# Cap note 2026-11-09: multi-server-e2e-connector-hardening

2026-11-09: server 3,480 -> 3,710, e2e 7,420 -> 7,530; the connector hardening (ADR 0009: a write left released by a crash settles as `unknown`, a read that throws still writes a `failed` receipt, a per-connection approver set, upstream calls kept out of the pod's queue, 2001:db8::/32 refused) and its tests, on the tree merged with main (#46): it measures 3,701 / 7,524, and each cap is the count rounded up to ten. No other cap changes.
