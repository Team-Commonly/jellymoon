# Cap note 2026-11-09: multi-server-e2e-connector-hardening-2

2026-11-09: server 3,620 -> 3,850, e2e 7,730 -> 7,830; the connector hardening (ADR 0009: a write left released by a crash settles as `unknown`, a read that throws still writes a `failed` receipt, a per-connection approver set, upstream calls kept out of the pod's queue, 2001:db8::/32 refused) and its tests, on the tree merged with main (#45, the sign-in hardening): it measures 3,842 / 7,830, and each cap is the count rounded up to ten. This replaces the branch's earlier note, written against main before #45. No other cap changes.
