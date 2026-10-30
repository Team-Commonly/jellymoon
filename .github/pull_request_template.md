## Feature
<!-- One feature per PR: Fxx from feature_list.json, or "infra" with a reason. -->

## Done-test
<!-- The exact command and its result. Red before this PR, green after. -->
```
$ <test command>
E2E_MS=<n> STEPS=<k>/<k>
```

## What changed and why
<!-- Three lines at most. The diff tells the rest. -->

## Review
- [ ] `make preflight` green locally
- [ ] `feature_list.json`: only `passes` changed
- [ ] `progress.md` entry added
- [ ] Every land needs the review's pod verdict at the head, whatever the paths (one review per head). See `docs/development/landing.md`
- [ ] Lesson captured in the repo (test, lint or docs/), or none surfaced
