# Cap note 2026-11-09: multi-core-podcore-daemon-trim

2026-11-09: three caps lowered to their measured size, each rounded up to the next ten. core 4,200 → 4,080 (measured 4,072); podcore 3,000 → 2,690 (measured 2,681); daemon 5,000 → 4,830 (measured 4,825). The caps grew on the 11-08 and 11-09 merges, and these areas now carry slack that no feature needs. The cap is the measured count rounded up to ten, and no cap was raised. Worker, server, web and e2e are within ten lines of their caps and are unchanged. The total goes from 27,620 to 27,020. No code, refactor or test changed.
