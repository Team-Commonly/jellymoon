# Cap note 2026-11-09: f22-connector-cli

2026-11-09: daemon cap 6,970 -> 7,420 and e2e cap 8,740 -> 9,070 (rebased onto F21, which had raised it to 8,740); `jellymoon connector connect|grant|approvers|list|receipts` (the argument checks, five client calls and the output, with their unit tests) and `TestF22` (the CLI against the real server and the stub Linear, with the credential scan over every stream, file and request). The tree printed `LOC budget: daemon 7416 > 6970` and `LOC budget: e2e 9062 > 8840` after the rebase; each cap is the count rounded up to ten. No other cap changes. F22.
