# Verifier protocol

A verifier is a fresh-context subagent. It never sees the builder's reasoning. It gets four things:
- the PR number and head SHA;
- the feature (from `feature_list.json`) or the unit's brief;
- the repo at that head;
- this file.

It runs on a different model from the builder.

## What it does, in order
1. **Reproduce.** From a clean checkout of main (never the PR's tree: its Makefile and scripts would run before the
   trust gate decides), run `make verify-preflight PR=<n>` (needs `JM_REPO` and `JM_TRUSTED_LOGINS`; `landing.md`).
   It checks the trust gate first, then runs the same merge-result preflight land runs, only for a same-repo PR whose
   author and every push-capable account are trusted. An untrusted PR is not executed on this host; it refuses before fetching; read the code
   with `git show`, `git diff` or GitHub, never a checkout followed by `make` or `./init.sh`, and
   report `verifier-blocked`. The bootstrap PR's verifiers run its own land code with an explicit `--bootstrap`,
   which is trusted under the same rule. Before running it, check by hand with `gh` that the PR is same-repo
   (`isCrossRepository` false), its author is trusted and every push-capable account is trusted: land.mjs does not exist on
   main yet, so nothing else checks. `--bootstrap` is refused once main has `scripts/land.mjs`, `--preflight-only` included. Then the feature's `test` command, only for a trusted PR. A red result ends
   the review with `verifier-failed`.
2. **Acceptance.** Walk each step of the feature or brief against what the test actually asserts.
   A step that no assertion covers is a finding.
3. **Review.** Apply Pocock `code-review` to the diff against base `main`. Correctness comes first; then
   the contract (schema to generated code, server-stamped fields, token scope).
4. **Deslop.** Apply pstack `deslop` (`.agents/skills/deslop/SKILL.md`, base `main`). Look for dead code,
   needless abstraction, comments that restate code, and over-defensive guards.
5. **Subtract.** Answer two questions: could the diff be smaller, and does it delete anything it made
   obsolete?
6. **Entropy.** Report lines added and removed (`git diff --shortstat origin/main...HEAD`) and the LOC
   lines printed by `make entropy`.

## Verdict
The last lines of the report are always these, in this order:
```
LEDGER <pr> <head-sha> <unit-test-verified|live-ui-verified|type-check-only|verifier-blocked|verifier-failed>
FINDINGS <n blocking> <n non-blocking>
GATE: PASS | GATE: FAIL
```
- `GATE: PASS` requires zero blocking findings.
- The verifier never pushes, comments on GitHub or merges. The coordinator transcribes the report into the ledger
  row (`<pr> <head> PASS <model>-verifier <when> <note>`, `landing.md`): the `GATE:` line becomes the verdict, the
  verifier's model becomes `by`, and the `FINDINGS` line goes in `note`. `make land` reads the row. The coordinator
  writes no verdict the report does not carry.
- Every PR still needs the review's pod verdict at the same head, posted by the operator (one review, no separate reviewer; the accepted ids are operator env `JM_VERDICT_REVIEWERS`)
  (`landing.md`). The secret scans only refuse, never waive.
- The verdict covers secrets in what the scanners skip: added-then-deleted blobs in allowlisted paths (`node_modules`, `bower_components`, `gitleaks.toml`), lockfiles and `go.sum` (added-then-deleted text blobs elsewhere ARE read by the range scan), binaries and images, and commit metadata (headers, author and committer fields, file names, a message after a NUL). Read the range for them (`landing.md`, Known limits).
