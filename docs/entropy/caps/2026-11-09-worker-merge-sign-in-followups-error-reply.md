# Cap note 2026-11-09: worker-merge-sign-in-followups-error-reply

2026-11-09: worker cap 2,500 -> 2,520; this merges #66's and #67's worker growth. Each raised the worker cap from 2,480 to 2,500 against its own tree (#66 counted 2,498, #67 counted 2,492), but both added lines to `worker/`, so their sum on one tree is more than either cap allows. The merged tree printed `LOC budget: worker 2515 > 2500`; the cap is the count rounded up to ten. No other cap changes.
