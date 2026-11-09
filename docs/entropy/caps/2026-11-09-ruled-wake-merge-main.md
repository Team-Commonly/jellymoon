# Cap note 2026-11-09: ruled-wake-merge-main

2026-11-09: server cap 4,150 -> 4,160 on merging main into `ruled-wake-names-task`. Main (#63) raised the server cap to 4,130 and this branch to 4,150, so `entropy.json` conflicted on that one line; the merged tree prints `LOC server 4155`, and the cap is the count rounded up to ten. The web cap (1,900) and daemon cap (4,940) come from main untouched and hold (web 1,893, daemon 4,934); worker 2,480, podcore 2,780 and e2e 7,900 are this branch's and hold. No other cap changes.
