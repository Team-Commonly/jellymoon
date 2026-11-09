# Cap note 2026-11-09: worker-server-after-referrer-and-sign-in

2026-11-09: worker cap 2,420 -> 2,470 and server cap 4,080 -> 4,120. Two changes each raised both caps and landed in parallel: main's referrer-policy change (worker 2,390 -> 2,420, server 4,070 -> 4,080) and this branch's sign-in follow-ups (worker 2,390 -> 2,440, server 4,070 -> 4,110). Merged, the two raises add up to more than either cap alone. The merged tree measures worker 2,463 and server 4,112; each cap is the count rounded up to ten. No other cap changes.
