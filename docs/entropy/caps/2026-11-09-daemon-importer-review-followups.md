# Cap note 2026-11-09: daemon-importer-review-followups

2026-11-09: daemon cap 6,910 -> 6,940; the importer review follow-ups add two tests: a 401 on a later page of a pod's messages skips the whole pod and counts it, and a secret in a pod name is reported without echoing a secret-shaped pod id. The tree printed `LOC budget: daemon 6939 > 6910`; the cap is the count rounded up to ten. No other cap changes.
